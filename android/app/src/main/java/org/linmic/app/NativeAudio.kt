package org.linmic.app

object NativeAudio {
    init { System.loadLibrary("linmic") }
    external fun pair(code: String, peer: ByteArray, certificate: ByteArray): ByteArray?
    external fun setPrivacyMute(muted: Boolean)
    external fun create(): Long
    external fun destroy(handle: Long)
    external fun configure(handle: Long, host: String, port: Int, bitrate: Int, frame: Int, session: Long, key: ByteArray, device: Int, compatible: Boolean): Int
    external fun sessionId(handle: Long): Int
    external fun start(handle: Long): Int
    external fun stop(handle: Long)
    external fun setMuted(handle: Long, muted: Boolean)
    external fun setGain(handle: Long, gain: Float)
    external fun setLoss(handle: Long, loss: Int)
    external fun stats(handle: Long, output: FloatArray)
}
class NativeAudioController {
    private var handle = 0L
    @Synchronized fun start(host: String, port: Int, bitrate: Int, frame: Int, session: Long, key: ByteArray, device: Int, gain: Float, muted: Boolean, compatible: Boolean = true) {
        stop()
        handle = NativeAudio.create()
        try {
            check(NativeAudio.configure(handle, host, port, bitrate, frame, session, key, device, compatible) == 0) { "Audio configuration failed" }
            NativeAudio.setGain(handle, gain)
            NativeAudio.setMuted(handle, muted)
            val code = NativeAudio.start(handle)
            check(code == 0) { "Microphone could not start ($code)" }
        } catch (e: Exception) { stop(); throw e }
    }
    @Synchronized fun sessionId(): Int = if (handle != 0L) NativeAudio.sessionId(handle) else 0
    @Synchronized fun stop() { if (handle != 0L) { NativeAudio.destroy(handle); handle = 0 } }
    @Synchronized fun mute(value: Boolean) { if (handle != 0L) NativeAudio.setMuted(handle, value) }
    @Synchronized fun gain(value: Float) { if (handle != 0L) NativeAudio.setGain(handle, value) }
    @Synchronized fun loss(value: Int) { if (handle != 0L) NativeAudio.setLoss(handle, value) }
    @Synchronized fun snapshot(output: FloatArray) { if (handle != 0L) NativeAudio.stats(handle, output) else output.fill(0f) }
}
