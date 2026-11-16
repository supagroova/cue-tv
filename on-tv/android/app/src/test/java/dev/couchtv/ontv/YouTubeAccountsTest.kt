package dev.couchtv.ontv

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class YouTubeAccountsTest {
    private val alice = "a".repeat(64)
    private val beth = "b".repeat(64)
    private class Memory : YouTubeVault {
        val data = mutableMapOf<String, String>()
        override fun get(key: String) = data[key]
        override fun put(key: String, value: String?) { if (value == null) data.remove(key) else data[key] = value }
    }
    private class Fixture {
        var time = 100_000L
        val vault = Memory()
        var deviceSequence = 0
        var polls = 0
        var refreshes = 0
        var reads = 0
        var decision = "authorization_pending"
        var owner = "Alice"
        var refreshError = false
        var unauthorized = false
        var insufficientScope = false
        var quota = false
        var lastToken = ""
        var verification = "https://www.google.com/device"
        val http = object : YouTubeTransport {
            override fun post(endpoint: String, fields: Map<String, String>): YouTubeReply {
                if (endpoint == "device/code") {
                    assertEquals(YouTubeAccounts.SCOPE, fields["scope"])
                    return YouTubeReply(200, JSONObject().put("user_code", "CODE-${++deviceSequence}").put("device_code", "fixture-device-$deviceSequence")
                        .put("verification_url", verification).put("interval", 5).put("expires_in", 1800))
                }
                if (fields["grant_type"] == "refresh_token") {
                    refreshes++
                    if (refreshError) return YouTubeReply(400, JSONObject().put("error", "invalid_grant"))
                    return YouTubeReply(200, JSONObject().put("access_token", "refreshed-access-$owner").put("expires_in", 3600))
                }
                polls++
                if (decision != "approved") return YouTubeReply(400, JSONObject().put("error", decision))
                return YouTubeReply(200, JSONObject().put("access_token", "fixture-access-$owner").put("refresh_token", "fixture-refresh-$owner")
                    .put("scope", if (insufficientScope) "openid" else YouTubeAccounts.SCOPE).put("expires_in", 3600).put("token_type", "Bearer"))
            }
            override fun get(resource: String, query: Map<String, String>, token: String): YouTubeReply {
                reads++; lastToken = token
                if (unauthorized) { unauthorized = false; return YouTubeReply(401, JSONObject()) }
                if (quota) return YouTubeReply(403, JSONObject().put("error", JSONObject().put("errors", JSONArray().put(JSONObject().put("reason", "quotaExceeded")))))
                val title = token.substringAfterLast('-')
                if (resource == "channels") return YouTubeReply(200, JSONObject().put("items", JSONArray().put(JSONObject().put("id", "UC" + "a".repeat(22))
                    .put("snippet", JSONObject().put("title", title)).put("contentDetails", JSONObject().put("relatedPlaylists", JSONObject().put("uploads", "UU" + "a".repeat(22)))))))
                val row = JSONObject().put("id", if (resource == "playlists") "PL" + "a".repeat(20) else "jNQXAC9IVRw")
                    .put("snippet", JSONObject().put("title", "$title video").put("channelTitle", title).put("resourceId", JSONObject().put("videoId", "jNQXAC9IVRw")))
                    .put("contentDetails", JSONObject().put("duration", "PT1H2M3S").put("itemCount", 12))
                return YouTubeReply(200, JSONObject().put("items", JSONArray().put(row)).put("nextPageToken", "page-2"))
            }
        }
        val engine = YouTubeAccounts(vault, http) { time }
        init { engine.configure(JSONObject().put("client_id", "123456789-tvfixture.apps.googleusercontent.com").put("client_secret", "fixture-client-secret")) }
        fun connect(id: String, name: String = "Alice"): JSONObject {
            owner = name; engine.begin(id); time += 6000; decision = "approved"
            return engine.poll(id)
        }
    }
    @Test fun connectionIsPrivateToEachPairedClient() {
        val f = Fixture(); val a = f.connect(alice)
        assertTrue(a.getBoolean("connected")); assertFalse(f.engine.status(beth).getBoolean("connected"))
        f.connect(beth, "Beth")
        assertEquals("Alice", f.engine.status(alice).getJSONObject("account").getString("title"))
        assertEquals("Beth", f.engine.status(beth).getJSONObject("account").getString("title"))
        assertNotEquals(f.engine.status(alice).getString("connection"), f.engine.status(beth).getString("connection"))
    }
    @Test fun publicStateNeverContainsGoogleCredentials() {
        val f = Fixture(); val pending = f.engine.begin(alice).toString()
        assertTrue(pending.contains("CODE-1"))
        for (secret in listOf("fixture-device", "client_secret", "refresh_token", "access_token")) assertFalse(pending.contains(secret))
        f.time += 6000; f.decision = "approved"
        val connected = f.engine.poll(alice).toString()
        for (secret in listOf("fixture-access", "fixture-refresh", "client_secret", "device_code")) assertFalse(connected.contains(secret))
    }
    @Test fun googlePollingIntervalIsEnforcedAcrossRepeatedPhoneRequests() {
        val f = Fixture(); f.engine.begin(alice)
        repeat(10) { f.engine.poll(alice) }; assertEquals(0, f.polls)
        f.time += 5000; f.engine.poll(alice); assertEquals(1, f.polls)
        repeat(10) { f.engine.poll(alice) }; assertEquals(1, f.polls)
    }
    @Test fun slowDownExtendsThePollingInterval() {
        val f = Fixture(); f.engine.begin(alice); f.time += 5000; f.decision = "slow_down"
        assertEquals(10, f.engine.poll(alice).getJSONObject("link").getInt("interval"))
        f.time += 6000; f.engine.poll(alice); assertEquals(1, f.polls)
        f.time += 4000; f.engine.poll(alice); assertEquals(2, f.polls)
    }
    @Test fun expiredOrCancelledCodesCannotBeRedeemedLater() {
        val f = Fixture(); f.engine.begin(alice); f.time += 1_801_000
        assertEquals("expired", f.engine.poll(alice).getString("state")); assertEquals(0, f.polls)
        f.engine.begin(alice); f.engine.cancel(alice); f.time += 6000; f.decision = "approved"
        assertFalse(f.engine.poll(alice).getBoolean("connected")); assertEquals(0, f.polls)
    }
    @Test fun deniedAccessDoesNotTouchTheOtherPhone() {
        val f = Fixture(); f.connect(alice); f.engine.begin(beth); f.time += 6000; f.decision = "access_denied"
        assertEquals("denied", f.engine.poll(beth).getString("state")); assertTrue(f.engine.status(alice).getBoolean("connected"))
    }
    @Test fun missingYoutubePermissionCannotBecomeAConnection() {
        val f = Fixture(); f.insufficientScope = true
        try { f.connect(alice); fail("A partial Google grant was accepted") } catch (e: ApiError) { assertEquals(403, e.status) }
        assertFalse(f.engine.status(alice).getBoolean("connected"))
    }
    @Test fun cachedPrivateListsCannotCrossPhoneConnections() {
        val f = Fixture(); f.connect(alice); f.connect(beth, "Beth")
        assertEquals("Alice video", f.engine.library(alice, "liked").getJSONArray("items").getJSONObject(0).getString("title"))
        val before = f.reads; f.engine.library(alice, "liked"); assertEquals(before, f.reads)
        assertEquals("Beth video", f.engine.library(beth, "liked").getJSONArray("items").getJSONObject(0).getString("title"))
    }
    @Test fun disconnectClearsOnlyThisPhonesCredentialsAndCachedLists() {
        val f = Fixture(); f.connect(alice); f.connect(beth, "Beth"); f.engine.library(alice, "liked")
        f.engine.disconnect(alice)
        assertFalse(f.vault.data.containsKey("account-$alice")); assertTrue(f.engine.status(beth).getBoolean("connected"))
        try { f.engine.library(alice, "liked"); fail("Disconnected account retained access") } catch (e: ApiError) { assertEquals(409, e.status) }
        f.connect(alice, "NewAccount")
        assertEquals("NewAccount video", f.engine.library(alice, "liked").getJSONArray("items").getJSONObject(0).getString("title"))
    }
    @Test fun expiredAccessTokenRefreshesAndRetriesOneUnauthorizedRead() {
        val f = Fixture(); f.connect(alice); f.time += 3_601_000
        f.engine.library(alice, "liked"); assertEquals(1, f.refreshes); assertTrue(f.lastToken.startsWith("refreshed-access"))
        f.unauthorized = true; f.engine.library(alice, "playlists"); assertEquals(2, f.refreshes)
    }
    @Test fun revokedGoogleGrantClearsSecretsAndRequiresReconnectOnlyForItsPhone() {
        val f = Fixture(); f.connect(alice); f.connect(beth, "Beth"); f.time += 3_601_000; f.refreshError = true
        try { f.engine.library(alice, "liked"); fail("Revoked connection was reused") } catch (e: ApiError) { assertEquals(409, e.status) }
        assertEquals("reconnect", f.engine.status(alice).getString("state")); assertFalse(f.vault.get("account-$alice")!!.contains("fixture-refresh"))
        assertTrue(f.engine.status(beth).getBoolean("connected"))
        f.refreshError = false
        assertEquals("pending", f.engine.begin(alice).getString("state"))
        f.time += 6000; f.owner = "AliceReconnected"; f.decision = "approved"
        assertTrue(f.engine.poll(alice).getBoolean("connected"))
        assertEquals("AliceReconnected", f.engine.status(alice).getJSONObject("account").getString("title"))
    }
    @Test fun malformedCallerAndUntrustedGoogleLinkAreRejected() {
        val f = Fixture()
        try { f.engine.status("somebody-else"); fail("Unbound client accepted") } catch (e: ApiError) { assertEquals(401, e.status) }
        for (link in listOf("http://www.google.com/device", "https://google.com.evil.invalid/device", "https://secret@www.google.com/device")) {
            f.verification = link
            try { f.engine.begin(alice); fail("Untrusted verification link accepted") } catch (e: ApiError) { assertEquals(502, e.status) }
        }
    }
    @Test fun replacingAnExistingAppRegistrationPreservesConnections() {
        val f = Fixture(); f.connect(alice)
        try { f.engine.configure(JSONObject().put("client_id", "123456789-other.apps.googleusercontent.com").put("client_secret", "another-fixture-secret")); fail("App identity changed") }
        catch (e: ApiError) { assertEquals(409, e.status) }
        assertTrue(f.engine.status(alice).getBoolean("connected"))
    }
    @Test fun providerQuotaErrorIsSpecificAndPreservesTheAccount() {
        val f = Fixture(); f.connect(alice); f.quota = true
        try { f.engine.library(alice, "liked"); fail("Quota response accepted") } catch (e: ApiError) { assertEquals(429, e.status) }
        assertTrue(f.engine.status(alice).getBoolean("connected"))
    }
    @Test fun unsupportedListsAndBadIdentifiersNeverReachGoogle() {
        val f = Fixture(); f.connect(alice); val before = f.reads
        for ((kind,id) in listOf("history" to "", "playlist" to "WL", "playlist" to "bad?id", "channel" to "other")) {
            try { f.engine.library(alice, kind, id); fail("Unsupported list accepted") } catch (e: ApiError) { assertEquals(400, e.status) }
        }
        assertEquals(before, f.reads)
    }
}
