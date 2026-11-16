package dev.couchtv.ontv

import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.net.HttpURLConnection
import java.net.URL
import java.net.URLEncoder
import java.util.concurrent.Semaphore
import java.util.zip.GZIPInputStream

/** Public video search, without a household login or a third-party search proxy.
 * Only display fields leave the parser. YouTube's tracking data and page scripts never do. */
object YouTube {
    private const val MAX_BYTES = 6 * 1024 * 1024
    private val permits = Semaphore(2)
    private data class Cached(val at: Long, val json: String)
    private val cache = object : LinkedHashMap<String, Cached>(12, .75f, true) {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, Cached>?) = size > 12
    }
    fun videoId(id: String): String {
        if (!id.matches(Regex("[A-Za-z0-9_-]{11}"))) throw ApiError(400, "Invalid YouTube video.")
        return id
    }
    fun watchUri(id: String) = "https://www.youtube.com/watch?v=${videoId(id)}"

    fun search(raw: String): JSONArray {
        val query = raw.trim().replace(Regex("\\s+"), " ")
        if (query.isEmpty()) return JSONArray()
        if (query.length > 100 || query.any { it.code < 32 }) throw ApiError(400, "Keep the search under 100 characters.")
        synchronized(cache) { cache[query]?.takeIf { System.currentTimeMillis() - it.at < 120_000 }?.let { return JSONArray(it.json) } }
        if (!permits.tryAcquire()) throw ApiError(429, "YouTube is still searching. Try again in a moment.")
        try {
            val url = URL("https://www.youtube.com/results?search_query=${URLEncoder.encode(query, "UTF-8")}&hl=en&gl=AU")
            val connection = url.openConnection() as HttpURLConnection
            connection.connectTimeout = 8000; connection.readTimeout = 12000
            connection.instanceFollowRedirects = false
            connection.setRequestProperty("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            connection.setRequestProperty("Accept-Language", "en-AU,en;q=0.9")
            connection.setRequestProperty("Accept-Encoding", "gzip")
            val html = try {
                if (connection.responseCode != 200) throw ApiError(502, "YouTube search is unavailable. Try again or open YouTube on the TV.")
                val stream = connection.inputStream.let { if (connection.contentEncoding == "gzip") GZIPInputStream(it) else it }
                stream.use { input ->
                    val output = ByteArrayOutputStream()
                    val buffer = ByteArray(8192)
                    while (true) {
                        val n = input.read(buffer)
                        if (n < 0) break
                        if (output.size() + n > MAX_BYTES) throw ApiError(502, "YouTube returned too much data. Try a more specific search.")
                        output.write(buffer, 0, n)
                    }
                    output.toString("UTF-8")
                }
            } finally { connection.disconnect() }
            val result = parsePage(html)
            synchronized(cache) { cache[query] = Cached(System.currentTimeMillis(), result.toString()) }
            return result
        } catch (e: ApiError) { throw e }
        catch (_: Exception) { throw ApiError(502, "Couldn’t search YouTube right now. Try again or open YouTube on the TV.") }
        finally { permits.release() }
    }

    internal fun parsePage(html: String): JSONArray {
        if (html.length > MAX_BYTES) throw ApiError(502, "YouTube search is unavailable.")
        val marker = Regex("(?:var ytInitialData|window\\[\"ytInitialData\"\\])\\s*=\\s*").find(html)
            ?: throw ApiError(502, "YouTube search is unavailable. Open YouTube on the TV and try again later.")
        val start = marker.range.last + 1
        if (html.getOrNull(start) != '{') throw ApiError(502, "YouTube search is unavailable.")
        var depth = 0; var quoted = false; var escape = false; var end = -1
        for (i in start until html.length) {
            val c = html[i]
            if (quoted) {
                if (escape) escape = false else if (c == '\\') escape = true else if (c == '"') quoted = false
            } else when (c) {
                '"' -> quoted = true
                '{', '[' -> { depth++; if (depth > 64) throw ApiError(502, "YouTube search is unavailable.") }
                '}', ']' -> { depth--; if (depth == 0) { end = i + 1; break } }
            }
        }
        if (end < 0) throw ApiError(502, "YouTube search is unavailable.")
        val root = JSONObject(html.substring(start, end))
        val contents = root.optJSONObject("contents") ?: throw ApiError(502, "YouTube search is unavailable.")
        val main = contents.optJSONObject("twoColumnSearchResultsRenderer")?.optJSONObject("primaryContents") ?: contents
        return parseResults(main)
    }

    internal fun parseResults(root: Any): JSONArray {
        val items = JSONArray(); val ids = mutableSetOf<String>(); var visited = 0
        fun text(value: JSONObject?) = (value?.optString("simpleText")?.takeIf { it.isNotBlank() }
            ?: value?.optJSONArray("runs")?.let { runs -> (0 until runs.length()).joinToString("") { runs.optJSONObject(it)?.optString("text").orEmpty() } }.orEmpty())
            .replace(Regex("[\\x00-\\x1f]"), " ").trim().take(300)
        fun visit(node: Any?, depth: Int) {
            if (depth > 48 || ++visited > 12_000 || items.length() >= 20) return
            when (node) {
                is JSONArray -> for (i in 0 until node.length()) visit(node.opt(i), depth + 1)
                is JSONObject -> {
                    // Advertising and suggested query/playlist blocks are not video results.
                    val keys = node.keys().asSequence().toList()
                    if (keys.any { it.contains("promoted", true) || it.startsWith("adSlot") || it.startsWith("adPlacement") || it == "searchPyvRenderer" }) return
                    val video = node.optJSONObject("videoRenderer")
                    if (video != null) {
                        val id = video.optString("videoId")
                        val title = text(video.optJSONObject("title"))
                        if (id.matches(Regex("[A-Za-z0-9_-]{11}")) && title.isNotEmpty() && ids.add(id)) {
                            items.put(JSONObject().put("id", id).put("title", title)
                                .put("channel", text(video.optJSONObject("ownerText") ?: video.optJSONObject("longBylineText")))
                                .put("duration", text(video.optJSONObject("lengthText")))
                                .put("thumbnail", "https://i.ytimg.com/vi/$id/mqdefault.jpg"))
                        }
                    } else keys.forEach { visit(node.opt(it), depth + 1) }
                }
            }
        }
        visit(root, 0)
        return items
    }
}
