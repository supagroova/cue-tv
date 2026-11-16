package dev.couchtv.ontv

import java.io.ByteArrayOutputStream
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.security.SecureRandom
import org.bouncycastle.crypto.digests.SHA512Digest
import org.bouncycastle.crypto.generators.HKDFBytesGenerator
import org.bouncycastle.crypto.modes.ChaCha20Poly1305
import org.bouncycastle.crypto.params.AEADParameters
import org.bouncycastle.crypto.params.HKDFParameters
import org.bouncycastle.crypto.params.KeyParameter

/** Companion's bounded binary containers. Ported from this project's bridge/appletv.py. */
internal object CompanionWire {
    const val MAX = 1 shl 20
    fun encode(value: Any?): ByteArray {
        val out = ByteArrayOutputStream()
        fun le(n: Long, size: Int) { repeat(size) { out.write((n ushr (it * 8)).toInt() and 255) } }
        fun sized(bytes: ByteArray, short: Int, long: Int) {
            require(bytes.size <= MAX)
            if (bytes.size <= 32) out.write(short + bytes.size) else {
                val size = when { bytes.size < 256 -> 1; bytes.size < 65536 -> 2; else -> 3 }
                out.write(long + size - 1); le(bytes.size.toLong(), size)
            }
            out.write(bytes)
        }
        fun put(v: Any?, depth: Int) {
            require(depth < 32)
            when (v) {
                null -> out.write(4)
                is Boolean -> out.write(if (v) 1 else 2)
                is Number -> {
                    val n = v.toLong(); require(n >= 0)
                    if (n < 40) out.write(8 + n.toInt()) else {
                        val size = when { n < 256 -> 1; n < 65536 -> 2; n < 4294967296L -> 4; else -> 8 }
                        out.write(0x30 + Integer.numberOfTrailingZeros(size)); le(n, size)
                    }
                }
                is String -> sized(v.toByteArray(Charsets.UTF_8), 0x40, 0x61)
                is ByteArray -> sized(v, 0x70, 0x91)
                is List<*> -> {
                    out.write(0xd0 + minOf(v.size, 15)); v.forEach { put(it, depth + 1) }; if (v.size >= 15) out.write(3)
                }
                is Map<*, *> -> {
                    out.write(0xe0 + minOf(v.size, 15)); v.forEach { (k, x) -> put(k, depth + 1); put(x, depth + 1) }; if (v.size >= 15) out.write(3)
                }
                else -> error("Unsupported Companion value")
            }
            require(out.size() <= MAX)
        }
        put(value, 0); return out.toByteArray()
    }

    fun decode(data: ByteArray): Any? {
        require(data.size <= MAX)
        var pos = 0
        val objects = ArrayList<Any?>()
        fun bytes(n: Int): ByteArray {
            require(n >= 0 && n <= data.size - pos)
            return data.copyOfRange(pos, pos + n).also { pos += n }
        }
        fun byte() = bytes(1)[0].toInt() and 255
        fun le(n: Int): Long = bytes(n).foldIndexed(0L) { i, a, b -> a or ((b.toLong() and 255) shl (i * 8)) }
        fun sized(tag: Int, short: Int, long: Int): ByteArray {
            val n = if (tag < long) (tag - short).toLong() else le(tag - long + 1)
            require(n in 0..MAX.toLong()); return bytes(n.toInt())
        }
        fun take(depth: Int): Any? {
            require(depth < 32)
            val tag = byte()
            return when (tag) {
                1 -> true; 2 -> false; 4 -> null; 5 -> bytes(16)
                in 8..0x2f -> (tag - 8).toLong()
                in 0x30..0x33 -> le(1 shl (tag - 0x30)).also { objects.add(it) }
                0x35 -> ByteBuffer.wrap(bytes(4)).order(ByteOrder.LITTLE_ENDIAN).float.also { objects.add(it) }
                0x36 -> ByteBuffer.wrap(bytes(8)).order(ByteOrder.LITTLE_ENDIAN).double.also { objects.add(it) }
                in 0x40..0x64 -> sized(tag, 0x40, 0x61).toString(Charsets.UTF_8).also { objects.add(it) }
                in 0x70..0x94 -> sized(tag, 0x70, 0x91).also { objects.add(it) }
                in 0xa0..0xc0 -> objects.getOrNull(tag - 0xa0) ?: error("Invalid Companion reference")
                in 0xc1..0xc4 -> le(tag - 0xc0)
                in 0xd0..0xdf -> {
                    val list = ArrayList<Any?>(); val count = tag - 0xd0
                    while (if (count == 15) { require(pos < data.size); data[pos].toInt() != 3 } else list.size < count) list.add(take(depth + 1))
                    if (count == 15) byte(); list
                }
                in 0xe0..0xef -> {
                    val map = LinkedHashMap<String, Any?>(); val count = tag - 0xe0; var entries = 0
                    while (if (count == 15) { require(pos < data.size); data[pos].toInt() != 3 } else entries < count) {
                        val key = take(depth + 1) as? String ?: error("Invalid Companion key")
                        require(!map.containsKey(key)); map[key] = take(depth + 1); entries++
                    }
                    if (count == 15) byte(); map
                }
                else -> error("Invalid Companion tag")
            }
        }
        return take(0).also { require(pos == data.size) }
    }

    fun tlv(vararg fields: Pair<Int, ByteArray>): ByteArray = ByteArrayOutputStream().apply {
        for ((tag, raw) in fields) {
            if (raw.isEmpty()) { write(tag); write(0) }
            var at = 0
            while (at < raw.size) { val n = minOf(255, raw.size - at); write(tag); write(n); write(raw, at, n); at += n }
        }
    }.toByteArray()

    fun tlvRead(raw: ByteArray): Map<Int, ByteArray> {
        val out = LinkedHashMap<Int, ByteArray>(); var pos = 0; var last = -1
        while (pos < raw.size) {
            require(raw.size - pos >= 2)
            val tag = raw[pos++].toInt() and 255; val n = raw[pos++].toInt() and 255
            require(n <= raw.size - pos)
            val part = raw.copyOfRange(pos, pos + n); pos += n
            require(tag == last || !out.containsKey(tag))
            out[tag] = if (tag == last) out.getValue(tag) + part else part; last = tag
        }
        return out
    }

    fun hkdf(key: ByteArray, salt: String, info: String): ByteArray = ByteArray(32).also {
        HKDFBytesGenerator(SHA512Digest()).apply { init(HKDFParameters(key, salt.toByteArray(), info.toByteArray())); generateBytes(it, 0, it.size) }
    }
    fun crypt(encrypt: Boolean, key: ByteArray, nonce: ByteArray, payload: ByteArray, aad: ByteArray = byteArrayOf()): ByteArray {
        require(key.size == 32 && nonce.size <= 12)
        val iv = ByteArray(12 - nonce.size) + nonce
        val cipher = ChaCha20Poly1305().apply { init(encrypt, AEADParameters(KeyParameter(key), 128, iv, aad)) }
        val out = ByteArray(cipher.getOutputSize(payload.size))
        val n = cipher.processBytes(payload, 0, payload.size, out, 0)
        return out.copyOf(n + cipher.doFinal(out, n))
    }
    fun counter(n: Long): ByteArray = ByteArray(12).also { b -> repeat(8) { b[it] = (n ushr (it * 8)).toByte() } }
    fun random(size: Int): ByteArray = ByteArray(size).also { SecureRandom().nextBytes(it) }
    fun hex(raw: ByteArray) = raw.joinToString("") { "%02x".format(it.toInt() and 255) }
    fun unhex(s: String): ByteArray { require(s.length % 2 == 0 && s.matches(Regex("[0-9a-fA-F]+"))); return s.chunked(2).map { it.toInt(16).toByte() }.toByteArray() }
}
