package dev.couchtv.ontv

import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.TimeoutException
import java.util.concurrent.atomic.AtomicReference

/** A single, short-lived handoff to our foreground tvOS app. Opening a title through the native
 * app avoids the external-URL confirmation raised by Companion's URL launcher. */
internal class AppleTitleQueue(private val clockMs: () -> Long = { System.nanoTime() / 1_000_000 }, private val ttlMs: Long = 15_000) {
    class Request(val serial: Long, val type: String, val id: String, val service: String, internal val created: Long) {
        private val result = AtomicReference<Boolean?>(null)
        private val ready = CountDownLatch(1)
        internal val done: Boolean get() = result.get() != null
        internal fun finish(opened: Boolean): Boolean {
            if (!result.compareAndSet(null, opened)) return false
            ready.countDown(); return true
        }
        fun await(timeoutMs: Long): Boolean {
            if (!ready.await(timeoutMs, TimeUnit.MILLISECONDS)) throw TimeoutException()
            return result.get() == true
        }
    }
    private var sequence = System.currentTimeMillis()
    private var pending: Request? = null
    @Synchronized fun enqueue(type: String, id: String, service: String = "stremio"): Request {
        cancel()
        return Request(++sequence, type, id, service, clockMs()).also { pending = it }
    }
    @Synchronized fun current(): Request? {
        val request = pending ?: return null
        if (clockMs() - request.created > ttlMs) { cancel(); return null }
        return request.takeUnless { it.done }
    }
    @Synchronized fun confirm(serial: Long, opened: Boolean): Boolean {
        val request = current()?.takeIf { it.serial == serial } ?: return false
        return request.finish(opened)
    }
    @Synchronized fun cancel() { pending?.finish(false); pending = null }
    @Synchronized fun clear(serial: Long) { if (pending?.serial == serial) cancel() }
}
