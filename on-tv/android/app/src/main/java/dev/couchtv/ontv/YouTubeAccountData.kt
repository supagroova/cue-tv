package dev.couchtv.ontv

import org.json.JSONArray
import org.json.JSONObject
import java.net.URI

internal object YouTubeAccountData {
    fun text(value: String) = value.replace(Regex("[\\x00-\\x1f]"), " ").trim().take(300)
    fun image(snippet: JSONObject?): String {
        val thumbnails = snippet?.optJSONObject("thumbnails") ?: return ""
        for (size in listOf("medium", "high", "default")) {
            val raw = thumbnails.optJSONObject(size)?.optString("url").orEmpty()
            val safe = runCatching { val uri = URI(raw); uri.scheme == "https" && uri.userInfo == null && uri.port == -1 &&
                (uri.host == "i.ytimg.com" || uri.host == "yt3.googleusercontent.com" || uri.host?.matches(Regex("yt[1-4]\\.ggpht\\.com")) == true) }.getOrDefault(false)
            if (safe) return raw
        }
        return ""
    }
    fun page(data: JSONObject, kind: String): JSONObject {
        val items = JSONArray(); val ids = mutableSetOf<String>()
        val source = data.optJSONArray("items") ?: JSONArray()
        for (i in 0 until minOf(source.length(), 50)) {
            val item = source.optJSONObject(i) ?: continue
            val snippet = item.optJSONObject("snippet") ?: continue
            val title = text(snippet.optString("title"))
            if (title.isEmpty() || title in setOf("Deleted video", "Private video")) continue
            val id = when (kind) {
                "subscriptions" -> snippet.optJSONObject("resourceId")?.optString("channelId").orEmpty()
                "playlists", "liked" -> item.optString("id")
                else -> snippet.optJSONObject("resourceId")?.optString("videoId") ?: item.optJSONObject("contentDetails")?.optString("videoId").orEmpty()
            }
            val valid = when (kind) {
                "subscriptions" -> id.matches(Regex("UC[A-Za-z0-9_-]{22}"))
                "playlists" -> id.matches(Regex("[A-Za-z0-9_-]{10,160}"))
                else -> id.matches(Regex("[A-Za-z0-9_-]{11}"))
            }
            if (!valid || !ids.add(id)) continue
            val row = JSONObject().put("id", id).put("title", title)
            if (kind in setOf("subscriptions", "playlists")) {
                row.put("thumbnail", image(snippet))
                row.put("count", item.optJSONObject("contentDetails")?.optInt("itemCount", 0)?.coerceAtLeast(0) ?: 0)
            } else row.put("channel", text(snippet.optString("videoOwnerChannelTitle").ifEmpty { snippet.optString("channelTitle") }))
                .put("thumbnail", "https://i.ytimg.com/vi/$id/mqdefault.jpg").put("duration", duration(item.optJSONObject("contentDetails")?.optString("duration").orEmpty()))
            items.put(row)
        }
        return JSONObject().put("items", items).put("kind", if (kind in setOf("subscriptions", "playlists")) kind else "videos")
            .put("next", data.optString("nextPageToken").takeIf { it.length in 1..1024 && it.all { c -> c.code in 33..126 } } ?: JSONObject.NULL)
    }
    private fun duration(raw: String): String {
        val match = Regex("PT(?:(\\d{1,4})H)?(?:(\\d{1,2})M)?(?:(\\d{1,2})S)?").matchEntire(raw) ?: return ""
        val h = match.groupValues[1].toIntOrNull() ?: 0; val m = match.groupValues[2].toIntOrNull() ?: 0; val s = match.groupValues[3].toIntOrNull() ?: 0
        return if (h > 0) "%d:%02d:%02d".format(h, m, s) else "%d:%02d".format(m, s)
    }
}
