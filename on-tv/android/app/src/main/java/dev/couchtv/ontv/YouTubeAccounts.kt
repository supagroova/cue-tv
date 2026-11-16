package dev.couchtv.ontv

import org.json.JSONArray
import org.json.JSONObject
import java.net.URI

internal interface YouTubeVault {
    fun get(key: String): String?
    fun put(key: String, value: String?)
}
internal data class YouTubeReply(val status: Int, val body: JSONObject)
internal interface YouTubeTransport {
    fun post(endpoint: String, fields: Map<String, String>): YouTubeReply
    fun get(resource: String, query: Map<String, String>, token: String): YouTubeReply
}

/** Google authorises the television; each existing remote pairing owns a separate connection.
 * The caller identity is derived from authentication, never supplied by a phone request body. */
internal class YouTubeAccounts(private val vault: YouTubeVault, private val http: YouTubeTransport,
    private val now: () -> Long = System::currentTimeMillis) {
    companion object { const val SCOPE = "https://www.googleapis.com/auth/youtube.readonly" }
    private val locks = Array(32) { Any() }
    private data class Cached(val at: Long, val json: String)
    private val cache = object : LinkedHashMap<String, Cached>(32, .75f, true) {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, Cached>?) = size > 32
    }
    private fun lock(id: String): Any {
        if (!id.matches(Regex("[a-f0-9]{64}"))) throw ApiError(401, "Pair this phone first.")
        return locks[(id.hashCode() and Int.MAX_VALUE) % locks.size]
    }
    private fun read(key: String) = vault.get(key)?.let { JSONObject(it) }
    private fun save(key: String, value: JSONObject?) = vault.put(key, value?.toString())
    private fun config() = read("config") ?: throw ApiError(503, "YouTube account connections aren’t ready on this TV yet. Search still works.")

    @Synchronized fun configure(input: JSONObject) {
        val id = input.optString("client_id"); val secret = input.optString("client_secret")
        if (!id.matches(Regex("[0-9]+-[A-Za-z0-9_-]+\\.apps\\.googleusercontent\\.com")) ||
            secret.length !in 16..256 || secret.any { it.code !in 33..126 }) throw ApiError(400, "Use the TV-device OAuth client downloaded from Google.")
        val previous = read("config")
        if (previous != null && previous.optString("client_id") != id) throw ApiError(409, "YouTube is already configured with another app registration. Existing connections were preserved.")
        save("config", JSONObject().put("client_id", id).put("client_secret", secret))
    }

    fun status(id: String): JSONObject = synchronized(lock(id)) { statusLocked(id) }
    private fun statusLocked(id: String): JSONObject {
        val cfg = read("config")
        val account = read("account-$id")
        var pending = read("link-$id")
        if (pending?.optString("state") == "pending" && pending.optLong("expires_at") <= now()) {
            pending = JSONObject().put("state", "expired"); save("link-$id", pending)
        }
        val connected = account?.optString("state") == "connected" && account.optString("client_id") == cfg?.optString("client_id")
        val state = when { cfg == null -> "setup_required"; connected -> "connected"; pending != null -> pending.optString("state"); account?.optString("state") == "reconnect" -> "reconnect"; else -> "disconnected" }
        return JSONObject().put("configured", cfg != null).put("connected", connected).put("state", state)
            .put("connection", if (connected) account?.optString("connection_id") else JSONObject.NULL)
            .put("account", if (connected) account?.optJSONObject("profile") ?: JSONObject().put("title", "YouTube account") else JSONObject.NULL)
            .put("link", if (state == "pending") JSONObject().put("user_code", pending!!.getString("user_code"))
                .put("verification_url", pending.getString("verification_url")).put("expires_in", maxOf(0, (pending.getLong("expires_at") - now()) / 1000))
                .put("interval", pending.getInt("interval")) else JSONObject.NULL)
    }

    fun begin(id: String): JSONObject = synchronized(lock(id)) {
        val state = statusLocked(id)
        if (state.optBoolean("connected")) throw ApiError(409, "Disconnect this phone before choosing a different YouTube account.")
        if (state.optString("state") == "pending") return@synchronized state
        val cfg = config()
        val response = http.post("device/code", mapOf("client_id" to cfg.getString("client_id"), "scope" to SCOPE))
        if (response.status != 200) throw ApiError(502, "Google couldn’t start the connection. Check the TV’s YouTube app registration and try again.")
        val body = response.body
        val code = body.optString("user_code"); val device = body.optString("device_code")
        val verification = body.optString("verification_url")
        if (code.length !in 1..32 || code.any { it.code !in 33..126 } || device.length !in 8..4096 || !verificationURL(verification) || body.optInt("expires_in") !in 10..3600)
            throw ApiError(502, "Google returned an invalid sign-in response. Try again.")
        val interval = body.optInt("interval", 5).coerceIn(5, 60)
        save("link-$id", JSONObject().put("state", "pending").put("device_code", device).put("user_code", code)
            .put("verification_url", verification).put("expires_at", now() + body.getInt("expires_in") * 1000L)
            .put("interval", interval).put("next_poll", now() + interval * 1000L).put("client_id", cfg.getString("client_id")))
        statusLocked(id)
    }

    fun poll(id: String): JSONObject = synchronized(lock(id)) {
        val state = statusLocked(id)
        if (state.optString("state") != "pending") return@synchronized state
        val link = read("link-$id")!!
        if (now() < link.getLong("next_poll")) return@synchronized state
        val cfg = config()
        if (cfg.getString("client_id") != link.getString("client_id")) { save("link-$id", null); return@synchronized statusLocked(id) }
        link.put("next_poll", now() + link.getInt("interval") * 1000L); save("link-$id", link)
        val response = http.post("token", mapOf("client_id" to cfg.getString("client_id"), "client_secret" to cfg.getString("client_secret"),
            "device_code" to link.getString("device_code"), "grant_type" to "urn:ietf:params:oauth:grant-type:device_code"))
        when {
            response.status == 200 -> {
                val body = response.body
                if (!body.optString("scope").split(' ').contains(SCOPE)) {
                    save("link-$id", JSONObject().put("state", "denied")); throw ApiError(403, "Allow read-only YouTube access on Google’s page to connect your account.")
                }
                val token = checkedToken(body.optString("access_token")); val refresh = checkedToken(body.optString("refresh_token"))
                val record = JSONObject().put("state", "connected").put("client_id", cfg.getString("client_id"))
                    .put("connection_id", java.util.UUID.randomUUID().toString())
                    .put("access_token", token).put("refresh_token", refresh).put("expires_at", expiry(body))
                save("account-$id", record); save("link-$id", null); clearCache(id)
                // A failed profile lookup does not discard a successfully granted connection.
                runCatching {
                    val profile = authorised(id, "channels", mapOf("part" to "snippet", "mine" to "true", "maxResults" to "1"))
                    val item = profile.optJSONArray("items")?.optJSONObject(0)
                    val latest = read("account-$id")?.takeIf { it.optString("state") == "connected" } ?: return@runCatching
                    latest.put("profile", JSONObject().put("title", YouTubeAccountData.text(item?.optJSONObject("snippet")?.optString("title").orEmpty()).ifEmpty { "YouTube account" })
                        .put("thumbnail", YouTubeAccountData.image(item?.optJSONObject("snippet"))).put("channel_id", item?.optString("id").orEmpty()))
                    save("account-$id", latest)
                }
            }
            response.body.optString("error") == "authorization_pending" -> Unit
            response.body.optString("error") == "slow_down" -> {
                val interval = (link.getInt("interval") + 5).coerceAtMost(60)
                link.put("interval", interval).put("next_poll", now() + interval * 1000L); save("link-$id", link)
            }
            response.body.optString("error") in setOf("access_denied", "expired_token", "invalid_grant") ->
                save("link-$id", JSONObject().put("state", if (response.body.optString("error") == "access_denied") "denied" else "expired"))
            else -> throw ApiError(502, "Google couldn’t complete the connection. Try again in a moment.")
        }
        statusLocked(id)
    }

    fun cancel(id: String): JSONObject = synchronized(lock(id)) { save("link-$id", null); statusLocked(id) }
    /** Local device disconnect. Revoking the Google grant would also revoke other devices using
     * the same Google account/project. The UI links to Google's own access-management screen. */
    fun disconnect(id: String): JSONObject = synchronized(lock(id)) {
        save("account-$id", null); save("link-$id", null); clearCache(id); statusLocked(id)
    }
    private fun clearCache(id: String) { synchronized(cache) { cache.keys.removeAll { it.startsWith("$id|") } } }

    fun library(id: String, kind: String, itemId: String = "", page: String = ""): JSONObject = synchronized(lock(id)) account@ {
        if (kind !in setOf("subscriptions", "playlists", "liked", "playlist", "channel")) throw ApiError(400, "Choose subscriptions, playlists or liked videos.")
        if (page.length > 1024 || page.any { it.code !in 33..126 }) throw ApiError(400, "Invalid YouTube page.")
        if (kind == "playlist" && (!itemId.matches(Regex("[A-Za-z0-9_-]{10,160}")) || itemId in setOf("WL", "HL"))) throw ApiError(400, "Choose a saved playlist.")
        if (kind == "channel" && !itemId.matches(Regex("UC[A-Za-z0-9_-]{22}"))) throw ApiError(400, "Choose a subscribed channel.")
        if (!statusLocked(id).optBoolean("connected")) throw ApiError(409, "Connect your YouTube account on this phone first.")
        val key = "$id|$kind|$itemId|$page"
        synchronized(cache) { cache[key]?.takeIf { now() - it.at < 120_000 }?.let { return@account JSONObject(it.json) } }
        val query = mutableMapOf("part" to "snippet,contentDetails", "maxResults" to "30")
        if (page.isNotEmpty()) query["pageToken"] = page
        val resource = when (kind) {
            "subscriptions" -> { query["mine"] = "true"; query["order"] = "alphabetical"; "subscriptions" }
            "playlists" -> { query["mine"] = "true"; "playlists" }
            "liked" -> { query["myRating"] = "like"; "videos" }
            "playlist" -> { query["playlistId"] = itemId; "playlistItems" }
            else -> {
                val channel = authorised(id, "channels", mapOf("part" to "contentDetails", "id" to itemId))
                val uploads = channel.optJSONArray("items")?.optJSONObject(0)?.optJSONObject("contentDetails")?.optJSONObject("relatedPlaylists")?.optString("uploads").orEmpty()
                if (!uploads.matches(Regex("[A-Za-z0-9_-]{10,160}"))) throw ApiError(404, "This channel has no available uploads.")
                query["playlistId"] = uploads; "playlistItems"
            }
        }
        val data = authorised(id, resource, query)
        val result = YouTubeAccountData.page(data, kind)
        synchronized(cache) { cache[key] = Cached(now(), result.toString()) }
        result
    }

    private fun authorised(id: String, resource: String, query: Map<String, String>): JSONObject {
        var record = read("account-$id") ?: throw ApiError(409, "Connect YouTube on this phone first.")
        if (record.optString("state") != "connected") throw ApiError(409, "Reconnect your YouTube account on this phone.")
        if (record.optLong("expires_at") < now() + 60_000) record = refresh(id, record)
        var reply = http.get(resource, query, record.getString("access_token"))
        if (reply.status == 401) {
            record = refresh(id, record); reply = http.get(resource, query, record.getString("access_token"))
        }
        if (reply.status == 401) { reconnect(id); throw ApiError(409, "Reconnect your YouTube account on this phone.") }
        if (reply.status == 403) {
            val reason = reply.body.optJSONObject("error")?.optJSONArray("errors")?.optJSONObject(0)?.optString("reason")
            if (reason in setOf("quotaExceeded", "dailyLimitExceeded")) throw ApiError(429, "YouTube’s daily account limit has been reached. Public search still works.")
            throw ApiError(403, "YouTube couldn’t share this list. Check your account’s permission or try another list.")
        }
        if (reply.status == 404) throw ApiError(404, "This YouTube list is no longer available.")
        if (reply.status != 200) throw ApiError(502, "Couldn’t load your YouTube library. Try again.")
        return reply.body
    }
    private fun refresh(id: String, record: JSONObject): JSONObject {
        val cfg = config()
        if (record.optString("client_id") != cfg.getString("client_id")) { reconnect(id); throw ApiError(409, "Reconnect YouTube on this phone.") }
        val response = http.post("token", mapOf("client_id" to cfg.getString("client_id"), "client_secret" to cfg.getString("client_secret"),
            "refresh_token" to record.getString("refresh_token"), "grant_type" to "refresh_token"))
        if (response.status != 200) {
            if (response.body.optString("error") in setOf("invalid_grant", "access_denied")) { reconnect(id); throw ApiError(409, "Your Google connection expired or was removed. Reconnect YouTube on this phone.") }
            throw ApiError(502, "Google couldn’t refresh your connection. Try again.")
        }
        record.put("access_token", checkedToken(response.body.optString("access_token"))).put("expires_at", expiry(response.body))
        response.body.optString("refresh_token").takeIf { it.isNotEmpty() }?.let { record.put("refresh_token", checkedToken(it)) }
        save("account-$id", record); return record
    }
    private fun reconnect(id: String) { save("account-$id", JSONObject().put("state", "reconnect")); clearCache(id) }
    private fun checkedToken(value: String): String {
        if (value.length !in 8..8192 || value.any { it.code !in 33..126 }) throw ApiError(502, "Google returned an invalid connection. Try connecting again.")
        return value
    }
    private fun expiry(body: JSONObject): Long {
        val seconds = body.optLong("expires_in")
        if (seconds !in 60..86400) throw ApiError(502, "Google returned an invalid connection lifetime.")
        return now() + seconds * 1000
    }
    internal fun verificationURL(raw: String): Boolean = runCatching {
        val uri = URI(raw)
        uri.scheme == "https" && uri.host in setOf("www.google.com", "google.com", "accounts.google.com") && uri.userInfo == null && uri.port == -1 && uri.fragment == null
    }.getOrDefault(false)
}
