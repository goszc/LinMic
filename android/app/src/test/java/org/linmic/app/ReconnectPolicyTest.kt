package org.linmic.app

import java.io.EOFException
import java.net.SocketTimeoutException
import java.security.cert.CertificateException
import javax.net.ssl.SSLException
import javax.net.ssl.SSLHandshakeException
import javax.net.ssl.SSLPeerUnverifiedException
import org.junit.Assert.*
import org.junit.Test

class ReconnectPolicyTest {
    @Test fun retriesTransientTransportIncludingTlsEof() {
        for (error in listOf(EOFException(), SocketTimeoutException(), SSLException("Connection reset"), SSLHandshakeException("Remote peer closed"))) {
            assertTrue(ReconnectPolicy.canRetry(error, true))
        }
    }
    @Test fun rejectsAuthenticationFailuresIncludingWrappedCertificates() {
        val wrapped = SSLHandshakeException("Handshake failed").apply { initCause(CertificateException("Changed identity")) }
        for (error in listOf(wrapped, SSLPeerUnverifiedException("Unverified"), PairingFailure("Bad code"), SecurityException("Microphone permission"))) {
            assertFalse(ReconnectPolicy.canRetry(error, true))
        }
    }
    @Test fun respectsDisabledReconnect() {
        assertFalse(ReconnectPolicy.canRetry(SocketTimeoutException(), false))
    }
    @Test fun firstSessionReconnectUsesNewlySavedIdentity() {
        assertEquals("authenticated-pin", ReconnectPolicy.reconnectPin("authenticated-pin", ""))
        assertEquals("saved-pin", ReconnectPolicy.reconnectPin("saved-pin", "old-pin"))
        assertEquals("initial-pin", ReconnectPolicy.reconnectPin("", "initial-pin"))
        assertEquals("", ReconnectPolicy.reconnectPin("", ""))
    }
}
