package org.linmic.app

import java.io.IOException
import java.security.cert.CertificateException
import javax.net.ssl.SSLPeerUnverifiedException

class PairingFailure(message: String) : IOException(message)

object ReconnectPolicy {
    // TLS transport failures may be transient. Authentication failures never are.
    fun canRetry(error: Throwable, enabled: Boolean): Boolean {
        if (!enabled) return false
        var cause: Throwable? = error
        val seen = HashSet<Throwable>()
        while (cause != null && seen.add(cause)) {
            if (cause is PairingFailure || cause is CertificateException ||
                cause is SSLPeerUnverifiedException || cause is SecurityException ||
                cause is IllegalArgumentException) return false
            cause = cause.cause
        }
        return true
    }

    fun reconnectPin(saved: String, initial: String): String = saved.ifEmpty { initial }
}
