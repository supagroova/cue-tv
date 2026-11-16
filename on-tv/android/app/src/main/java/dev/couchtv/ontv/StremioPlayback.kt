package dev.couchtv.ontv

import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.InetAddress
import java.net.URI
import java.security.SecureRandom
import java.util.concurrent.Callable
import java.util.concurrent.ExecutorCompletionService
import java.util.concurrent.Executors
import java.util.concurrent.TimeUnit

/** The account and add-on credentials remain on the TV. Phones receive display fields and opaque choices. */
internal class StremioPlaybackEngine(
    private val profile: () -> String?,
    private val installed: () -> JSONArray,
    private val fetch: (String) -> JSONObject,
    private val now: () -> Long = System::currentTimeMillis,
) {
    private data class Meta(val display: JSONObject, val provider: String, val at: Long)
    private data class Addon(val url: String, val manifest: JSONObject, val name: String)
    private data class Choice(val owner: String, val profile: String, val uri: String, val at: Long)
    private data class Ticket(val uri: String, val at: Long)
    private data class Group(val index: Int, val addon: Addon, val streams: JSONArray?, val failed: Boolean)
    private val metadata = LinkedHashMap<String, Meta>()
    private val choices = LinkedHashMap<String, Choice>()
    private val tickets = LinkedHashMap<String, Ticket>()
    private var addonSnapshot: Pair<Long, List<Addon>>? = null
    private var savedProfile: String? = null
    private val random = SecureRandom()
    private fun opaque() = ByteArray(16).also(random::nextBytes).joinToString("") { "%02x".format(it) }
    @Synchronized private fun checkProfile(): String? {
        val current = profile()
        if (savedProfile != current) { metadata.clear(); choices.clear(); tickets.clear(); addonSnapshot = null; savedProfile = current }
        val time = now()
        choices.entries.removeAll { time - it.value.at > 10 * 60_000 }
        tickets.entries.removeAll { time - it.value.at > 25_000 }
        return current
    }
    @Synchronized fun clear() { metadata.clear(); choices.clear(); tickets.clear(); addonSnapshot = null; savedProfile = null }
    private fun addons(): List<Addon> {
        val version = checkProfile() ?: throw ApiError(409, "Connect Stremio to load your add-ons.")
        synchronized(this) { addonSnapshot?.takeIf { now() - it.first < 5 * 60_000 }?.let { return it.second } }
        val raw = installed()
        val result = ArrayList<Addon>()
        for (i in 0 until minOf(raw.length(), 40)) {
            val value = raw.optJSONObject(i) ?: continue
            val url = value.optString("transportUrl")
            if (runCatching { StremioPlaybackData.resourceURL(url, "meta", "movie", "tt1") }.isFailure) continue
            val manifest = value.optJSONObject("manifest") ?: runCatching { fetch(url) }.getOrNull() ?: continue
            result.add(Addon(url, manifest, StremioPlaybackData.text(manifest.optString("name").ifBlank { "Stremio add-on" }, 80)))
        }
        synchronized(this) {
            if (checkProfile() != version) throw ApiError(409, "The Stremio account changed. Load this title again.")
            addonSnapshot = now() to result
        }
        return result
    }
    private fun meta(type: String, id: String): Meta {
        StremioPlaybackData.validate(type, id)
        val version = checkProfile()
        val key = "$type:$id"
        synchronized(this) { metadata[key]?.takeIf { now() - it.at < 15 * 60_000 }?.let { return it } }
        val providers = sequence {
            if (id.matches(Regex("tt[0-9]+"))) yield(CINEMETA)
            if (version != null) yieldAll(addons().filter { StremioPlaybackData.accepts(it.manifest, "meta", type, id) }.map { it.url }.take(3))
        }
        for (provider in providers.distinct()) {
            val raw = runCatching { fetch(StremioPlaybackData.resourceURL(provider, "meta", type, id)).optJSONObject("meta") }.getOrNull() ?: continue
            val result = Meta(StremioPlaybackData.detail(type, id, raw), provider, now())
            synchronized(this) {
                if (checkProfile() != version) throw ApiError(409, "The Stremio account changed. Load this title again.")
                while (metadata.size >= 40) metadata.remove(metadata.keys.first())
                metadata[key] = result
            }
            return result
        }
        throw ApiError(502, "Couldn’t load this title’s details. Try again or open it in Stremio on the TV.")
    }
    fun detail(type: String, id: String): JSONObject = JSONObject(meta(type, id).display.toString())
    fun sources(owner: String, type: String, id: String, video: String): JSONObject {
        requireOwner(owner)
        StremioPlaybackData.validate(type, id); StremioPlaybackData.validate(type, video)
        val version = checkProfile() ?: throw ApiError(409, "Connect Stremio to load your add-ons.")
        val meta = meta(type, id)
        val episodes = meta.display.getJSONArray("videos")
        if ((type == "movie" && video != id) || (type == "series" && (0 until episodes.length()).none { episodes.getJSONObject(it).getString("id") == video }))
            throw ApiError(400, "Choose an episode from this show.")
        val installedAddons = addons()
        val providers = installedAddons.filter { StremioPlaybackData.realDebridTorrentio(it.manifest, it.url) && StremioPlaybackData.accepts(it.manifest, "stream", type, video) }.take(24)
        val service = ExecutorCompletionService<Group>(pool)
        val futures = providers.mapIndexed { index, addon -> service.submit(Callable {
            try { Group(index, addon, fetch(StremioPlaybackData.resourceURL(addon.url, "stream", type, video)).optJSONArray("streams") ?: JSONArray(), false) }
            catch (_: Exception) { Group(index, addon, null, true) }
        }) }
        val results = ArrayList<Group>()
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(22)
        try {
            repeat(providers.size) {
                val left = deadline - System.nanoTime()
                if (left > 0) service.poll(left, TimeUnit.NANOSECONDS)?.let { results.add(it.get()) }
            }
        } finally { futures.forEach { if (!it.isDone) it.cancel(true) } }
        val rows = JSONArray()
        var failed = providers.size - results.size
        synchronized(this) {
            if (checkProfile() != version) throw ApiError(409, "The Stremio account changed. Load these sources again.")
            for (group in results.sortedBy { it.index }) {
                if (group.failed) { failed++; continue }
                val streams = group.streams ?: continue
                for (i in 0 until minOf(streams.length(), 100)) {
                    if (rows.length() >= 200) break
                    val stream = streams.optJSONObject(i) ?: continue
                    if (!StremioPlaybackData.realDebridStream(stream)) continue
                    val uri = if (StremioPlaybackData.usable(stream)) runCatching {
                        StremioPlaybackData.playerURI(stream, group.addon.url, meta.provider, type, id, video)
                    }.getOrNull() else null
                    val usable = uri != null
                    val handle = opaque()
                    if (uri != null) {
                        while (choices.size >= 400) choices.remove(choices.keys.first())
                        choices[handle] = Choice(owner, version, uri, now())
                    }
                    rows.put(JSONObject().put("id", handle).put("provider", group.addon.name)
                        .put("name", StremioPlaybackData.text(stream.optString("name").ifBlank { group.addon.name }, 160))
                        .put("description", StremioPlaybackData.text(stream.optString("description", stream.optString("title")), 700))
                        .put("kind", "Real-Debrid")
                        .put("playable", usable).put("reason", if (usable) JSONObject.NULL else "Choose this source in Stremio on the TV."))
                }
            }
        }
        return JSONObject().put("items", rows).put("providers", providers.size).put("failed", failed).put("limited", rows.length() >= 200)
            .put("restriction", "torrentio-realdebrid")
            .put("setup", JSONObject().put("addons", installedAddons.size)
                .put("torrentio", installedAddons.count { it.manifest.optString("id") == "com.stremio.torrentio.addon" })
                .put("realDebrid", installedAddons.count { StremioPlaybackData.realDebridTorrentio(it.manifest, it.url) }))
    }
    fun selected(owner: String, handle: String): String = synchronized(this) {
        requireOwner(owner)
        val version = checkProfile()
        val choice = choices[handle]?.takeIf { it.owner == owner && it.profile == version }
            ?: throw ApiError(409, "The source list expired. Load sources again and choose one.")
        choice.uri
    }
    @Synchronized fun ticket(uri: String): String {
        checkProfile()
        while (tickets.size >= 16) tickets.remove(tickets.keys.first())
        return opaque().also { tickets[it] = Ticket(uri, now()) }
    }
    @Synchronized fun player(ticket: String): JSONObject {
        checkProfile()
        val found = tickets[ticket] ?: throw ApiError(404, "The playback request expired. Choose the source again.")
        return JSONObject().put("uri", found.uri)
    }
    @Synchronized fun finish(ticket: String) { tickets.remove(ticket) }
    private fun requireOwner(owner: String) {
        if (!owner.matches(Regex("[a-f0-9]{64}"))) throw ApiError(401, "Pair this phone first.")
    }
    companion object {
        const val CINEMETA = "https://v3-cinemeta.strem.io/manifest.json"
        private val pool = Executors.newFixedThreadPool(4) { runnable -> Thread(runnable, "stremio-sources").apply { isDaemon = true } }
    }
}

internal object StremioPlayback {
    private val engine = StremioPlaybackEngine(Stremio::accountFingerprint, Stremio::addons, StremioResourceHTTP::get)
    fun detail(type: String, id: String) = engine.detail(type, id)
    fun sources(owner: String, type: String, id: String, video: String) = engine.sources(owner, type, id, video)
    fun selected(owner: String, handle: String) = engine.selected(owner, handle)
    fun ticket(uri: String) = engine.ticket(uri)
    fun player(ticket: String) = engine.player(ticket)
    fun finish(ticket: String) = engine.finish(ticket)
    fun clear() = engine.clear()
}

/** Bounded add-on GET requests check redirect destinations and resolved network addresses. */
private object StremioResourceHTTP {
    fun get(input: String): JSONObject {
        var uri = URI(input)
        repeat(4) {
            if (uri.scheme != "https" || uri.host.isNullOrBlank() || uri.userInfo != null || uri.fragment != null)
                throw ApiError(502, "The add-on address could not be used.")
            val addresses = InetAddress.getAllByName(uri.host)
            if (addresses.isEmpty() || addresses.any { it.isAnyLocalAddress || it.isLoopbackAddress || it.isLinkLocalAddress || it.isSiteLocalAddress || it.isMulticastAddress ||
                    (it.address.size == 16 && (it.address[0].toInt() and 0xfe) == 0xfc) })
                throw ApiError(502, "This add-on is not available through a public secure address.")
            val connection = uri.toURL().openConnection() as HttpURLConnection
            try {
                connection.instanceFollowRedirects = false
                connection.connectTimeout = 6_000; connection.readTimeout = 10_000
                connection.setRequestProperty("Accept", "application/json")
                connection.setRequestProperty("User-Agent", "CouchTV/1.0")
                val code = connection.responseCode
                if (code in 300..399) {
                    val next = connection.getHeaderField("Location") ?: throw ApiError(502, "The add-on did not respond.")
                    uri = uri.resolve(next)
                } else {
                    if (code !in 200..299) throw ApiError(502, "The add-on did not respond.")
                    val body = connection.inputStream.use { it.readBytesBounded(4 * 1024 * 1024) }
                    return JSONObject(body.toString(Charsets.UTF_8))
                }
            } finally { connection.disconnect() }
        }
        throw ApiError(502, "The add-on could not be reached.")
    }
    private fun java.io.InputStream.readBytesBounded(limit: Int): ByteArray {
        val buffer = java.io.ByteArrayOutputStream()
        val block = ByteArray(8192)
        while (true) {
            val count = read(block); if (count < 0) break
            if (buffer.size() + count > limit) throw ApiError(502, "The add-on response was too large.")
            buffer.write(block, 0, count)
        }
        return buffer.toByteArray()
    }
}
