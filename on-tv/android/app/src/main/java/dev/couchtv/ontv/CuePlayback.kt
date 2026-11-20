package dev.couchtv.ontv

import org.json.JSONArray
import org.json.JSONObject
import java.security.SecureRandom
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone

/** Cue progress is local household data; the existing Stremio account is never overwritten. */
internal class CueHistoryEngine(private val read: () -> JSONObject, private val write: (JSONObject) -> Unit, private val now: () -> Long = System::currentTimeMillis) {
    @Synchronized fun progress(profile: String, meta: JSONObject, position: Long, duration: Long, state: String) {
        if (position !in 0..2_592_000_000L || duration !in 0..2_592_000_000L || state !in setOf("playing", "paused", "stopped", "ended")) throw ApiError(400, "Invalid playback progress.")
        if (position < 5_000 && state != "ended") return
        val data = read(); val rows = data.optJSONArray(profile) ?: JSONArray()
        val value = JSONObject(meta.toString()); value.remove("url"); value.remove("playback")
        val offset = if (duration > 0) position.coerceAtMost(duration) else position
        val finished = state == "ended" || duration > 0 && offset >= duration * .95
        val date = SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss.SSS'Z'", Locale.US).apply { timeZone = TimeZone.getTimeZone("UTC") }.format(Date(now()))
        value.put("position", offset).put("duration", duration).put("lastWatched", date).put("inProgress", !finished)
            .put("progress", if (duration > 0) offset.toDouble() / duration else 0.0)
        val next = JSONArray().put(value)
        for (i in 0 until minOf(rows.length(), 99)) rows.optJSONObject(i)?.takeIf {
            it.optString("type") != value.optString("type") || it.optString("id") != value.optString("id")
        }?.let(next::put)
        data.put(profile, next); write(data)
    }
    @Synchronized fun rows(profile: String): JSONArray = read().optJSONArray(profile) ?: JSONArray()
    @Synchronized fun resume(profile: String, meta: JSONObject): Long {
        val rows = rows(profile)
        return (0 until rows.length()).mapNotNull { rows.optJSONObject(it) }.firstOrNull {
            it.optString("type") == meta.optString("type") && it.optString("id") == meta.optString("id") &&
                it.optString("video") == meta.optString("video") && it.optBoolean("inProgress")
        }?.optLong("position") ?: 0
    }
}

internal object CuePlayback {
    private data class Session(val profile: String, val meta: JSONObject, val at: Long)
    private val sessions = LinkedHashMap<String, Session>()
    private val history = CueHistoryEngine({ Store.readJson("cue-history.json") as? JSONObject ?: JSONObject() }, { Store.writeJson("cue-history.json", it) })
    @Synchronized fun prepare(media: JSONObject): JSONObject {
        val profile = Stremio.accountFingerprint() ?: throw ApiError(409, "Connect Stremio first.")
        val id = ByteArray(16).also { SecureRandom().nextBytes(it) }.joinToString("") { "%02x".format(it) }
        val meta = JSONObject(media.toString()).also { it.remove("url") }
        while (sessions.size >= 8) sessions.remove(sessions.keys.first())
        sessions[id] = Session(profile, meta, System.currentTimeMillis())
        val local = history.rows(profile)
        val hasLocal = (0 until local.length()).mapNotNull { local.optJSONObject(it) }.any { it.optString("type") == meta.optString("type") && it.optString("id") == meta.optString("id") }
        return JSONObject(media.toString()).put("playback", id).put("resume", if (hasLocal) history.resume(profile, meta) else Stremio.resume(meta))
    }
    @Synchronized fun progress(body: JSONObject) {
        val value = sessions[body.optString("playback")]?.takeIf { it.profile == Stremio.accountFingerprint() && System.currentTimeMillis() - it.at < 24 * 60 * 60_000L }
            ?: throw ApiError(409, "This playback session has ended.")
        history.progress(value.profile, value.meta, body.optLong("position", -1), body.optLong("duration", -1), body.optString("state"))
    }
    fun merge(remote: JSONObject): JSONObject {
        val profile = Stremio.accountFingerprint() ?: return remote
        val combined = LinkedHashMap<String, JSONObject>()
        val incoming = remote.optJSONArray("items") ?: JSONArray()
        val local = history.rows(profile)
        for (rows in listOf(incoming, local)) for (i in 0 until rows.length()) rows.optJSONObject(i)?.let { item ->
            val key = item.optString("type") + ":" + item.optString("id")
            if (combined[key]?.optString("lastWatched").orEmpty() <= item.optString("lastWatched")) combined[key] = item
        }
        return JSONObject(remote.toString()).put("items", JSONArray(combined.values.sortedByDescending { it.optString("lastWatched") }.take(100)))
    }
}
