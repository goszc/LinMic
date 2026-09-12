package org.linmic.app

import android.app.*
import android.content.Intent
import android.content.pm.ServiceInfo
import android.media.audiofx.AutomaticGainControl
import android.media.audiofx.NoiseSuppressor
import android.os.*
import org.json.JSONObject
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong

sealed class StreamState {
    data object Idle : StreamState()
    data object Connecting : StreamState()
    data object Streaming : StreamState()
    data object Muted : StreamState()
    data class Reconnecting(val attempt: Int) : StreamState()
    data class Error(val message: String) : StreamState()
}
object StreamModel {
    @Volatile var state: StreamState = StreamState.Idle
    @Volatile var meter = FloatArray(144)
    @Volatile var server = JSONObject()
    @Volatile var muted = false
    @Volatile var activityVisible = false
    val native = NativeAudioController()
}
class StreamingForegroundService : Service() {
    override fun attachBaseContext(base: android.content.Context) { super.attachBaseContext(AppLocales.wrap(base)) }
    private val wanted = AtomicBoolean(false)
    private val muteRequest = AtomicBoolean(false)
    private val revision = AtomicLong(0)
    @Volatile private var control: ControlClient? = null
    private var worker: Thread? = null
    private var powerLocks: StreamPowerLocks? = null
    private lateinit var settings: SettingsRepository
    private var ns: NoiseSuppressor? = null
    private var agc: AutomaticGainControl? = null
    override fun onCreate() {
        super.onCreate(); settings = SettingsRepository(this)
        getSystemService(NotificationManager::class.java).createNotificationChannel(NotificationChannel("stream", getString(R.string.notification_channel), NotificationManager.IMPORTANCE_LOW))
    }
    override fun onBind(intent: Intent?) = null
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            "stop" -> { stopSelf(); return START_NOT_STICKY }
            "mute" -> {
                StreamModel.muted = !StreamModel.muted
                StreamModel.native.mute(StreamModel.muted)
                muteRequest.set(true)
                showNotification()
                return START_NOT_STICKY
            }
            "gain" -> { StreamModel.native.gain(settings.gain); return START_NOT_STICKY }
        }
        if (wanted.getAndSet(true)) return START_NOT_STICKY
        StreamModel.state = StreamState.Connecting
        val notification = notification()
        if (Build.VERSION.SDK_INT >= 30) startForeground(1, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE) else startForeground(1, notification)
        powerLocks = StreamPowerLocks(this).also { it.refresh(settings.wifiLowLatency, StreamModel.activityVisible) }
        val code = intent?.getStringExtra("code") ?: ""
        val pin = intent?.getStringExtra("fingerprint") ?: settings.fingerprint(settings.host)
        worker = Thread({ streamLoop(pin, code) }, "linmic-control").also { it.start() }
        return START_NOT_STICKY
    }
    private fun streamLoop(pin: String, code: String) {
        val backoff = longArrayOf(250, 500, 1000, 2000, 3000, 5000)
        var attempt = 0
        while (wanted.get()) {
            val client = ControlClient(settings); control = client
            try {
                StreamModel.state = if (attempt == 0) StreamState.Connecting else StreamState.Reconnecting(attempt)
                powerLocks?.refresh(settings.wifiLowLatency, StreamModel.activityVisible)
                // Pairing persists a pin even if the connection drops before hello_ack.
                val currentPin = ReconnectPolicy.reconnectPin(settings.fingerprint(settings.host), pin)
                val ack = client.connect(settings.host, settings.port, currentPin, if (currentPin.isEmpty()) code else "")
                val start = client.request(JSONObject().put("type", "start_stream"))
                check(start.optString("type") == "start_stream_ack") { "PC rejected stream" }
                revision.set(start.optLong("revision"))
                StreamModel.muted = start.optBoolean("muted")
                val frame = ack.getInt("frame_samples")
                val key = ack.getString("audio_key").chunked(2).map { it.toInt(16).toByte() }.toByteArray()
                if (!wanted.get()) break
                StreamModel.native.start(ack.getString("_peer_host"), ack.getInt("udp_port"), if (frame == 240) 64000 else 48000, frame, ack.getLong("session_id"), key, settings.inputDevice, settings.gain, StreamModel.muted)
                key.fill(0)
                val initial = FloatArray(144); StreamModel.native.snapshot(initial)
                val session = initial[15].toInt()
                if (session > 0) {
                    if (settings.systemNs && NoiseSuppressor.isAvailable()) ns = NoiseSuppressor.create(session)?.apply { enabled = true }
                    if (settings.systemAgc && AutomaticGainControl.isAvailable()) agc = AutomaticGainControl.create(session)?.apply { enabled = true }
                }
                StreamModel.state = if (StreamModel.muted) StreamState.Muted else StreamState.Streaming
                showNotification(); attempt = 0
                var rtt = 0.0
                var heartbeat = 0L
                var highLossSince = 0L
                while (wanted.get()) {
                    val now = SystemClock.elapsedRealtime()
                    if (muteRequest.getAndSet(false)) {
                        val response = client.request(JSONObject().put("type", if (StreamModel.muted) "mute" else "unmute").put("revision", revision.incrementAndGet()))
                        revision.set(response.optLong("revision", revision.get()))
                        StreamModel.muted = response.optBoolean("muted", StreamModel.muted)
                    }
                    if (now - heartbeat >= 100) {
                        val startTime = SystemClock.elapsedRealtimeNanos()
                        val response = client.request(JSONObject().put("type", "ping").put("t0", startTime/1000).put("rtt_ms", rtt))
                        check(response.optString("type") == "pong") { "PC disconnected" }
                        val endTime = SystemClock.elapsedRealtimeNanos()
                        val estimate = Timing.estimate(startTime/1000,response.optLong("t1"),response.optLong("t2"),endTime/1000)
                        val measured = estimate.rttMs
                        response.put("clock_offset_us",estimate.clockOffsetUs)
                        rtt = if (rtt == 0.0) measured else 0.875*rtt+0.125*measured
                        response.put("rtt_ms", rtt)
                        response.put("estimated_latency_ms",response.optDouble("estimated_latency_ms",0.0)+StreamModel.meter[5])
                        check(response.optString("state")!="RECONNECTING") { "PC audio timeout" }
                        StreamModel.server = response
                        if (!muteRequest.get() && response.optLong("revision") >= revision.get()) {
                            revision.set(response.optLong("revision")); StreamModel.muted = response.optBoolean("muted")
                        }
                        StreamModel.native.mute(StreamModel.muted)
                        StreamModel.state = if (StreamModel.muted) StreamState.Muted else StreamState.Streaming
                        val loss = response.optDouble("loss_percent", 0.0)
                        if (loss > 1.0) { if (highLossSince == 0L) highLossSince = now } else highLossSince = 0L
                        StreamModel.native.loss(if (highLossSince != 0L && now-highLossSince > 3000) loss.toInt().coerceIn(1,20) else 0)
                        heartbeat = now
                        powerLocks?.refresh(settings.wifiLowLatency, StreamModel.activityVisible)
                    }
                    val stats = FloatArray(144); StreamModel.native.snapshot(stats); StreamModel.meter = stats
                    check(stats[6] == 0f) { "Audio device changed (${stats[6].toInt()})" }
                    Thread.sleep(33)
                }
            } catch (e: Exception) {
                if (!wanted.get()) break
                attempt++
                StreamModel.state = StreamState.Reconnecting(attempt)
                // Authentication failures require a fresh explicit pairing action.
                if (!ReconnectPolicy.canRetry(e, settings.reconnect)) {
                    StreamModel.state = StreamState.Error(e.message ?: "Connection failed")
                    wanted.set(false)
                } else {
                    android.util.Log.w("LinMic", "Transport interrupted (${e.javaClass.simpleName}); reconnecting")
                }
            } finally {
                StreamModel.native.stop(); ns?.release(); ns=null; agc?.release(); agc=null
                client.close(); if (control === client) control = null
                StreamModel.meter = FloatArray(144)
            }
            if (wanted.get()) {
                try { Thread.sleep(backoff[(attempt-1).coerceIn(0,backoff.lastIndex)]) } catch (_: InterruptedException) { break }
            }
        }
        Handler(mainLooper).post { stopSelf() }
    }
    private fun notification(): Notification {
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        fun action(name: String, request: Int) = PendingIntent.getService(this, request, Intent(this, javaClass).setAction(name), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        return Notification.Builder(this, "stream").setSmallIcon(R.drawable.ic_mic).setContentTitle(getString(R.string.app_name))
            .setContentText(getString(if (StreamModel.muted) R.string.muted else R.string.streaming_to, settings.host)).setContentIntent(open).setOngoing(true)
            .addAction(Notification.Action.Builder(null, getString(if (StreamModel.muted) R.string.unmute else R.string.mute), action("mute",1)).build())
            .addAction(Notification.Action.Builder(null, getString(R.string.disconnect), action("stop",2)).build()).build()
    }
    private fun showNotification() { getSystemService(NotificationManager::class.java).notify(1, notification()) }
    override fun onDestroy() {
        wanted.set(false); control?.close(); worker?.interrupt(); StreamModel.native.stop()
        powerLocks?.close()
        if (StreamModel.state !is StreamState.Error) StreamModel.state = StreamState.Idle
        stopForeground(STOP_FOREGROUND_REMOVE)
        super.onDestroy()
    }
}
