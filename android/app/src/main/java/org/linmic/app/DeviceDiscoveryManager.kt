package org.linmic.app

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.net.wifi.WifiManager
import android.os.Handler
import android.os.Looper

@Suppress("DEPRECATION")
class DeviceDiscoveryManager(context: Context, private val changed: (List<NsdServiceInfo>) -> Unit) {
    private val manager = context.getSystemService(NsdManager::class.java)
    private val lock = (context.applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager).createMulticastLock("LinMic:discovery").apply { setReferenceCounted(false) }
    private val handler = Handler(Looper.getMainLooper())
    private val computers = linkedMapOf<String,NsdServiceInfo>()
    private var started = false
    private val queue = java.util.ArrayDeque<NsdServiceInfo>()
    private var resolving = false
    private fun resolveNext() {
        if (!started || resolving || queue.isEmpty()) return
        resolving = true
        val next = queue.removeFirst()
        manager.resolveService(next, object : NsdManager.ResolveListener {
            override fun onResolveFailed(info: NsdServiceInfo, code: Int) { handler.post { resolving=false; resolveNext() } }
            override fun onServiceResolved(info: NsdServiceInfo) { handler.post { resolving=false; if (started) { computers[info.serviceName]=info; changed(computers.values.toList()); resolveNext() } } }
        })
    }
    private val listener = object : NsdManager.DiscoveryListener {
        override fun onDiscoveryStarted(type: String) {}
        override fun onDiscoveryStopped(type: String) {}
        override fun onStartDiscoveryFailed(type: String, code: Int) { stop() }
        override fun onStopDiscoveryFailed(type: String, code: Int) {}
        override fun onServiceFound(info: NsdServiceInfo) { handler.post { if (started) { queue.add(info); resolveNext() } } }
        override fun onServiceLost(info: NsdServiceInfo) { handler.post { computers.remove(info.serviceName); changed(computers.values.toList()) } }
    }
    fun start() { if (started) return; started=true; lock.acquire(); runCatching { manager.discoverServices("_linmic._tcp.", NsdManager.PROTOCOL_DNS_SD, listener) }.onFailure { stop() } }
    fun stop() { if (!started) return; started=false; runCatching { manager.stopServiceDiscovery(listener) }; if(lock.isHeld)lock.release(); queue.clear(); computers.clear() }
}
