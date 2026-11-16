package dev.couchtv.ontv

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Assume.assumeTrue
import org.junit.Test

class YouTubeTest {
    private fun video(id: String = "abcdefghijk", title: String = "A video") = JSONObject().put("videoRenderer", JSONObject()
        .put("videoId", id).put("title", JSONObject().put("runs", org.json.JSONArray().put(JSONObject().put("text", title))))
        .put("ownerText", JSONObject().put("simpleText", "A channel"))
        .put("lengthText", JSONObject().put("simpleText", "12:34")))
    private fun page(contents: Any, marker: String = "var ytInitialData"): String {
        val body = if (contents is org.json.JSONArray) JSONObject().put("sectionListRenderer", JSONObject().put("contents", contents)) else contents
        return "<script>$marker = ${JSONObject().put("contents", body)};</script>"
    }
    @Test fun extractsOnlyDisplayFieldsAndHandlesBracesInsideTitles() {
        val result = YouTube.parsePage(page(video(title = "A {video} with \"quotes\" and \\slashes")))
        assertEquals(1, result.length())
        val item = result.getJSONObject(0)
        assertEquals("A {video} with \"quotes\" and \\slashes", item.getString("title"))
        assertEquals("A channel", item.getString("channel"))
        assertEquals("12:34", item.getString("duration"))
        assertEquals("https://i.ytimg.com/vi/abcdefghijk/mqdefault.jpg", item.getString("thumbnail"))
        assertEquals(setOf("id", "title", "channel", "duration", "thumbnail"), item.keys().asSequence().toSet())
    }
    @Test fun ignoresAdsDuplicatesInvalidIdsAndSecondarySuggestions() {
        val rows = org.json.JSONArray().put(JSONObject().put("adSlotRenderer", video("advert12345")))
            .put(video()).put(video()).put(video("bad?id"))
        val contents = JSONObject().put("twoColumnSearchResultsRenderer", JSONObject()
            .put("primaryContents", JSONObject().put("contents", rows)).put("secondaryContents", video("secondary12")))
        assertEquals(1, YouTube.parsePage(page(contents)).length())
    }
    @Test fun supportsWindowAssignmentAndEmptyResults() {
        assertEquals(0, YouTube.parsePage(page(JSONObject(), "window[\"ytInitialData\"]")).length())
    }
    @Test fun rejectsConsentPagesAndTruncatedData() {
        for (html in listOf("<html>Consent required</html>", "var ytInitialData = {\"contents\":{}", "var ytInitialData = []")) {
            try { YouTube.parsePage(html); fail("Malformed page accepted") } catch (_: ApiError) { }
        }
    }
    @Test fun boundsResultCountAndIgnoresUntrustedImageAddresses() {
        val rows = org.json.JSONArray()
        repeat(30) { rows.put(video("video%06d".format(it)).also { it.getJSONObject("videoRenderer").put("thumbnail", "http://evil.invalid") }) }
        assertEquals(20, YouTube.parsePage(page(rows)).length())
    }
    @Test fun onlyValidVideoIdsBecomeWatchLinks() {
        assertEquals("https://www.youtube.com/watch?v=abc_DEF-123", YouTube.watchUri("abc_DEF-123"))
        for (id in listOf("", "short", "abcdefghijk&x=1", "../abcdefgh", "abcdefghij\n")) {
            try { YouTube.watchUri(id); fail("Invalid video accepted") } catch (_: ApiError) { }
        }
    }
    @Test fun youtubeHandoffRetainsItsServiceAndCannotOverwriteAStremioReply() {
        val queue = AppleTitleQueue()
        val stremio = queue.enqueue("series", "tt123")
        val youtube = queue.enqueue("video", "abcdefghijk", "youtube")
        assertEquals("youtube", queue.current()?.service)
        assertFalse(stremio.await(10))
        assertFalse(queue.confirm(stremio.serial, true))
        assertTrue(queue.confirm(youtube.serial, true))
        assertTrue(youtube.await(10))
    }
    @Test fun livePublicSearchWhenExplicitlyEnabled() {
        assumeTrue(System.getenv("COUCHTV_TEST_YOUTUBE_LIVE") == "1")
        val result = YouTube.search("Melbourne photography")
        assertTrue("Public YouTube search returned no videos", result.length() > 0)
        println("Live public YouTube search: ${result.length()} valid video results")
        repeat(result.length()) { YouTube.videoId(result.getJSONObject(it).getString("id")) }
    }
}
