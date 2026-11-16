package dev.couchtv.ontv

import org.junit.Assert.*
import org.junit.Test

class CompanionWireTest {
    @Test fun pythonOPACKPacket() {
        // bridge/appletv.py opack_encode({_i:_launchApp,_x:42,_t:2,_c:{_bundleID:com.test.app}})
        val packet = CompanionWire.unhex("e4425f694a5f6c61756e6368417070425f78302a425f740a425f63e1495f62756e646c6549444c636f6d2e746573742e617070")
        val value = mapOf("_i" to "_launchApp", "_x" to 42, "_t" to 2, "_c" to mapOf("_bundleID" to "com.test.app"))
        assertArrayEquals(packet, CompanionWire.encode(value))
        val decoded = CompanionWire.decode(packet) as Map<*, *>
        assertEquals("_launchApp", decoded["_i"]); assertEquals(42L, decoded["_x"])
    }
    @Test fun containersAndReferences() {
        val packet = CompanionWire.unhex("e242616243616263a0a1")
        // Duplicate map keys are rejected even if they arrive through an object reference.
        assertThrows(IllegalArgumentException::class.java) { CompanionWire.decode(packet) }
        val list = List(18) { "title-$it" }
        assertEquals(list, CompanionWire.decode(CompanionWire.encode(list)))
    }
    @Test fun truncatedDataAndExcessDepthAreRejected() {
        for (hex in listOf("e1", "61ff", "a0", "dfff")) assertThrows(Exception::class.java) { CompanionWire.decode(CompanionWire.unhex(hex)) }
        assertThrows(Exception::class.java) { CompanionWire.decode(ByteArray(35) { 0xd1.toByte() } + byteArrayOf(4)) }
    }
    @Test fun tlvFragmentationAndTruncation() {
        val raw = ByteArray(600) { it.toByte() }
        assertArrayEquals(raw, CompanionWire.tlvRead(CompanionWire.tlv(3 to raw)).getValue(3))
        assertThrows(IllegalArgumentException::class.java) { CompanionWire.tlvRead(byteArrayOf(3, 4, 1)) }
    }
    @Test fun authenticatedHeaderAndCounterCannotBeChanged() {
        val key = ByteArray(32) { it.toByte() }; val header = byteArrayOf(8, 0, 0, 20); val clear = byteArrayOf(1, 2, 3, 4)
        val sealed = CompanionWire.crypt(true, key, CompanionWire.counter(0), clear, header)
        assertArrayEquals(clear, CompanionWire.crypt(false, key, CompanionWire.counter(0), sealed, header))
        assertThrows(Exception::class.java) { CompanionWire.crypt(false, key, CompanionWire.counter(1), sealed, header) }
        assertThrows(Exception::class.java) { CompanionWire.crypt(false, key, CompanionWire.counter(0), sealed, byteArrayOf(7, 0, 0, 20)) }
    }
}
