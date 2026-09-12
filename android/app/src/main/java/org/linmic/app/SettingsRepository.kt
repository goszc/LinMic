package org.linmic.app

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

class SettingsRepository(context: Context) {
    private val preferences = context.getSharedPreferences("settings", Context.MODE_PRIVATE)
    var host: String
        get() = preferences.getString("host", "")!!
        set(v) { preferences.edit().putString("host", v).apply() }
    var port: Int
        get() = preferences.getInt("port", 39820)
        set(v) { preferences.edit().putInt("port", v).apply() }
    var profile: String
        get() = preferences.getString("profile", "balanced")!!
        set(v) { preferences.edit().putString("profile", v).apply() }
    var gain: Float
        get() = preferences.getFloat("gain", 0f)
        set(v) { preferences.edit().putFloat("gain", v.coerceIn(-60f,24f)).apply() }
    var inputDevice: Int
        get() = preferences.getInt("device", 0)
        set(v) { preferences.edit().putInt("device", v).apply() }
    var reconnect: Boolean
        get() = preferences.getBoolean("reconnect", true)
        set(v) { preferences.edit().putBoolean("reconnect", v).apply() }
    var wifiLowLatency: Boolean
        get() = preferences.getBoolean("wifi", true)
        set(v) { preferences.edit().putBoolean("wifi", v).apply() }
    var systemNs: Boolean
        get() = preferences.getBoolean("ns", false)
        set(v) { preferences.edit().putBoolean("ns", v).apply() }
    var systemAgc: Boolean
        get() = preferences.getBoolean("agc", false)
        set(v) { preferences.edit().putBoolean("agc", v).apply() }
    var theme: Int
        get() = preferences.getInt("theme", 0)
        set(v) { preferences.edit().putInt("theme", v).apply() }
    fun fingerprint(host: String) = preferences.getString("pin:$host", "")!!
    fun saveFingerprint(host: String, pin: String) { preferences.edit().putString("pin:$host", pin).commit() }
    fun forget(host: String) { preferences.edit().remove("pin:$host").remove("token:$host").apply() }
    private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        return (store.getKey("linmic-pairing", null) as? SecretKey) ?: KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").run {
            init(KeyGenParameterSpec.Builder("linmic-pairing", KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())
            generateKey()
        }
    }
    fun saveToken(host: String, token: String) {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.ENCRYPT_MODE, key()) }
        val encrypted = cipher.doFinal(token.toByteArray())
        preferences.edit().putString("token:$host", Base64.encodeToString(cipher.iv + encrypted, Base64.NO_WRAP)).commit()
    }
    fun token(host: String): String {
        val text = preferences.getString("token:$host", null) ?: return ""
        return runCatching {
            val bytes = Base64.decode(text, Base64.NO_WRAP)
            val cipher = Cipher.getInstance("AES/GCM/NoPadding").apply { init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, bytes.copyOfRange(0, 12))) }
            String(cipher.doFinal(bytes.copyOfRange(12, bytes.size)))
        }.getOrDefault("")
    }
}
