package org.linmic.app

import android.app.*
import android.content.Intent
import android.content.pm.ServiceInfo
import android.media.AudioManager
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
    @Volatile var health = CaptureMonitor.Health.STARTING
    @Volatile var meter = FloatArray(148)
    @Volatile var server = JSONObject()
    @Volatile var muted = false
    @Volatile var activityVisible = false
    @Volatile var systemSilenced = false
    @Volatile var repairs = 0
    @Volatile var startedAt = 0L
    @Volatile var owner = 0L
}
class StreamingForegroundService : Service() {
    override fun attachBaseContext(base: android.content.Context) { super.attachBaseContext(AppLocales.wrap(base)) }
    private val wanted = AtomicBoolean(false)
    private val muteRequest = AtomicBoolean(false)
    private val revision = AtomicLong(0)
    private val repairRequest = AtomicBoolean(false)
    private val owner = System.nanoTime()
    @Volatile private var control: ControlClient? = null
    private var worker: Thread? = null
    private var powerLocks: StreamPowerLocks? = null
    private lateinit var settings: SettingsRepository
    private lateinit var diagnostics: SessionDiagnostics
    private val main = Handler(Looper.getMainLooper())
    private var pendingStart: Intent? = null
    @Volatile private var destroyed = false
    @Volatile private var foreground = false

    override fun onCreate() {
        super.onCreate()
        synchronized(StreamModel) { StreamModel.owner = owner }
        settings = SettingsRepository(this)
        diagnostics = SessionDiagnostics(this)
        getSystemService(NotificationManager::class.java).createNotificationChannel(NotificationChannel("stream", getString(R.string.notification_channel), NotificationManager.IMPORTANCE_LOW))
        diagnostics.event("SERVICE_CREATED")
    }
    override fun onBind(intent: Intent?) = null
    private fun ownsModel() = !destroyed && StreamModel.owner == owner
    private fun publishMute(value: Boolean, force: Boolean = false) {
        synchronized(StreamModel) {
            if (!ownsModel() || (!force && muteRequest.get())) return
            StreamModel.muted = value
            NativeAudio.setPrivacyMute(value)
        }
    }
    private fun publish(state: StreamState) {
        if (!ownsModel()) return
        val changed = StreamModel.state != state
        StreamModel.state = state
        if (changed) {
            diagnostics.event("STATE_${state.javaClass.simpleName}")
            main.post { if (ownsModel() && foreground) showNotification() }
        }
    }
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            "stop" -> {
                settings.sessionRequested = false; wanted.set(false); pendingStart = null
                NativeAudio.setPrivacyMute(true)
                control?.close(); worker?.interrupt(); diagnostics.event("USER_STOP")
                stopSelf(); return START_NOT_STICKY
            }
            "mute" -> {
                if (wanted.get()) {
                    publishMute(!StreamModel.muted, true)
                    settings.sessionMuted = StreamModel.muted; muteRequest.set(true)
                    showNotification()
                } else stopSelf()
                return if (wanted.get()) START_STICKY else START_NOT_STICKY
            }
            "gain" -> return if (wanted.get()) START_STICKY else START_NOT_STICKY
            "repair" -> {
                if (wanted.get()) {
                    diagnostics.event("USER_REPAIR"); repairRequest.set(true); control?.close()
                    publish(StreamState.Reconnecting(1))
                    return START_STICKY
                }
                stopSelf(); return START_NOT_STICKY
            }
        }
        // Only a system restart of an explicitly requested, already authenticated session may resume.
        if (intent == null && (!settings.sessionRequested || !settings.reconnect || settings.fingerprint(settings.host).isEmpty())) {
            stopSelf(); return START_NOT_STICKY
        }
        if (worker?.isAlive == true) {
            if (!wanted.get()) pendingStart = intent ?: Intent()
            return START_STICKY
        }
        settings.sessionRequested = true
        wanted.set(true)
        publishMute(settings.sessionMuted, true)
        StreamModel.startedAt = SystemClock.elapsedRealtime()
        StreamModel.repairs = 0; StreamModel.health = CaptureMonitor.Health.STARTING
        publish(StreamState.Connecting)
        try {
            if (Build.VERSION.SDK_INT >= 30) startForeground(1, notification(), ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE) else startForeground(1, notification())
            foreground = true
            powerLocks = StreamPowerLocks(this).also { it.refresh(settings.wifiLowLatency, StreamModel.activityVisible) }
        } catch (e: RuntimeException) {
            diagnostics.event("FOREGROUND_DENIED_${e.javaClass.simpleName}")
            wanted.set(false); settings.sessionRequested = false
            publish(StreamState.Error(getString(R.string.open_to_resume)))
            stopSelf(); return START_NOT_STICKY
        }
        val code = intent?.getStringExtra("code") ?: ""
        val pin = intent?.getStringExtra("fingerprint") ?: settings.fingerprint(settings.host)
        worker = Thread({ streamLoop(pin, code) }, "linmic-control").also { it.start() }
        return START_STICKY
    }
    private fun streamLoop(pin: String, code: String) {
        // One worker owns its native controller. An older service cannot destroy a newer stream.
        val native = NativeAudioController()
        val monitor = CaptureMonitor()
        val audioManager = getSystemService(AudioManager::class.java)
        val backoff = longArrayOf(250, 500, 1000, 2000, 3000, 5000)
        var attempt = 0
        val host = settings.host; val port = settings.port
        while (wanted.get() && ownsModel()) {
            val client = ControlClient(settings); control = client
            var ns: NoiseSuppressor? = null
            var agc: AutomaticGainControl? = null
            try {
                publish(if (attempt == 0) StreamState.Connecting else StreamState.Reconnecting(attempt))
                powerLocks?.refresh(settings.wifiLowLatency, StreamModel.activityVisible)
                StreamModel.server = JSONObject()
                val currentPin = ReconnectPolicy.reconnectPin(settings.fingerprint(host), pin)
                val ack = client.connect(host, port, currentPin, if (currentPin.isEmpty()) code else "")
                val start = client.request(JSONObject().put("type", "start_stream"))
                check(start.optString("type") == "start_stream_ack") { "PC rejected stream" }
                revision.set(start.optLong("revision"))
                val desiredMute = StreamModel.muted || start.optBoolean("muted")
                publishMute(desiredMute)
                if (desiredMute != start.optBoolean("muted")) muteRequest.set(true)
                val frame = ack.getInt("frame_samples")
                val key = ack.getString("audio_key").chunked(2).map { it.toInt(16).toByte() }.toByteArray()
                if (!wanted.get() || !ownsModel()) { key.fill(0); break }
                try {
                    native.start(ack.getString("_peer_host"), ack.getInt("udp_port"), if (frame == 240) 64000 else 48000, frame, ack.getLong("session_id"), key, settings.inputDevice, settings.gain, StreamModel.muted, settings.compatible)
                } finally { key.fill(0) }
                val initial = FloatArray(148); native.snapshot(initial)
                val session = native.sessionId()
                if (session > 0) {
                    if (settings.systemNs && NoiseSuppressor.isAvailable()) ns = runCatching { NoiseSuppressor.create(session)?.apply { enabled = true } }.getOrNull()
                    if (settings.systemAgc && AutomaticGainControl.isAvailable()) agc = runCatching { AutomaticGainControl.create(session)?.apply { enabled = true } }.getOrNull()
                }
                diagnostics.event("CAPTURE_STARTED mode=${if(settings.compatible) "COMPATIBLE" else "FAST"} session=$session")
                val connectedAt = SystemClock.elapsedRealtime()
                monitor.begin(connectedAt)
                StreamModel.health = CaptureMonitor.Health.STARTING
                var rtt = 0.0; var heartbeat = 0L; var policyChecked = 0L; var highLossSince = 0L
                var previousHealth = CaptureMonitor.Health.STARTING
                var appliedGain = settings.gain
                repairRequest.set(false)
                while (wanted.get() && ownsModel()) {
                    val now = SystemClock.elapsedRealtime()
                    powerLocks?.refresh(settings.wifiLowLatency, StreamModel.activityVisible)
                    check(!repairRequest.get()) { "Requested recovery" }
                    if (muteRequest.getAndSet(false)) {
                        native.mute(StreamModel.muted)
                        val response = client.request(JSONObject().put("type", if (StreamModel.muted) "mute" else "unmute").put("revision", revision.incrementAndGet()))
                        revision.set(response.optLong("revision", revision.get()))
                        publishMute(response.optBoolean("muted", StreamModel.muted))
                    }
                    if (settings.gain != appliedGain) { appliedGain = settings.gain; native.gain(appliedGain) }
                    if (now - heartbeat >= 250) {
                        val startTime = SystemClock.elapsedRealtimeNanos()
                        val response = client.request(JSONObject().put("type", "ping").put("t0", startTime/1000).put("rtt_ms", rtt))
                        if (!wanted.get() || !ownsModel()) break
                        check(response.optString("type") == "pong") { "PC disconnected" }
                        val endTime = SystemClock.elapsedRealtimeNanos()
                        val estimate = Timing.estimate(startTime/1000,response.optLong("t1"),response.optLong("t2"),endTime/1000)
                        rtt = if (rtt == 0.0) estimate.rttMs else 0.875*rtt+0.125*estimate.rttMs
                        response.put("rtt_ms", rtt)
                        response.put("estimated_latency_ms",response.optDouble("estimated_latency_ms",0.0)+StreamModel.meter[5])
                        StreamModel.server = response
                        if (!muteRequest.get() && response.optLong("revision") >= revision.get()) {
                            revision.set(response.optLong("revision")); publishMute(response.optBoolean("muted"))
                        }
                        native.mute(StreamModel.muted)
                        if (settings.sessionMuted != StreamModel.muted) settings.sessionMuted = StreamModel.muted
                        val loss = response.optDouble("loss_percent", 0.0)
                        if (loss > 1.0) { if (highLossSince == 0L) highLossSince = now } else highLossSince = 0L
                        native.loss(if (highLossSince != 0L && now-highLossSince > 3000) loss.toInt().coerceIn(1,20) else 0)
                        heartbeat = now
                    }
                    val stats = FloatArray(148); native.snapshot(stats); StreamModel.meter = stats
                    check(stats[6] == 0f) { "Audio device error ${stats[6].toInt()}" }
                    if (Build.VERSION.SDK_INT >= 29 && now-policyChecked >= 1000) {
                        StreamModel.systemSilenced = runCatching { audioManager.activeRecordingConfigurations.firstOrNull { it.clientAudioSessionId == session }?.isClientSilenced ?: false }.getOrDefault(false)
                        policyChecked = now
                    }
                    val health = monitor.sample(now, stats[7], stats[2], StreamModel.server.optLong("session_packets"), stats[145], StreamModel.muted, StreamModel.systemSilenced)
                    StreamModel.health = health.health
                    if (health.health != previousHealth) {
                        if (health.health !in listOf(CaptureMonitor.Health.LIVE, CaptureMonitor.Health.QUIET)) diagnostics.event("HEALTH_${health.health}")
                        previousHealth = health.health
                        main.post { if (ownsModel() && foreground) showNotification() }
                    }
                    if (health.recover) {
                        StreamModel.repairs++
                        diagnostics.event("AUTO_REPAIR_${health.health}")
                        throw java.io.IOException("Capture recovery")
                    }
                    publish(if (StreamModel.muted) StreamState.Muted else StreamState.Streaming)
                    if (now-connectedAt > 10000) attempt = 0
                    Thread.sleep(50)
                }
            } catch (e: Exception) {
                if (!wanted.get() || !ownsModel()) break
                attempt++
                diagnostics.event("RETRY_${e.javaClass.simpleName} attempt=$attempt")
                publish(StreamState.Reconnecting(attempt))
                if (!ReconnectPolicy.canRetry(e, settings.reconnect)) {
                    publish(StreamState.Error(e.message ?: getString(R.string.error)))
                    wanted.set(false); settings.sessionRequested = false
                }
            } finally {
                native.stop(); ns?.release(); agc?.release(); client.close()
                if (control === client) control = null
                if (ownsModel()) { StreamModel.meter = FloatArray(148); StreamModel.systemSilenced = false }
            }
            if (wanted.get()) {
                try { Thread.sleep(backoff[(attempt-1).coerceIn(0,backoff.lastIndex)]) } catch (_: InterruptedException) { break }
            }
        }
        native.stop()
        powerLocks?.close()
        main.post {
            if (!ownsModel()) return@post
            worker = null
            val pending = pendingStart; pendingStart = null
            if (pending != null) onStartCommand(pending, 0, 0) else stopSelf()
        }
    }
    private fun notification(): Notification {
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        fun action(name: String, request: Int) = PendingIntent.getService(this, request, Intent(this, javaClass).setAction(name), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        val text = when(val state = StreamModel.state) {
            is StreamState.Reconnecting -> getString(R.string.reconnecting, state.attempt)
            StreamState.Connecting -> getString(R.string.connecting)
            is StreamState.Error -> getString(R.string.open_to_resume)
            else -> when(StreamModel.health) {
                CaptureMonitor.Health.SYSTEM_BLOCKED -> getString(R.string.system_blocked)
                CaptureMonitor.Health.DIGITAL_SILENCE -> getString(R.string.digital_silence)
                CaptureMonitor.Health.CAPTURE_STALLED, CaptureMonitor.Health.SEND_STALLED, CaptureMonitor.Health.DELIVERY_STALLED -> getString(R.string.audio_stalled)
                else -> getString(if (StreamModel.muted) R.string.muted else R.string.streaming_to, settings.host)
            }
        }
        return Notification.Builder(this, "stream").setSmallIcon(R.drawable.ic_mic).setContentTitle(getString(R.string.app_name))
            .setContentText(text).setContentIntent(open).setOngoing(true).setOnlyAlertOnce(true)
            .addAction(Notification.Action.Builder(null, getString(if (StreamModel.muted) R.string.unmute else R.string.mute), action("mute",1)).build())
            .addAction(Notification.Action.Builder(null, getString(R.string.repair), action("repair",3)).build())
            .addAction(Notification.Action.Builder(null, getString(R.string.disconnect), action("stop",2)).build()).build()
    }
    private fun showNotification() { getSystemService(NotificationManager::class.java).notify(1, notification()) }
    override fun onDestroy() {
        diagnostics.event("SERVICE_DESTROYED")
        destroyed = true; wanted.set(false); control?.close(); worker?.interrupt(); powerLocks?.close()
        if (StreamModel.owner == owner) {
            StreamModel.meter = FloatArray(148)
            if (StreamModel.state !is StreamState.Error) StreamModel.state = StreamState.Idle
        }
        stopForeground(STOP_FOREGROUND_REMOVE)
        super.onDestroy()
    }
}
