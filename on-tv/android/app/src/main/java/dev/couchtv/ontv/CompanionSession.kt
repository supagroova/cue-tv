package dev.couchtv.ontv

import java.io.Closeable
import java.io.DataInputStream
import java.net.InetSocketAddress
import java.net.Socket
import java.security.MessageDigest
import org.bouncycastle.crypto.params.Ed25519PrivateKeyParameters
import org.bouncycastle.crypto.params.Ed25519PublicKeyParameters
import org.bouncycastle.crypto.params.X25519PrivateKeyParameters
import org.bouncycastle.crypto.params.X25519PublicKeyParameters
import org.bouncycastle.crypto.signers.Ed25519Signer

/** A short, authenticated Companion connection. Pair setup is a one-off Mac provisioning step;
 * subsequent controls run entirely on Android TV, with the pinned Apple TV identity. */
internal class CompanionSession(host: String, port: Int) : Closeable {
    data class Credentials(val clientId: String, val privateKey: ByteArray, val serverId: String, val serverKey: ByteArray)
    private val socket = Socket()
    private val input: DataInputStream
    private var outKey: ByteArray? = null
    private var inKey: ByteArray? = null
    private var sent = 0L
    private var received = 0L
    private var xid = 0L

    init {
        try { socket.connect(InetSocketAddress(host, port), 3500); socket.soTimeout = 4500; socket.tcpNoDelay = true; input = DataInputStream(socket.getInputStream()) }
        catch (e: Exception) { socket.close(); throw e }
    }
    override fun close() { socket.close(); outKey?.fill(0); inKey?.fill(0) }
    private fun send(type: Int, bytes: ByteArray) {
        val size = bytes.size + if (outKey != null) 16 else 0
        require(size <= CompanionWire.MAX)
        val header = byteArrayOf(type.toByte(), (size ushr 16).toByte(), (size ushr 8).toByte(), size.toByte())
        val payload = outKey?.let { CompanionWire.crypt(true, it, CompanionWire.counter(sent++), bytes, header) } ?: bytes
        socket.getOutputStream().apply { write(header); write(payload); flush() }
    }
    private fun receive(expected: Int): Map<String, Any?> {
        val header = ByteArray(4).also { input.readFully(it) }
        require(header[0].toInt() and 255 == expected) { "Unexpected Companion frame" }
        val size = ((header[1].toInt() and 255) shl 16) or ((header[2].toInt() and 255) shl 8) or (header[3].toInt() and 255)
        require(size in 1..CompanionWire.MAX)
        val payload = ByteArray(size).also { input.readFully(it) }
        val clear = inKey?.let { CompanionWire.crypt(false, it, CompanionWire.counter(received++), payload, header) } ?: payload
        @Suppress("UNCHECKED_CAST")
        return CompanionWire.decode(clear) as? Map<String, Any?> ?: error("Invalid Companion message")
    }
    private fun verifyStep(type: Int, sequence: Int, vararg values: Pair<Int, ByteArray>): Map<Int, ByteArray> {
        val fields = linkedMapOf<String, Any?>("_pd" to CompanionWire.tlv(6 to byteArrayOf(sequence.toByte()), *values))
        if (sequence == 1) fields["_auTy"] = 4
        send(type, CompanionWire.encode(fields))
        val reply = receive(if (type == 5) 6 else type)
        val tlv = CompanionWire.tlvRead(reply["_pd"] as? ByteArray ?: error("Missing pairing response"))
        require(7 !in tlv && tlv[6]?.contentEquals(byteArrayOf((sequence + 1).toByte())) == true) { "Apple TV pairing needs to be renewed" }
        return tlv
    }
    fun verify(cred: Credentials) {
        require(cred.privateKey.size == 32 && cred.serverKey.size == 32)
        val privateKey = X25519PrivateKeyParameters(CompanionWire.random(32), 0)
        val publicKey = privateKey.generatePublicKey().encoded
        val reply = verifyStep(5, 1, 3 to publicKey)
        val serverPublic = reply[3]?.takeIf { it.size == 32 } ?: error("Invalid Apple TV public key")
        val shared = ByteArray(32)
        privateKey.generateSecret(X25519PublicKeyParameters(serverPublic, 0), shared, 0)
        val key = CompanionWire.hkdf(shared, "Pair-Verify-Encrypt-Salt", "Pair-Verify-Encrypt-Info")
        val inner = CompanionWire.tlvRead(CompanionWire.crypt(false, key, "PV-Msg02".toByteArray(), reply.getValue(5)))
        val identifier = inner[1] ?: error("Missing Apple TV identity")
        require(MessageDigest.isEqual(identifier, cred.serverId.toByteArray(Charsets.UTF_8))) { "Apple TV identity changed" }
        val message = serverPublic + identifier + publicKey
        val signer = Ed25519Signer().apply { init(false, Ed25519PublicKeyParameters(cred.serverKey, 0)); update(message, 0, message.size) }
        require(signer.verifySignature(inner.getValue(10))) { "Apple TV signature did not match" }
        val client = cred.clientId.toByteArray(Charsets.UTF_8)
        val proof = publicKey + client + serverPublic
        val signature = Ed25519Signer().run { init(true, Ed25519PrivateKeyParameters(cred.privateKey, 0)); update(proof, 0, proof.size); generateSignature() }
        val sub = CompanionWire.tlv(1 to client, 10 to signature)
        verifyStep(6, 3, 5 to CompanionWire.crypt(true, key, "PV-Msg03".toByteArray(), sub))
        outKey = CompanionWire.hkdf(shared, "", "ClientEncrypt-main")
        inKey = CompanionWire.hkdf(shared, "", "ServerEncrypt-main")
        key.fill(0); shared.fill(0)
    }
    fun ask(name: String, content: Map<String, Any?> = emptyMap()): Map<String, Any?> {
        xid = (xid + 1) and 0xffff
        send(8, CompanionWire.encode(mapOf("_i" to name, "_x" to xid, "_t" to 2, "_c" to content)))
        repeat(20) {
            val reply = receive(8)
            if ((reply["_x"] as? Number)?.toLong() == xid && (reply["_t"] as? Number)?.toInt() in setOf(3, 4)) {
                if ((reply["_t"] as Number).toInt() == 4 || reply.containsKey("_em")) error("Apple TV refused this action")
                @Suppress("UNCHECKED_CAST")
                return reply["_c"] as? Map<String, Any?> ?: emptyMap()
            }
        }
        error("No reply from Apple TV")
    }
    fun start() {
        ask("_systemInfo", mapOf("_bf" to 0, "_cf" to 512, "_clFl" to 128, "_i" to "couch-tv", "_idsID" to CompanionWire.hex(CompanionWire.random(8)),
            "_pubID" to "couch-tv", "_sf" to 256, "_sv" to "170.18", "model" to "iPhone10,6", "name" to "Cue TV"))
        val sid = CompanionWire.random(4).foldIndexed(0L) { i, n, b -> n or ((b.toLong() and 255) shl (i * 8)) }
        ask("_sessionStart", mapOf("_srvT" to "com.apple.tvremoteservices", "_sid" to sid))
        // Older tvOS versions do not recognise the extra TV Remote session request.
        runCatching { ask("TVRCSessionStart", mapOf("ProtocolVersionKey" to "1.2")) }
    }
    fun key(code: Int, hold: Boolean = false) {
        ask("_hidC", mapOf("_hBtS" to 1, "_hidC" to code))
        try { Thread.sleep(if (hold) 1000 else 50) }
        finally { ask("_hidC", mapOf("_hBtS" to 2, "_hidC" to code)) }
    }
    fun prepareForApp(wake: Boolean = false) {
        if (wake) key(13)
        // Wake does not dismiss an aerial screensaver. Back does; Select shows its location.
        var state = runCatching { (ask("FetchAttentionState")["state"] as? Number)?.toInt() }.getOrNull()
        if (state == 1) {
            key(13)
            state = runCatching { (ask("FetchAttentionState")["state"] as? Number)?.toInt() }.getOrNull()
        }
        if (state == 2) key(5)
    }
    fun app(bundle: String) { ask("_launchApp", mapOf("_bundleID" to bundle)) }
    fun url(uri: String) { ask("_launchApp", mapOf("_urlS" to uri)) }
}
