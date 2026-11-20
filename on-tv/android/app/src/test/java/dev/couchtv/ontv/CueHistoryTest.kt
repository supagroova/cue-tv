package dev.couchtv.ontv

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class CueHistoryTest {
    private fun meta(video: String = "tt1234:1:1") = JSONObject().put("type", "series").put("id", "tt1234").put("video", video)
        .put("title", "Sample show").put("url", "https://example.org/private-media").put("playback", "temporary-session")
    @Test fun progressSurvivesReloadWithoutStoringMediaAddressesOrSessionHandles() {
        var storage = JSONObject(); var clock = 1000L
        fun engine() = CueHistoryEngine({ JSONObject(storage.toString()) }, { storage = it }, { clock })
        engine().progress("first", meta(), 30_000, 120_000, "paused")
        assertEquals(30_000L, engine().resume("first", meta()))
        val row = engine().rows("first").getJSONObject(0)
        assertFalse(row.has("url")); assertFalse(row.has("playback")); assertEquals(.25, row.getDouble("progress"), .001)
        assertTrue(row.getBoolean("inProgress"))
        assertEquals(0, engine().rows("second").length()); assertEquals(0L, engine().resume("first", meta("tt1234:1:2")))
        clock += 10_000
        engine().progress("first", meta(), 119_000, 120_000, "stopped")
        assertFalse(engine().rows("first").getJSONObject(0).getBoolean("inProgress")); assertEquals(0L, engine().resume("first", meta()))
    }
    @Test fun aFailedStartDoesNotCreateHistoryAndInvalidProgressCannotOverwriteIt() {
        var storage = JSONObject()
        val engine = CueHistoryEngine({ storage }, { storage = it })
        engine.progress("first", meta(), 0, 0, "paused"); assertEquals(0, engine.rows("first").length())
        engine.progress("first", meta(), 7000, 120000, "playing")
        assertThrows(ApiError::class.java) { engine.progress("first", meta(), -1, 120000, "playing") }
        assertEquals(7000L, engine.resume("first", meta()))
    }
}
