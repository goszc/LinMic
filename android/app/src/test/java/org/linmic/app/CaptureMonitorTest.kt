package org.linmic.app

import org.junit.Assert.*
import org.junit.Test

class CaptureMonitorTest {
    @Test fun quietRoomIsNotAStalledMicrophone() {
        val m=CaptureMonitor();m.begin(0)
        for(t in 0..120){val r=m.sample(t*1000L,t*48000f,t*100f,t*100L,0.00002f,false,false);assertEquals(CaptureMonitor.Health.QUIET,r.health);assertFalse(r.recover)}
    }
    @Test fun muteAndAndroidPrivacySilenceDoNotTriggerReopen() {
        for(blocked in listOf(false,true)) {
            val m=CaptureMonitor();m.begin(0)
            for(t in 0..120){val r=m.sample(t*1000L,t*48000f,t*100f,t*100L,0f,!blocked,blocked);assertFalse(r.recover);assertEquals(if(blocked)CaptureMonitor.Health.SYSTEM_BLOCKED else CaptureMonitor.Health.MUTED,r.health)}
        }
    }
    @Test fun detectsCaptureStoppedWithLiveControlConnection() {
        val m=CaptureMonitor();m.begin(0);m.sample(0,480f,1f,1,0.1f,false,false)
        val r=m.sample(4000,480f,1f,1,0.1f,false,false)
        assertEquals(CaptureMonitor.Health.CAPTURE_STALLED,r.health);assertTrue(r.recover)
    }
    @Test fun detectsEncoderOrSenderStoppedDespiteCaptureProgress() {
        val m=CaptureMonitor();m.begin(0);m.sample(0,480f,1f,1,0.1f,false,false)
        val r=m.sample(4000,192000f,1f,1,0.1f,false,false)
        assertEquals(CaptureMonitor.Health.SEND_STALLED,r.health);assertTrue(r.recover)
    }
    @Test fun detectsPcNotReceivingEvenWhenPhoneSends() {
        val m=CaptureMonitor();m.begin(0);m.sample(0,480f,1f,1,0.1f,false,false)
        val r=m.sample(5000,240000f,500f,1,0.1f,false,false)
        assertEquals(CaptureMonitor.Health.DELIVERY_STALLED,r.health);assertTrue(r.recover)
    }
    @Test fun allZeroDriverFramesGetBoundedRecovery() {
        val m=CaptureMonitor();m.begin(0)
        var repairs=0
        for(t in 0..240){val r=m.sample(t*1000L,t*48000f,t*100f,t*100L,0f,false,false);if(r.recover)repairs++}
        assertEquals(1,repairs)
    }
    @Test fun silenceProbeDoesNotBlockRecoveryOfRealCaptureFailure() {
        val m=CaptureMonitor();m.begin(0)
        for(t in 0..12)m.sample(t*1000L,t*48000f,t*100f,t*100L,0f,false,false)
        val r=m.sample(16000,12*48000f,1200f,1200,0f,false,false)
        assertEquals(CaptureMonitor.Health.CAPTURE_STALLED,r.health);assertTrue(r.recover)
    }
    @Test fun reconnectGraceDoesNotForgetRecoveryCooldown() {
        val m=CaptureMonitor();m.begin(0);m.sample(0,480f,1f,1,0.1f,false,false)
        assertTrue(m.sample(4000,480f,1f,1,0.1f,false,false).recover)
        m.begin(5000);assertFalse(m.sample(5000,0f,0f,0,0f,false,false).recover)
        assertFalse(m.sample(9000,0f,0f,0,0f,false,false).recover)
    }
    @Test fun resumesAfterSystemUnmutesAndCounterReset() {
        val m=CaptureMonitor();m.begin(0)
        assertEquals(CaptureMonitor.Health.SYSTEM_BLOCKED,m.sample(0,480f,1f,1,0f,false,true).health)
        assertEquals(CaptureMonitor.Health.LIVE,m.sample(1000,48000f,100f,100,0.2f,false,false).health)
        m.begin(2000);assertEquals(CaptureMonitor.Health.LIVE,m.sample(2100,480f,1f,1,0.2f,false,false).health)
    }
}
