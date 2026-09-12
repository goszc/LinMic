package org.linmic.app
import org.junit.Assert.*
import org.junit.Test
import java.io.*
class WireTest {
    @Test fun unicodeFrame() {
        val bytes=ByteArrayOutputStream();Wire.write(DataOutputStream(bytes),"{\"name\":\"Telefone — João\"}")
        assertEquals("{\"name\":\"Telefone — João\"}",Wire.read(DataInputStream(ByteArrayInputStream(bytes.toByteArray()))))
    }
    @Test(expected=IllegalArgumentException::class) fun oversizedFrame() { Wire.read(DataInputStream(ByteArrayInputStream(byteArrayOf(0x7f,0xff.toByte(),0xff.toByte(),0xff.toByte())))) }
    @Test(expected=IllegalArgumentException::class) fun zeroLength() { Wire.read(DataInputStream(ByteArrayInputStream(ByteArray(4)))) }
    @Test(expected=EOFException::class) fun truncatedFrame() { Wire.read(DataInputStream(ByteArrayInputStream(byteArrayOf(0,0,0,4,65)))) }
    @Test fun ntpRemovesServerProcessing() { val e=Timing.estimate(1000,11000,13000,5000);assertEquals(2.0,e.rttMs,0.0001);assertEquals(9000.0,e.clockOffsetUs,0.0001) }
    @Test fun profiles() { assertEquals(240,Timing.frameSamples("ultra"));assertEquals(480,Timing.frameSamples("balanced"));assertEquals(960,Timing.frameSamples("stable")) }
    @Test(expected=IllegalArgumentException::class) fun invalidProfile() {Timing.frameSamples("unknown")}
}
