package org.linmic.app

import android.content.Context
import android.net.wifi.WifiManager
import android.os.Build
import android.os.PowerManager
import android.os.SystemClock

/** Owned by the microphone service, including its reconnect intervals, never by the Activity. */
@Suppress("DEPRECATION")
class StreamPowerLocks(context: Context) : AutoCloseable {
    private val power = context.getSystemService(PowerManager::class.java)
    private val manager = context.applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
    private val cpu = power.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "LinMic:stream").apply { setReferenceCounted(false) }
    // Android 14 replaces HIGH_PERF with screen-on-only LOW_LATENCY. Do not pretend
    // a legacy lock bypasses Doze on newer versions.
    private val backgroundWifi = if (Build.VERSION.SDK_INT < 34)
        manager.createWifiLock(WifiManager.WIFI_MODE_FULL_HIGH_PERF, "LinMic:voice").apply { setReferenceCounted(false) } else null
    private val lowLatencyWifi = if (Build.VERSION.SDK_INT >= 29)
        manager.createWifiLock(WifiManager.WIFI_MODE_FULL_LOW_LATENCY, "LinMic:latency").apply { setReferenceCounted(false) } else null
    private var closed = false
    private var renewed = 0L

    @Synchronized fun refresh(lowLatency: Boolean, visible: Boolean) {
        if (closed) return
        val now = SystemClock.elapsedRealtime()
        if (!cpu.isHeld || now - renewed >= 60_000) {
            cpu.acquire(10 * 60_000L)
            renewed = now
        }
        // Acquire baseline protection before dropping the optional foreground lock.
        backgroundWifi?.let { if (!it.isHeld) it.acquire() }
        lowLatencyWifi?.let {
            if (lowLatency && visible && power.isInteractive) {
                if (!it.isHeld) it.acquire()
            } else if (it.isHeld) it.release()
        }
    }

    @Synchronized override fun close() {
        closed = true
        lowLatencyWifi?.let { if (it.isHeld) it.release() }
        backgroundWifi?.let { if (it.isHeld) it.release() }
        if (cpu.isHeld) cpu.release()
    }
}
