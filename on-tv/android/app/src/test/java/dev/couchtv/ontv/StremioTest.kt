package dev.couchtv.ontv

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class StremioTest {
    @Test fun `search queries remain encoded path data`() {
        assertEquals("https://v3-cinemeta.strem.io/catalog/series/top/search=Bluey%20%26%20friends.json", Stremio.searchPath("series", " Bluey & friends "))
        assertTrue(Stremio.searchPath("movie", "a/b?x=#").endsWith("search=a%2Fb%3Fx%3D%23.json"))
        for (q in listOf("", " ", "a".repeat(101))) {
            try { Stremio.searchPath("movie", q); fail("accepted invalid search") } catch (e: ApiError) { assertEquals(400, e.status) }
        }
    }
    private fun title(id: String, name: String, last: String, offset: Long, duration: Long, watched: Int = 0): JSONObject =
        JSONObject().put("_id", id).put("name", name).put("type", "series").put("removed", false)
            .put("state", JSONObject().put("lastWatched", last).put("timeOffset", offset).put("duration", duration).put("timesWatched", watched))

    @Test fun `history uses actual watch state and newest order`() {
        val recent = title("tt123", "Started", "2026-11-14T12:00:00Z", 50, 100)
        val older = title("tt124", "Finished", "2026-11-12T12:00:00Z", 0, 100, 1)
        val savedOnly = title("tt125", "Saved only", "", 0, 0)
        val removed = title("tt126", "Removed", "2026-11-15T12:00:00Z", 50, 100).put("removed", true)
        val result = Stremio.watchedItems(JSONArray().put(older).put(savedOnly).put(removed).put(recent))
        assertEquals(2, result.length())
        assertEquals("Started", result.getJSONObject(0).getString("title"))
        assertTrue(result.getJSONObject(0).getBoolean("inProgress"))
        assertEquals(.5, result.getJSONObject(0).getDouble("progress"), .0001)
        assertEquals("Finished", result.getJSONObject(1).getString("title"))
        assertFalse(result.getJSONObject(1).getBoolean("inProgress"))
    }

    @Test fun `detail links are limited to safe Stremio identifiers`() {
        assertEquals("stremio:///detail/series/tt123", Stremio.detailUri("series", "tt123"))
        assertEquals("stremio:///detail/movie/tt456/tt456", Stremio.detailUri("movie", "tt456"))
        for (bad in listOf("../bad", "tt123?x=1", "foo/bar", "")) {
            try { Stremio.detailUri("series", bad); fail("accepted an invalid id") } catch (e: ApiError) { assertEquals(400, e.status) }
        }
    }

    @Test fun `trending catalogues come from existing secure addons and need no input`() {
        fun addon(url: String, extra: JSONArray = JSONArray()): JSONObject = JSONObject().put("transportUrl", url)
            .put("manifest", JSONObject().put("catalogs", JSONArray().put(JSONObject().put("type", "series")
                .put("name", "Trending shows").put("id", "trending_series").put("extra", extra))))
        val required = JSONArray().put(JSONObject().put("name", "search").put("isRequired", true))
        val all = JSONArray().put(addon("http://example.com/manifest.json"))
            .put(addon("https://example.com/manifest.json", required)).put(addon("https://catalog.example/manifest.json"))
        assertEquals("https://catalog.example/catalog/series/trending_series.json", Stremio.trendingUrl(all, "series"))
        assertNull(Stremio.trendingUrl(all, "movie"))
    }
}
