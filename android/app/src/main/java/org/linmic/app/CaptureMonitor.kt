package org.linmic.app

/** Progress, not loudness, determines whether the pipeline is alive. */
class CaptureMonitor {
    enum class Health { STARTING, LIVE, QUIET, MUTED, SYSTEM_BLOCKED, CAPTURE_STALLED, SEND_STALLED, DELIVERY_STALLED, DIGITAL_SILENCE }
    data class Result(val health: Health, val recover: Boolean = false)
    private var capture = -1f
    private var sent = -1f
    private var received = -1L
    private var capturedAt = 0L
    private var sentAt = 0L
    private var receivedAt = 0L
    private var signalAt = 0L
    private var silenceProbeUsed = false
    private val recoveries = java.util.ArrayDeque<Long>()
    fun begin(now: Long) {
        capture = -1f; sent = -1f; received = -1
        capturedAt = now; sentAt = now; receivedAt = now; signalAt = now
    }
    fun sample(now: Long, frames: Float, packets: Float, pcPackets: Long, rawPeak: Float, muted: Boolean, silenced: Boolean): Result {
        if (frames != capture) { capture = frames; capturedAt = now }
        if (packets != sent) { sent = packets; sentAt = now }
        if (pcPackets != received) { received = pcPackets; receivedAt = now }
        if (rawPeak > 0f && !silenced && !muted) silenceProbeUsed = false
        if (rawPeak > 0f || muted || silenced) signalAt = now
        val health = when {
            silenced -> Health.SYSTEM_BLOCKED
            now - capturedAt >= 4000 -> Health.CAPTURE_STALLED
            now - sentAt >= 4000 -> Health.SEND_STALLED
            now - receivedAt >= 5000 -> Health.DELIVERY_STALLED
            muted -> Health.MUTED
            now - signalAt >= 12000 -> Health.DIGITAL_SILENCE
            rawPeak < 0.003f -> Health.QUIET
            else -> Health.LIVE
        }
        if (health == Health.DIGITAL_SILENCE) {
            val repair = !silenceProbeUsed
            silenceProbeUsed = true
            return Result(health, repair)
        }
        val stuck = health in setOf(Health.CAPTURE_STALLED, Health.SEND_STALLED, Health.DELIVERY_STALLED)
        while (recoveries.isNotEmpty() && now - recoveries.first > 60000) recoveries.removeFirst()
        // A silence probe must never consume recovery capacity for a real capture/transport stall.
        val repair = stuck && recoveries.size < 3 && (recoveries.isEmpty() || now - recoveries.last >= 10000)
        if (repair) recoveries.addLast(now)
        return Result(health, repair)
    }
}
