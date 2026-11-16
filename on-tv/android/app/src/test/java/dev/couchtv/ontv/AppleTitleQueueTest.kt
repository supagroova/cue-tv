package dev.couchtv.ontv

import org.junit.Assert.*
import org.junit.Test

class AppleTitleQueueTest {
    @Test fun nativeAcknowledgementCompletesOnlyTheCurrentTitle() {
        val queue = AppleTitleQueue()
        val request = queue.enqueue("series", "tt123")
        assertFalse(queue.confirm(request.serial + 1, true))
        assertTrue(queue.confirm(request.serial, true))
        assertTrue(request.await(100))
        assertNull(queue.current())
        assertFalse(queue.confirm(request.serial, true))
    }
    @Test fun newerTitleCancelsAnOlderUnopenedTitle() {
        val queue = AppleTitleQueue()
        val first = queue.enqueue("series", "tt123")
        val second = queue.enqueue("movie", "tt456")
        assertFalse(first.await(100))
        assertFalse(queue.confirm(first.serial, true))
        assertEquals(second.serial, queue.current()?.serial)
        assertTrue(queue.confirm(second.serial, true))
        assertTrue(second.await(100))
    }
    @Test fun homeCancelsPendingPlayback() {
        val queue = AppleTitleQueue()
        val request = queue.enqueue("series", "tt123")
        queue.cancel()
        assertNull(queue.current()); assertFalse(request.await(100))
    }
    @Test fun staleTitleExpiresAndCannotBeReplayedAfterForegrounding() {
        var now = 100L
        val queue = AppleTitleQueue({ now }, 1000)
        val request = queue.enqueue("series", "tt123")
        now = 1101
        assertNull(queue.current()); assertFalse(queue.confirm(request.serial, true)); assertFalse(request.await(100))
    }
    @Test fun cleanupOfAnOlderRequestDoesNotCancelTheNewerOne() {
        val queue = AppleTitleQueue()
        val first = queue.enqueue("series", "tt123")
        val second = queue.enqueue("movie", "tt456")
        queue.clear(first.serial)
        assertEquals(second.serial, queue.current()?.serial)
        assertTrue(queue.confirm(second.serial, false)); assertFalse(second.await(100))
    }
}
