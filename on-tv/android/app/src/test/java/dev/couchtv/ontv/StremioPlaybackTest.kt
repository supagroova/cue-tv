package dev.couchtv.ontv

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import org.bouncycastle.util.encoders.Base64
import java.net.URLDecoder
import java.util.zip.InflaterInputStream

class StremioPlaybackTest {
    private val alice = "a".repeat(64)
    private val beth = "b".repeat(64)
    private fun episode(id: String, season: Int, number: Int) = JSONObject().put("id", id).put("season", season).put("episode", number).put("title", "Episode $number")
    private fun meta() = JSONObject().put("name", "Fixture show").put("videos", JSONArray()
        .put(episode("tt1:2:1", 2, 1)).put(episode("tt1:1:2", 1, 2)).put(episode("tt1:1:1", 1, 1)))
    private fun addon(url: String = "https://torrentio.strem.fun/realdebrid=private-key/manifest.json", name: String = "Fixture provider") = JSONObject().put("transportUrl", url)
        .put("manifest", JSONObject().put("id", "com.stremio.torrentio.addon").put("name", name).put("types", JSONArray().put("movie").put("series")).put("resources", JSONArray().put("stream")))
    private fun engine(time: () -> Long = { 1000 }, profile: () -> String? = { "account-v1" }, extra: JSONArray = JSONArray(), stream: JSONObject = JSONObject().put("url", "https://torrentio.strem.fun/realdebrid/secret-token/video.mp4")): StremioPlaybackEngine =
        StremioPlaybackEngine(profile, { JSONArray().put(addon()).also { out -> for (i in 0 until extra.length()) out.put(extra.get(i)) } }, { url ->
            when {
                url.contains("/meta/series/") -> JSONObject().put("meta", meta())
                url.contains("/meta/movie/") -> JSONObject().put("meta", JSONObject().put("name", "Fixture film"))
                url.contains("unavailable-key") -> throw IllegalStateException("fixture private URL must not reach the response")
                else -> JSONObject().put("streams", JSONArray().put(JSONObject(stream.toString()).put("name", "[RD+] 1080p").put("title", "Fixture filename\n2 GB")))
            }
        }, time)
    private fun failure(status: Int, operation: () -> Unit) {
        try { operation(); fail("Invalid operation was accepted") } catch (error: ApiError) { assertEquals(status, error.status) }
    }
    @Test fun `episodes are sorted and invalid duplicate records are removed`() {
        val source = meta().getJSONArray("videos").put(episode("tt1:1:1", 1, 1)).put(episode("bad/id", 1, 3)).put(episode("tt1:1:9", -1, 9))
        val result = StremioPlaybackData.detail("series", "tt1", JSONObject().put("name", "Show").put("videos", source)).getJSONArray("videos")
        assertEquals(3, result.length()); assertEquals("tt1:1:1", result.getJSONObject(0).getString("id")); assertEquals(2, result.getJSONObject(2).getInt("season"))
    }
    @Test fun `resource paths preserve configured credentials without accepting phone URL fragments`() {
        assertEquals("https://torrentio.strem.fun/providers=yts%7Crealdebrid=private/stream/series/tt1%3A1%3A2.json",
            StremioPlaybackData.resourceURL("https://torrentio.strem.fun/providers=yts|realdebrid=private/manifest.json", "stream", "series", "tt1:1:2"))
        assertEquals("https://addon.example/private%2Fpath/stream/series/tt1%3A1%3A2.json?key=private",
            StremioPlaybackData.resourceURL("https://addon.example/private%2Fpath/manifest.json?key=private", "stream", "series", "tt1:1:2"))
        for (url in listOf("http://addon.example/manifest.json", "https://user:secret@addon.example/manifest.json", "file:///manifest.json", "https://addon.example/manifest.json#fragment")) {
            failure(502) { StremioPlaybackData.resourceURL(url, "stream", "series", "tt1") }
        }
        failure(400) { StremioPlaybackData.resourceURL("https://addon.example/manifest.json", "stream", "series", "../private") }
    }
    @Test fun `manifest resources and prefixes limit provider requests`() {
        val manifest = JSONObject().put("types", JSONArray().put("series")).put("resources", JSONArray().put(JSONObject().put("name", "stream").put("types", JSONArray().put("series")).put("idPrefixes", JSONArray().put("tt"))))
        assertTrue(StremioPlaybackData.accepts(manifest, "stream", "series", "tt1:1:1"))
        assertFalse(StremioPlaybackData.accepts(manifest, "stream", "movie", "tt1"))
        assertFalse(StremioPlaybackData.accepts(manifest, "stream", "series", "kitsu:1"))
        assertFalse(StremioPlaybackData.accepts(manifest, "meta", "series", "tt1"))
    }
    @Test fun `phone rows contain no private stream or add-on URL`() {
        val rows = engine().sources(alice, "series", "tt1", "tt1:1:2")
        val row = rows.getJSONArray("items").getJSONObject(0)
        assertTrue(row.getString("id").matches(Regex("[a-f0-9]{32}")))
        assertEquals("Fixture provider", row.getString("provider")); assertTrue(row.getBoolean("playable"))
        assertFalse(rows.toString().contains("private-key")); assertFalse(rows.toString().contains("secret-token"))
        for (key in listOf("url", "uri", "infoHash", "transportUrl", "stream")) assertFalse(row.has(key))
    }
    @Test fun `selected source uses Stremio core player protocol with exact episode and payload`() {
        val engine = engine()
        val row = engine.sources(alice, "series", "tt1", "tt1:1:2").getJSONArray("items").getJSONObject(0)
        val uri = engine.selected(alice, row.getString("id"))
        val fields = uri.removePrefix("stremio:///player/").split('/').map { URLDecoder.decode(it, "UTF-8") }
        assertEquals(6, fields.size)
        val original = JSONObject(InflaterInputStream(Base64.decode(fields[0]).inputStream()).readBytes().toString(Charsets.UTF_8))
        assertEquals("https://torrentio.strem.fun/realdebrid/secret-token/video.mp4", original.getString("url"))
        assertEquals("https://torrentio.strem.fun/realdebrid=private-key/manifest.json", fields[1])
        assertEquals(StremioPlaybackEngine.CINEMETA, fields[2]); assertEquals(listOf("series", "tt1", "tt1:1:2"), fields.takeLast(3))
    }
    @Test fun `upstream core fixture is decoded by the same selected-stream protocol`() {
        val stream = JSONObject().put("url", "http://domain.root/some/path")
        val uri = StremioPlaybackData.playerURI(stream, "https://addon.example/manifest.json", StremioPlaybackEngine.CINEMETA, "movie", "tt1", "tt1")
        val encoded = URLDecoder.decode(uri.removePrefix("stremio:///player/").substringBefore('/'), "UTF-8")
        val upstream = "eAEBJgDZ/3sidXJsIjoiaHR0cDovL2RvbWFpbi5yb290L3NvbWUvcGF0aCJ9AYANjw=="
        assertEquals(String(InflaterInputStream(Base64.decode(upstream).inputStream()).readBytes()), String(InflaterInputStream(Base64.decode(encoded).inputStream()).readBytes()))
    }
    @Test fun `torrent selection preserves file index trackers and subtitle hints`() {
        val stream = JSONObject().put("infoHash", "a".repeat(40)).put("fileIdx", 4).put("sources", JSONArray().put("tracker:https://tracker.example/announce"))
            .put("behaviorHints", JSONObject().put("filename", "Episode.mkv").put("bingeGroup", "fixture-1080p"))
        assertTrue(StremioPlaybackData.usable(stream))
        val uri = StremioPlaybackData.playerURI(stream, "https://addon.example/manifest.json", StremioPlaybackEngine.CINEMETA, "series", "tt1", "tt1:1:2")
        val raw = URLDecoder.decode(uri.removePrefix("stremio:///player/").substringBefore('/'), "UTF-8")
        val decoded = JSONObject(InflaterInputStream(Base64.decode(raw).inputStream()).readBytes().toString(Charsets.UTF_8))
        assertEquals(4, decoded.getInt("fileIdx")); assertEquals("fixture-1080p", decoded.getJSONObject("behaviorHints").getString("bingeGroup"))
        assertEquals(stream.getJSONArray("sources").toString(), decoded.getJSONArray("sources").toString())
    }
    @Test fun `one phone cannot select another phone's cached source`() {
        val engine = engine(); val id = engine.sources(alice, "movie", "tt1", "tt1").getJSONArray("items").getJSONObject(0).getString("id")
        failure(409) { engine.selected(beth, id) }; failure(401) { engine.selected("invalid", id) }
        assertTrue(engine.selected(alice, id).startsWith("stremio:///player/"))
    }
    @Test fun `expiry and account changes invalidate private source choices`() {
        var time = 1000L; var version = "account-one"
        val engine = engine(time = { time }, profile = { version })
        fun select() = engine.sources(alice, "movie", "tt1", "tt1").getJSONArray("items").getJSONObject(0).getString("id")
        val old = select(); time += 10 * 60_000 + 1; failure(409) { engine.selected(alice, old) }
        val fresh = select(); version = "account-two"; failure(409) { engine.selected(alice, fresh) }
    }
    @Test fun `private TV handoff tickets expire and are cleared after use`() {
        var time = 1000L; val engine = engine(time = { time })
        val ticket = engine.ticket("stremio:///player/private-fixture")
        assertEquals("stremio:///player/private-fixture", engine.player(ticket).getString("uri"))
        engine.finish(ticket); failure(404) { engine.player(ticket) }
        val expired = engine.ticket("stremio:///player/private-fixture"); time += 25001
        failure(404) { engine.player(expired) }
    }
    @Test fun `native playback keeps the exact episode and private URL outside phone rows`() {
        val engine = engine()
        val rows = engine.sources(alice, "series", "tt1", "tt1:1:2")
        val handle = rows.getJSONArray("items").getJSONObject(0).getString("id")
        val media = engine.nativePlayer(engine.selected(alice, handle))
        assertEquals("series", media.getString("type")); assertEquals("tt1:1:2", media.getString("video"))
        assertTrue(media.getString("url").startsWith("https://torrentio.strem.fun/"))
        assertFalse(rows.toString().contains(media.getString("url")))
        assertEquals(1, media.getInt("season")); assertEquals(2, media.getInt("episode"))
    }
    @Test fun `partial provider failures preserve available sources without private errors`() {
        val engine = engine(extra = JSONArray().put(addon("https://torrentio.strem.fun/realdebrid=unavailable-key/manifest.json", "Slow provider")))
        val result = engine.sources(alice, "movie", "tt1", "tt1")
        assertEquals(2, result.getInt("providers")); assertEquals(1, result.getInt("failed")); assertEquals(1, result.getJSONArray("items").length())
        assertFalse(result.toString().contains("private URL"))
    }
    @Test fun `episode must belong to the selected show and movie video matches movie id`() {
        failure(400) { engine().sources(alice, "series", "tt1", "tt9:1:1") }
        failure(400) { engine().sources(alice, "movie", "tt1", "tt2") }
    }
    @Test fun `external websites are visible but cannot be passed as a playable source`() {
        val result = engine(stream = JSONObject().put("externalUrl", "https://website.example/watch")).sources(alice, "movie", "tt1", "tt1")
        assertEquals(0, result.getJSONArray("items").length())
        assertFalse(result.toString().contains("website.example"))
    }
    @Test fun `only configured official Torrentio Real Debrid providers are queried`() {
        val calls = ArrayList<String>()
        val official = addon()
        val rawTorrentio = addon("https://torrentio.strem.fun/manifest.json")
        val other = addon("https://another.example/realdebrid=private/manifest.json")
        val engine = StremioPlaybackEngine({ "profile" }, { JSONArray().put(official).put(rawTorrentio).put(other) }, { url ->
            synchronized(calls) { calls.add(url) }
            if (url.contains("/meta/")) JSONObject().put("meta", JSONObject().put("name", "Film"))
            else JSONObject().put("streams", JSONArray()
                .put(JSONObject().put("name", "[RD+] Torrentio 1080p").put("url", "https://torrentio.strem.fun/realdebrid/private/file"))
                .put(JSONObject().put("name", "Torrentio 1080p").put("infoHash", "a".repeat(40)))
                .put(JSONObject().put("name", "[AD+] Torrentio 1080p").put("url", "https://torrentio.strem.fun/alldebrid/private/file")))
        })
        val result = engine.sources(alice, "movie", "tt1", "tt1")
        assertEquals(1, result.getInt("providers")); assertEquals(1, result.getJSONArray("items").length())
        assertEquals("torrentio-realdebrid", result.getString("restriction"))
        assertEquals(1, calls.count { it.contains("/stream/") })
        assertTrue(calls.none { it.contains("another.example") })
    }
    @Test fun `Real Debrid restriction rejects raw torrents other debrid and misleading labels`() {
        fun stream(name: String, url: String) = JSONObject().put("name", name).put("url", url)
        assertTrue(StremioPlaybackData.realDebridStream(stream("[RD+] Torrentio 4K", "https://torrentio.strem.fun/realdebrid/private/file")))
        assertTrue(StremioPlaybackData.realDebridStream(stream("[RD download] Torrentio", "https://torrentio.strem.fun/resolve/realdebrid/private/file")))
        for (value in listOf(
            stream("[RD+] Torrentio", "https://other.example/realdebrid/private/file"),
            stream("[AD+] Torrentio", "https://torrentio.strem.fun/alldebrid/private/file"),
            stream("[RD+] Torrentio", "magnet:?xt=urn:btih:fixture"),
            stream("[RD+] Torrentio", "https://torrentio.strem.fun/realdebrid/private/file").put("infoHash", "a".repeat(40)),
        )) assertFalse(StremioPlaybackData.realDebridStream(value))
        val manifest = addon().getJSONObject("manifest")
        assertTrue(StremioPlaybackData.realDebridTorrentio(manifest, "https://torrentio.strem.fun/providers=yts|realdebrid=private|sort=quality/manifest.json"))
        assertFalse(StremioPlaybackData.realDebridTorrentio(manifest, "https://torrentio.strem.fun/manifest.json"))
        assertFalse(StremioPlaybackData.realDebridTorrentio(manifest, "https://torrentio.strem.fun.evil.example/realdebrid=private/manifest.json"))
    }
}
