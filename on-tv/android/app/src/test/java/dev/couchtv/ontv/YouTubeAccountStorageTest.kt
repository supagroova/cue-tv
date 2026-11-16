package dev.couchtv.ontv

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import javax.crypto.KeyGenerator

class YouTubeAccountStorageTest {
    private fun box(): YouTubeSecretBox {
        val key = KeyGenerator.getInstance("AES").apply { init(256) }.generateKey()
        return YouTubeSecretBox { key }
    }
    @Test fun storedCredentialsAreEncryptedWithFreshNonces() {
        val box = box(); val secret = "fixture-refresh-token-never-public"
        val a = box.seal("account-alice", secret); val b = box.seal("account-alice", secret)
        assertFalse(a.contains(secret)); assertNotEquals(a, b)
        assertEquals(secret, box.open("account-alice", a))
    }
    @Test fun encryptedRecordsCannotBeSwappedBetweenPhones() {
        val box = box(); val raw = box.seal("account-alice", "fixture-refresh-token")
        try { box.open("account-beth", raw); fail("A credential could be moved to another phone") } catch (_: java.security.GeneralSecurityException) { }
    }
    @Test fun tamperingOrAReplacementEncryptionKeyFailsClosed() {
        val box = box(); val raw = box.seal("account-alice", "fixture-refresh-token")
        val envelope = JSONObject(raw); val data = envelope.getString("data")
        envelope.put("data", (if (data[0] == 'A') "B" else "A") + data.drop(1))
        try { box.open("account-alice", envelope.toString()); fail("Tampered record accepted") } catch (_: java.security.GeneralSecurityException) { }
        try { box().open("account-alice", raw); fail("Different key accepted") } catch (_: java.security.GeneralSecurityException) { }
    }
    @Test fun imageAddressesAreLimitedToYoutubeHosts() {
        fun snippet(url: String) = JSONObject().put("thumbnails", JSONObject().put("medium", JSONObject().put("url", url)))
        assertEquals("https://yt3.ggpht.com/avatar", YouTubeAccountData.image(snippet("https://yt3.ggpht.com/avatar")))
        for (bad in listOf("http://i.ytimg.com/image", "https://evil.invalid/image", "https://secret@i.ytimg.com/image")) assertEquals("", YouTubeAccountData.image(snippet(bad)))
    }
    @Test fun videoMetadataDropsPrivateDeletedAndInvalidRowsAndFormatsDuration() {
        fun video(id: String, title: String) = JSONObject().put("id", id).put("snippet", JSONObject().put("title", title).put("channelTitle", "Sample"))
            .put("contentDetails", JSONObject().put("duration", "PT1H2M3S"))
        val raw = JSONObject().put("items", JSONArray().put(video("jNQXAC9IVRw", "A video")).put(video("jNQXAC9IVRw", "Duplicate"))
            .put(video("bad?id", "Invalid")).put(video("aqz-KE-bpKQ", "Private video")))
        val result = YouTubeAccountData.page(raw, "liked")
        assertEquals(1, result.getJSONArray("items").length())
        assertEquals("1:02:03", result.getJSONArray("items").getJSONObject(0).getString("duration"))
        assertEquals("videos", result.getString("kind")); assertTrue(result.isNull("next"))
    }
}
