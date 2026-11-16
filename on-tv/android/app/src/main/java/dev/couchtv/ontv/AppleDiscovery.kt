package dev.couchtv.ontv

import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.content.Context
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.atomic.AtomicBoolean

/** Companion's port may change after a restart. Discovery sends no control or pairing request;
 * every resolved endpoint still has to prove the pinned identity before a command is sent. */
internal object AppleDiscovery {
    data class Endpoint(val host: String, val port: Int)
    private data class Found(val name: String, val endpoint: Endpoint)
    private val found = ConcurrentHashMap<String, Found>()
    private val waiting = ConcurrentLinkedQueue<NsdServiceInfo>()
    private val resolving = AtomicBoolean(false)
    @Volatile private var started = false
    @Synchronized fun start() {
        if (started) return
        val manager = Store.context().getSystemService(Context.NSD_SERVICE) as? NsdManager ?: return
        fun resolveNext() {
            if (!resolving.compareAndSet(false, true)) return
            val service = waiting.poll()
            if (service == null) { resolving.set(false); return }
            @Suppress("DEPRECATION")
            manager.resolveService(service, object : NsdManager.ResolveListener {
                override fun onResolveFailed(info: NsdServiceInfo, error: Int) { resolving.set(false); resolveNext() }
                override fun onServiceResolved(info: NsdServiceInfo) {
                    val host = info.host?.hostAddress
                    if (host != null && ':' !in host && info.port in 1..65535 && found.size < 64)
                        found[info.serviceName] = Found(info.serviceName, Endpoint(host, info.port))
                    resolving.set(false); resolveNext()
                }
            })
        }
        val listener = object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(type: String) {}
            override fun onDiscoveryStopped(type: String) { started = false }
            override fun onStartDiscoveryFailed(type: String, error: Int) { started = false }
            override fun onStopDiscoveryFailed(type: String, error: Int) {}
            override fun onServiceLost(service: NsdServiceInfo) { found.remove(service.serviceName) }
            override fun onServiceFound(service: NsdServiceInfo) {
                if (waiting.size < 64) { waiting.add(service); resolveNext() }
            }
        }
        try { started = true; manager.discoverServices("_companion-link._tcp.", NsdManager.PROTOCOL_DNS_SD, listener) }
        catch (e: Exception) { started = false }
    }
    fun endpoint(name: String, host: String, port: Int): Endpoint {
        start()
        return found.values.firstOrNull { it.name.equals(name, ignoreCase = true) }?.endpoint
            ?: found.values.firstOrNull { it.endpoint.host == host }?.endpoint ?: Endpoint(host, port)
    }
}
