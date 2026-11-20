package dev.couchtv.ontv

import java.io.File
import java.util.concurrent.TimeUnit
import org.bouncycastle.crypto.params.Ed25519PrivateKeyParameters
import org.junit.Assert.*
import org.junit.Test

class CompanionSessionTest {
    private fun peer(mode: String, block: (CompanionSession) -> Unit) {
        val python = System.getenv("COUCHTV_TEST_PYTHON") ?: error("Set COUCHTV_TEST_PYTHON to the project's .venv/bin/python for interoperability tests")
        val script = File(python).parentFile.parentFile.parentFile.resolve("tests/companion_peer.py")
        val process = ProcessBuilder(python, script.absolutePath, mode).redirectErrorStream(true).start()
        try {
            val reader = process.inputStream.bufferedReader()
            val port = reader.readLine().toInt()
            CompanionSession("127.0.0.1", port).use(block)
            assertTrue("Python peer did not finish", process.waitFor(25, TimeUnit.SECONDS))
            val report = reader.readText()
            assertEquals(report, 0, process.exitValue()); assertTrue(report, report.contains("PASS"))
        } finally { process.destroyForcibly() }
    }
    private fun credentials() = CompanionSession.Credentials("test-remote", ByteArray(32) { it.toByte() }, "test-apple-tv",
        Ed25519PrivateKeyParameters(ByteArray(32) { (it + 32).toByte() }, 0).generatePublicKey().encoded)
    @Test fun pythonPeerVerifiesIdentityAndLaunchWithInterleavedEvent() = peer("ok") {
        it.verify(credentials()); it.start(); it.prepareForApp(); it.url("stremio:///detail/series/tt4574334")
    }
    @Test fun awakePeerReceivesNoExtraBackKey() = peer("awake") {
        it.verify(credentials()); it.start(); it.prepareForApp(); it.url("stremio:///detail/series/tt4574334")
    }
    @Test fun selectAndTVButtonSendOneDownAndUpWithRealHoldDuration() = peer("buttons") {
        it.verify(credentials()); it.start()
        it.key(6); it.key(6, true); it.key(7); it.key(7, true)
    }
    @Test fun changedIdentityIsRejectedBeforeSendingClientProof() = peer("identity") {
        assertThrows(IllegalArgumentException::class.java) { it.verify(credentials()) }
    }
    @Test fun incorrectSignatureIsRejectedBeforeSendingClientProof() = peer("signature") {
        assertThrows(IllegalArgumentException::class.java) { it.verify(credentials()) }
    }
}
