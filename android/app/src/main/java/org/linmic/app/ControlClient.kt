package org.linmic.app

import org.json.JSONObject
import java.io.DataInputStream
import java.io.DataOutputStream
import java.net.InetSocketAddress
import java.net.Socket
import java.security.MessageDigest
import java.security.SecureRandom
import java.security.cert.X509Certificate
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLSocket
import javax.net.ssl.TrustManager
import javax.net.ssl.X509TrustManager

// Initial TLS trust is provisional until PAKE proves the six-digit code and binds this certificate.
// Returning connections require the previously authenticated pin before sending a device token.
@android.annotation.SuppressLint("CustomX509TrustManager")
class ControlClient(private val settings: SettingsRepository) : AutoCloseable {
    @Volatile private var socket: Socket? = null
    private lateinit var input: DataInputStream
    private lateinit var output: DataOutputStream
    fun connect(host: String, port: Int, fingerprint: String, code: String): JSONObject {
        require(fingerprint.isEmpty() || fingerprint.matches(Regex("[a-fA-F0-9]{64}")))
        if (fingerprint.isEmpty()) require(code.matches(Regex("[0-9]{6}"))) { "Pairing: enter the six-digit desktop code" }
        var presented = ByteArray(0)
        val trust = object : X509TrustManager {
            override fun getAcceptedIssuers() = emptyArray<X509Certificate>()
            override fun checkClientTrusted(chain: Array<out X509Certificate>, auth: String) { throw java.security.cert.CertificateException("Not a server") }
            override fun checkServerTrusted(chain: Array<out X509Certificate>, auth: String) {
                if (chain.isEmpty()) throw java.security.cert.CertificateException("Missing certificate")
                val actual = MessageDigest.getInstance("SHA-256").digest(chain[0].encoded)
                if (fingerprint.isNotEmpty()) {
                    val expected = decodeHex(fingerprint)
                    if (!MessageDigest.isEqual(actual, expected)) throw java.security.cert.CertificateException("The PC identity changed. Remove pairing and pair again.")
                }
                presented = actual
                chain[0].checkValidity()
            }
        }
        val ssl = SSLContext.getInstance("TLS").apply { init(null, arrayOf<TrustManager>(trust), SecureRandom()) }
        val tcp = Socket(); socket = tcp
        val address=java.net.InetAddress.getAllByName(host).firstOrNull { it.isSiteLocalAddress || it.isLoopbackAddress || it.isLinkLocalAddress || (it.address.size==16 && (it.address[0].toInt() and 254)==252) }
            ?: throw IllegalArgumentException("Use a PC on your local network")
        tcp.connect(InetSocketAddress(address, port), 5000)
        val tls = ssl.socketFactory.createSocket(tcp, host, port, true) as SSLSocket
        socket = tls
        tls.soTimeout = 5000
        tls.tcpNoDelay = true
        tls.enabledProtocols = tls.supportedProtocols.filter { it == "TLSv1.3" || it == "TLSv1.2" }.toTypedArray()
        tls.startHandshake()
        input = DataInputStream(tls.inputStream); output = DataOutputStream(tls.outputStream)
        send(JSONObject().put("type", "hello").put("protocol", 1).put("client", JSONObject().put("name", android.os.Build.MODEL))
            .put("profile", settings.profile).put("token", if(fingerprint.isNotEmpty()) settings.token(host) else ""))
        var response = read()
        if (response.optString("type") == "pair_challenge") {
            check(response.optString("method") == "spake2-v1") { "Pairing: desktop app must be updated" }
            val exchange = NativeAudio.pair(code, decodeHex(response.getString("message")), presented)
                ?: throw IllegalStateException("Pairing: invalid challenge")
            send(JSONObject().put("type", "pair_submit").put("method", "spake2-v1")
                .put("message", encodeHex(exchange.copyOfRange(0,33))).put("proof", encodeHex(exchange.copyOfRange(33,65))))
            val paired = read()
            check(paired.optString("type") == "pair_success") { "Pairing: check the code displayed on desktop" }
            check(MessageDigest.isEqual(exchange.copyOfRange(65,97), decodeHex(paired.getString("proof")))) { "Pairing: PC authentication failed" }
            exchange.fill(0)
            settings.saveToken(host, paired.getString("device_token"))
            settings.saveFingerprint(host, encodeHex(presented))
            response = read()
        }
        check(response.optString("type") == "hello_ack") { if(response.optString("type")=="pair_error") "Pairing: generate a code in the desktop app" else "PC rejected the connection" }
        response.put("_peer_host",tls.inetAddress.hostAddress)
        return response
    }
    private fun decodeHex(text: String): ByteArray { require(text.length % 2 == 0 && text.length <= 128); return text.chunked(2).map { it.toInt(16).toByte() }.toByteArray() }
    private fun encodeHex(bytes: ByteArray) = bytes.joinToString("") { "%02x".format(it.toInt() and 255) }
    fun request(value: JSONObject): JSONObject { send(value); return read() }
    private fun send(value: JSONObject) { Wire.write(output, value.toString()) }
    private fun read(): JSONObject = JSONObject(Wire.read(input))
    override fun close() { runCatching { socket?.close() }; socket = null }
}
