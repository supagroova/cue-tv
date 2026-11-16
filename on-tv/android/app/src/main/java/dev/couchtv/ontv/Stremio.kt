package dev.couchtv.ontv

import android.net.Uri
import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import java.net.URLEncoder

/** Read-only Stremio account and Cinemeta catalogue access. The account key stays on the TV. */
object Stremio {
    private const val KEY = "stremio_account_key"
    private const val PACKAGE = "com.stremio.one"
    private val idPattern = Regex("[A-Za-z0-9:_-]{1,160}")
    @Volatile private var linkCode: String? = null
    @Volatile private var linkUntil = 0L
    @Volatile private var cached: JSONObject? = null
    @Volatile private var cachedAt = 0L
    @Volatile private var lastLibrary = JSONArray()

    fun connected() = !Store.str(KEY).isNullOrBlank()
    internal fun accountFingerprint(): String? = Store.str(KEY)?.takeIf { it.isNotBlank() }?.let {
        java.security.MessageDigest.getInstance("SHA-256").digest(it.toByteArray()).joinToString("") { byte -> "%02x".format(byte) }
    }
    internal fun addons(): JSONArray {
        val key = Store.str(KEY)?.takeIf { it.isNotBlank() } ?: throw ApiError(409, "Connect Stremio to load your add-ons.")
        return request("https://api.strem.io/api/addonCollectionGet", JSONObject().put("type", "AddonCollectionGet").put("authKey", key).put("update", true))
            .optJSONObject("result")?.optJSONArray("addons") ?: throw ApiError(502, "Couldn’t load your Stremio add-ons. Try again.")
    }

    // These URLs are fixed here, never supplied by a phone or the catalogue.
    private fun request(url: String, data: JSONObject? = null): JSONObject {
        try {
            val c = URL(url).openConnection() as HttpURLConnection
            try {
                c.connectTimeout = 8_000; c.readTimeout = 15_000
                c.setRequestProperty("User-Agent", "Mozilla/5.0 CouchTV/1.0")
                c.setRequestProperty("Accept", "application/json")
                if (data != null) {
                    c.requestMethod = "POST"; c.doOutput = true
                    c.setRequestProperty("Content-Type", "application/json")
                    c.outputStream.use { it.write(data.toString().toByteArray(Charsets.UTF_8)) }
                }
                if (c.responseCode !in 200..299) throw ApiError(502, "Stremio could not be reached. Try again.")
                val text = c.inputStream.bufferedReader(Charsets.UTF_8).use { it.readText() }
                val result = JSONObject(text)
                if (result.has("error")) throw ApiError(502, "Stremio could not complete the request. Try linking again.")
                return result
            } finally { c.disconnect() }
        } catch (e: ApiError) { throw e }
        catch (_: Exception) { throw ApiError(502, "Stremio could not be reached. Try again.") }
    }

    /** A short-lived Stremio link. Passwords are entered only on Stremio's own site. */
    @Synchronized fun beginLink(): String {
        val r = request("https://link.stremio.com/api/v2/create?type=Create").optJSONObject("result")
            ?: throw ApiError(502, "Stremio could not start linking.")
        val code = r.optString("code")
        val link = r.optString("link")
        val uri = Uri.parse(link)
        if (!Regex("[A-Za-z0-9_-]{2,128}").matches(code) || uri.scheme != "https" || uri.host != "link.stremio.com")
            throw ApiError(502, "Stremio returned an invalid link.")
        linkCode = code; linkUntil = System.currentTimeMillis() + 5 * 60_000L
        return link
    }

    @Synchronized fun pollLink(): Boolean {
        val code = linkCode ?: return connected()
        if (System.currentTimeMillis() >= linkUntil) { linkCode = null; return false }
        val r = request("https://link.stremio.com/api/v2/read?type=Read&code=" + URLEncoder.encode(code, "UTF-8"))
            .optJSONObject("result") ?: return false
        val token = r.optString("authKey").takeIf { it.isNotBlank() } ?: return false
        val auth = request("https://api.strem.io/api/loginWithToken", JSONObject().put("type", "LoginWithToken").put("token", token))
            .optJSONObject("result")?.optString("authKey").orEmpty()
        if (auth.isEmpty()) throw ApiError(502, "Stremio did not finish linking. Try again.")
        Store.putStr(KEY, auth)
        lastLibrary = JSONArray()
        StremioPlayback.clear()
        linkCode = null; linkUntil = 0; cachedAt = 0
        return true
    }

    @Synchronized fun unlink() {
        Store.putStr(KEY, null)
        StremioPlayback.clear()
        linkCode = null; linkUntil = 0; cachedAt = 0
        lastLibrary = JSONArray()
    }

    internal fun cleanMeta(item: JSONObject): JSONObject? {
        val id = item.optString("id", item.optString("_id"))
        val type = item.optString("type")
        val name = item.optString("name")
        if (!idPattern.matches(id) || type !in setOf("movie", "series") || name.isBlank()) return null
        fun image(k: String): Any = item.optString(k).takeIf { it.startsWith("https://") } ?: JSONObject.NULL
        return JSONObject().put("id", id).put("type", type).put("title", name.take(180))
            .put("poster", image("poster")).put("background", image("background")).put("logo", image("logo"))
            .put("description", item.optString("description").take(600))
            .put("year", item.optString("releaseInfo", item.optString("year")).take(30))
    }

    /** Find an existing trending catalogue in the account's add-ons. Never returns its URL to a client. */
    internal fun trendingUrl(addons: JSONArray, wantedType: String): String? {
        for (i in 0 until minOf(addons.length(), 100)) {
            val addon = addons.optJSONObject(i) ?: continue
            val transport = addon.optString("transportUrl")
            val uri = runCatching { java.net.URI(transport) }.getOrNull() ?: continue
            if (uri.scheme != "https" || uri.host.isNullOrBlank() || uri.userInfo != null || uri.query != null || !uri.path.endsWith("/manifest.json")) continue
            val catalogs = addon.optJSONObject("manifest")?.optJSONArray("catalogs") ?: continue
            for (j in 0 until minOf(catalogs.length(), 100)) {
                val c = catalogs.optJSONObject(j) ?: continue
                val id = c.optString("id")
                if (c.optString("type") != wantedType || !Regex("[A-Za-z0-9._-]{1,120}").matches(id)) continue
                if (!(c.optString("name") + " " + id).contains("trending", ignoreCase = true)) continue
                val extra = c.optJSONArray("extra") ?: JSONArray()
                if ((0 until extra.length()).any { extra.optJSONObject(it)?.optBoolean("isRequired") == true }) continue
                return transport.substringBeforeLast('/') + "/catalog/$wantedType/$id.json"
            }
        }
        return null
    }

    @Synchronized fun discover(): JSONObject {
        val now = System.currentTimeMillis()
        if (cached != null && now - cachedAt < 15 * 60_000L) return JSONObject(cached.toString())
        fun catalog(type: String): JSONArray {
            val metas = request("https://v3-cinemeta.strem.io/catalog/$type/top.json").optJSONArray("metas") ?: JSONArray()
            val out = JSONArray()
            for (i in 0 until minOf(metas.length(), 30)) metas.optJSONObject(i)?.let { cleanMeta(it)?.let(out::put) }
            return out
        }
        return try {
            val result = JSONObject().put("series", catalog("series")).put("movies", catalog("movie"))
            val key = Store.str(KEY)
            if (!key.isNullOrBlank()) runCatching {
                val body = JSONObject().put("type", "AddonCollectionGet").put("authKey", key).put("update", true)
                val addons = request("https://api.strem.io/api/addonCollectionGet", body).optJSONObject("result")?.optJSONArray("addons") ?: JSONArray()
                for ((type, field) in listOf("series" to "trendingSeries", "movie" to "trendingMovies")) {
                    val url = trendingUrl(addons, type) ?: continue
                    val metas = request(url).optJSONArray("metas") ?: continue
                    val out = JSONArray()
                    for (i in 0 until minOf(metas.length(), 30)) metas.optJSONObject(i)?.let { cleanMeta(it)?.let(out::put) }
                    if (out.length() > 0) result.put(field, out)
                }
            }
            result.also { cached = it; cachedAt = now }
        } catch (e: ApiError) { cached?.let { JSONObject(it.toString()) } ?: throw e }
    }

    internal fun searchPath(type: String, query: String): String {
        if (type !in setOf("movie", "series") || query.isBlank() || query.length > 100)
            throw ApiError(400, "Enter a title to search for.")
        val value = URLEncoder.encode(query.trim(), "UTF-8").replace("+", "%20")
        return "https://v3-cinemeta.strem.io/catalog/$type/top/search=$value.json"
    }

    fun search(query: String): JSONArray {
        val out = JSONArray()
        for (type in listOf("series", "movie")) {
            val metas = request(searchPath(type, query)).optJSONArray("metas") ?: continue
            for (i in 0 until minOf(metas.length(), 15)) metas.optJSONObject(i)?.let { cleanMeta(it)?.let(out::put) }
        }
        return out
    }

    /** Stremio's own library, sorted by its last-watched timestamp. Nothing is written back. */
    fun library(): JSONObject {
        val key = Store.str(KEY)?.takeIf { it.isNotBlank() } ?: return JSONObject().put("connected", false).put("items", JSONArray())
        val data = JSONObject().put("authKey", key).put("collection", "libraryItem").put("ids", JSONArray()).put("all", true)
        val values = request("https://api.strem.io/api/datastoreGet", data).optJSONArray("result")
            ?: throw ApiError(502, "Stremio did not return the library. Try linking again.")
        val items = watchedItems(values); lastLibrary = items
        return JSONObject().put("connected", true).put("items", items)
    }
    internal fun resume(meta: JSONObject): Long {
        val snapshot = lastLibrary
        return (0 until snapshot.length()).mapNotNull { snapshot.optJSONObject(it) }.firstOrNull {
            it.optString("type") == meta.optString("type") && it.optString("id") == meta.optString("id") && it.optString("video") == meta.optString("video") && it.optBoolean("inProgress")
        }?.optLong("position")?.coerceIn(0, 2_592_000_000L) ?: 0
    }

    internal fun watchedItems(values: JSONArray): JSONArray {
        val items = ArrayList<JSONObject>()
        for (i in 0 until values.length()) {
            val raw = values.optJSONObject(i) ?: continue
            val state = raw.optJSONObject("state") ?: continue
            val last = state.optString("lastWatched").takeUnless { it == "null" }.orEmpty()
            val offset = state.optLong("timeOffset", 0)
            val watched = state.optLong("overallTimeWatched", 0) > 0 || state.optInt("timesWatched", 0) > 0 || offset > 0 || last.isNotEmpty()
            if (!watched || (raw.optBoolean("removed") && !raw.optBoolean("temp"))) continue
            val item = cleanMeta(raw) ?: continue
            val imdb = item.optString("id")
            if (Regex("tt[0-9]+").matches(imdb) && item.isNull("background"))
                item.put("background", "https://images.metahub.space/background/medium/$imdb/img")
            val duration = state.optLong("duration", 0)
            item.put("lastWatched", last.ifEmpty { raw.optString("_mtime") })
                .put("position", offset).put("duration", duration).put("video", state.optString("video_id", if (item.optString("type") == "movie") imdb else ""))
                .put("progress", if (duration > 0 && offset > 0) (offset.toDouble() / duration).coerceIn(0.0, 1.0) else JSONObject.NULL)
                .put("inProgress", offset > 0)
            items.add(item)
        }
        items.sortByDescending { it.optString("lastWatched") }
        return JSONArray(items.take(60))
    }

    internal fun detailUri(type: String, id: String): String {
        if (type !in setOf("movie", "series") || !idPattern.matches(id)) throw ApiError(400, "Invalid Stremio title.")
        // A movie is a single video, so Stremio's documented detail link includes its video id.
        val video = if (type == "movie") "/$id" else ""
        return "stremio:///detail/$type/$id$video"
    }

    fun open(type: String, id: String) {
        detailUri(type, id)
        Tv.start(android.content.Intent(Store.context(), HomeActivity::class.java).putExtra("cue-type", type).putExtra("cue-id", id)
            .addFlags(android.content.Intent.FLAG_ACTIVITY_CLEAR_TOP or android.content.Intent.FLAG_ACTIVITY_SINGLE_TOP))
    }
}
