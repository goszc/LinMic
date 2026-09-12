package org.linmic.app

import java.io.DataInputStream
import java.io.DataOutputStream

object Wire {
    const val MAX_CONTROL = 16384
    fun read(input: DataInputStream): String {
        val length = input.readInt()
        require(length in 1..MAX_CONTROL) { "Invalid control frame length" }
        val data = ByteArray(length)
        input.readFully(data)
        return data.toString(Charsets.UTF_8)
    }
    fun write(output: DataOutputStream, text: String) {
        val data = text.toByteArray(Charsets.UTF_8)
        require(data.size in 1..MAX_CONTROL)
        output.writeInt(data.size); output.write(data); output.flush()
    }
}
data class TimeEstimate(val rttMs: Double, val clockOffsetUs: Double)
object Timing {
    fun estimate(t0: Long, t1: Long, t2: Long, t3: Long): TimeEstimate {
        require(t3 >= t0 && t2 >= t1)
        val rtt = ((t3-t0)-(t2-t1)).coerceAtLeast(0)/1000.0
        val offset = ((t1-t0).toDouble()+(t2-t3).toDouble())/2.0
        return TimeEstimate(rtt,offset)
    }
    fun frameSamples(profile: String) = when(profile) { "ultra" -> 240; "balanced" -> 480; "stable" -> 960; else -> throw IllegalArgumentException("Unknown profile") }
}
