package dev.couchtv.ontv

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.atomic.AtomicInteger

class StremioCollectionsTest {
    private fun sample(url: String): JSONObject {
        val type = if (url.contains("/series/")) "series" else "movie"
        return JSONObject().put("metas", JSONArray().put(JSONObject().put("id", "tt1234").put("name", url.substringAfterLast('/')).put("type", type)))
    }
    @Test fun categoriesUseDistinctUpstreamRankingsAndFollowTheSelectedCountry() {
        val engine = StremioCollectionsEngine(::sample)
        val rows = engine.get("AU").getJSONArray("collections")
        assertEquals(18, rows.length())
        val netflix = (0 until rows.length()).map { rows.getJSONObject(it) }.filter { it.getString("service") == "Netflix" && it.getString("type") == "series" }
        assertEquals(listOf("tnd", "pop", "new"), netflix.map { it.getString("kind") })
        assertEquals(3, netflix.map { it.getJSONArray("items").getJSONObject(0).getString("title") }.toSet().size)
        assertTrue(netflix.all { it.getString("country") == "AU" })
        assertTrue(engine.address("GB", engine.shelves().first()).contains("GB_en_jw_tnd_nfx"))
        assertThrows(IllegalArgumentException::class.java) { engine.address("../US", engine.shelves().first()) }
    }
    @Test fun aFailingProviderDoesNotReplaceTrendingWithPopularOrLoseOtherShelves() {
        var now = 1L; var failed = false
        val calls = AtomicInteger()
        val engine = StremioCollectionsEngine({ url ->
            calls.incrementAndGet()
            if (failed && url.contains("_tnd_nfx")) error("temporary outage")
            sample(url)
        }, { now })
        engine.get("AU"); engine.get("AU"); assertEquals(18, calls.get())
        failed = true; now += 31 * 60_000
        val result = engine.get("AU")
        assertEquals(2, result.getInt("failed")); assertEquals(18, result.getJSONArray("collections").length())
        val row = result.getJSONArray("collections").getJSONObject(0)
        assertTrue(row.getBoolean("stale")); assertEquals("tnd", row.getString("kind"))
        assertTrue(row.getJSONArray("items").getJSONObject(0).getString("title").contains("_tnd_nfx"))
    }
    @Test fun emptyListsStayEmptyAndNoResultsProduceARetryableError() {
        val empty = StremioCollectionsEngine({ JSONObject().put("metas", JSONArray()) }).get("AU")
        assertEquals(0, empty.getJSONArray("collections").getJSONObject(0).getJSONArray("items").length())
        val absent = StremioCollectionsEngine({ error("offline") })
        assertEquals(502, assertThrows(ApiError::class.java) { absent.get("AU") }.status)
    }
}
