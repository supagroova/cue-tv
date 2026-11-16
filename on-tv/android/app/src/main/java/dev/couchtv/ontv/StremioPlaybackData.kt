package dev.couchtv.ontv

import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.net.URI
import java.net.URLEncoder
import java.util.zip.Deflater
import java.util.zip.DeflaterOutputStream
import org.bouncycastle.util.encoders.Base64

/** Display data is separate from the private add-on URLs and stream payloads. */
internal object StremioPlaybackData {
    private val identifier = Regex("[A-Za-z0-9:_-]{1,160}")
    // Torrentio separates configuration fields with literal pipes. Encode them for Java URI
    // without decoding or rewriting the private configuration itself.
    private fun addonURI(address: String) = URI(address.replace("|", "%7C"))
    fun validate(type: String, id: String) {
        if (type !in setOf("movie", "series") || !identifier.matches(id)) throw ApiError(400, "Choose a valid Stremio title.")
    }
    fun text(raw: String, limit: Int = 300): String = raw.replace(Regex("[\\x00-\\x08\\x0b\\x0c\\x0e-\\x1f]"), " ")
        .replace(Regex("(?i)https?://\\S+|magnet:\\S+"), "[link]").trim().take(limit)
    fun image(raw: String): Any = runCatching {
        val uri = URI(raw)
        raw.takeIf { uri.scheme == "https" && uri.host != null && uri.userInfo == null }
    }.getOrNull() ?: JSONObject.NULL
    fun detail(type: String, id: String, meta: JSONObject): JSONObject {
        validate(type, id)
        val result = JSONObject().put("id", id).put("type", type)
            .put("title", text(meta.optString("name").ifBlank { id }, 180))
            .put("description", text(meta.optString("description"), 2000))
            .put("poster", image(meta.optString("poster"))).put("background", image(meta.optString("background")))
            .put("year", text(meta.optString("releaseInfo"), 30)).put("runtime", text(meta.optString("runtime"), 30))
        val source = meta.optJSONArray("videos") ?: JSONArray()
        val seen = HashSet<String>()
        val videos = ArrayList<JSONObject>()
        for (i in 0 until minOf(source.length(), 5000)) {
            val video = source.optJSONObject(i) ?: continue
            val videoId = video.optString("id")
            val season = video.optInt("season", -1); val episode = video.optInt("episode", -1)
            if (!identifier.matches(videoId) || !seen.add(videoId) || season !in 0..1000 || episode !in 0..10000) continue
            videos.add(JSONObject().put("id", videoId).put("season", season).put("episode", episode)
                .put("title", text(video.optString("title", video.optString("name")).ifBlank { "Episode $episode" }, 180))
                .put("description", text(video.optString("overview", video.optString("description")), 800))
                .put("thumbnail", image(video.optString("thumbnail"))).put("released", text(video.optString("released"), 40)))
        }
        videos.sortWith(compareBy({ it.getInt("season") }, { it.getInt("episode") }))
        return result.put("videos", JSONArray(videos))
    }
    fun accepts(manifest: JSONObject, resource: String, type: String, id: String): Boolean {
        val resources = manifest.optJSONArray("resources") ?: return false
        val types = manifest.optJSONArray("types")
        if (types != null && (0 until types.length()).none { types.optString(it) == type }) return false
        val topPrefixes = manifest.optJSONArray("idPrefixes")
        if (topPrefixes != null && (0 until topPrefixes.length()).none { id.startsWith(topPrefixes.optString(it)) }) return false
        for (i in 0 until resources.length()) {
            if (resources.opt(i) == resource) return true
            val value = resources.optJSONObject(i) ?: continue
            if (value.optString("name") != resource) continue
            val allowed = value.optJSONArray("types")
            if (allowed != null && (0 until allowed.length()).none { allowed.optString(it) == type }) continue
            val prefixes = value.optJSONArray("idPrefixes")
            if (prefixes == null || (0 until prefixes.length()).any { id.startsWith(prefixes.optString(it)) }) return true
        }
        return false
    }
    fun realDebridTorrentio(manifest: JSONObject, address: String): Boolean = runCatching {
        val uri = addonURI(address)
        manifest.optString("id") == "com.stremio.torrentio.addon" && uri.scheme == "https" &&
            uri.host == "torrentio.strem.fun" && uri.userInfo == null &&
            Regex("(?:^|[/|])realdebrid=[^/|]+").containsMatchIn(uri.path)
    }.getOrDefault(false)
    fun realDebridStream(stream: JSONObject): Boolean = runCatching {
        val uri = URI(stream.optString("url"))
        stream.optString("infoHash").isBlank() &&
            Regex("^\\[RD(?:\\+| download)?\\]", RegexOption.IGNORE_CASE).containsMatchIn(stream.optString("name")) &&
            uri.scheme == "https" && uri.host == "torrentio.strem.fun" && uri.userInfo == null &&
            Regex("^/(?:resolve/)?realdebrid/").containsMatchIn(uri.path)
    }.getOrDefault(false)
    fun resourceURL(manifest: String, resource: String, type: String, id: String): String {
        validate(type, id)
        if (resource !in setOf("meta", "stream")) throw ApiError(400, "Invalid Stremio resource.")
        val uri = runCatching { addonURI(manifest) }.getOrNull() ?: throw ApiError(502, "An add-on address is invalid.")
        if (uri.scheme != "https" || uri.host.isNullOrBlank() || uri.userInfo != null || uri.fragment != null || !uri.path.endsWith("/manifest.json"))
            throw ApiError(502, "This add-on needs a secure manifest address.")
        val path = uri.rawPath.removeSuffix("/manifest.json") + "/$resource/$type/" + component(id) + ".json"
        return uri.scheme + "://" + uri.rawAuthority + path + (uri.rawQuery?.let { "?$it" } ?: "")
    }
    fun usable(stream: JSONObject): Boolean {
        if (stream.toString().length > 16000) return false
        if (Regex("[a-fA-F0-9]{40}").matches(stream.optString("infoHash"))) return stream.optInt("fileIdx", 0) in 0..65535
        if (Regex("[A-Za-z0-9_-]{11}").matches(stream.optString("ytId"))) return true
        val uri = runCatching { URI(stream.optString("url")) }.getOrNull() ?: return false
        return uri.scheme in setOf("http", "https", "magnet") && (uri.scheme == "magnet" || uri.host != null)
    }
    fun playerURI(stream: JSONObject, provider: String, metaProvider: String, type: String, id: String, video: String): String {
        validate(type, id); validate(type, video)
        if (!usable(stream)) throw ApiError(400, "Choose another stream, or open this episode in Stremio on the TV.")
        val bytes = ByteArrayOutputStream()
        val compressor = Deflater(Deflater.NO_COMPRESSION)
        try { DeflaterOutputStream(bytes, compressor).use { it.write(stream.toString().toByteArray(Charsets.UTF_8)) } }
        finally { compressor.end() }
        val encoded = Base64.toBase64String(bytes.toByteArray())
        val values = listOf(encoded, provider, metaProvider, type, id, video)
        val uri = "stremio:///player/" + values.joinToString("/") { component(it) }
        if (uri.toByteArray(Charsets.UTF_8).size > 40000) throw ApiError(400, "This source needs to be opened in Stremio on the TV.")
        return uri
    }
    private fun component(value: String) = URLEncoder.encode(value, "UTF-8").replace("+", "%20").replace("%7E", "~")
}
