package org.linmic.app

import android.app.ActivityManager
import android.content.Context
import android.os.Build
import org.json.JSONArray
import org.json.JSONObject

/** Private, bounded metadata only: never audio, host names, addresses or authentication material. */
class SessionDiagnostics(private val context: Context) {
    private val prefs = context.getSharedPreferences("diagnostics", Context.MODE_PRIVATE)
    @Synchronized fun event(code: String) {
        val old = runCatching { JSONArray(prefs.getString("events", "[]")) }.getOrDefault(JSONArray())
        val next = JSONArray()
        for (i in (old.length()-39).coerceAtLeast(0) until old.length()) next.put(old.getJSONObject(i))
        next.put(JSONObject().put("time", System.currentTimeMillis()).put("event", code.replace(Regex("[^a-zA-Z0-9_:=. -]"), "").take(100)))
        prefs.edit().putString("events", next.toString()).apply()
    }
    fun report(): String {
        val events = runCatching { JSONArray(prefs.getString("events", "[]")) }.getOrDefault(JSONArray())
        return buildString {
            append("LinMic 0.3.0 | Android API ${Build.VERSION.SDK_INT}\n")
            append("State: ${StreamModel.state.javaClass.simpleName}\nHealth: ${StreamModel.health}\n")
            append("Repairs: ${StreamModel.repairs} | Capture: ${StreamModel.meter.getOrElse(7){0f}} | Sent: ${StreamModel.meter.getOrElse(2){0f}}\n")
            append("System silenced: ${StreamModel.systemSilenced}\n")
            if (Build.VERSION.SDK_INT >= 30) {
                runCatching { context.getSystemService(ActivityManager::class.java).getHistoricalProcessExitReasons(null, 0, 3) }.getOrNull()?.forEach {
                    append("Previous exit: reason=${it.reason}, time=${it.timestamp}\n")
                }
            }
            for (i in 0 until events.length()) {
                val row=events.getJSONObject(i)
                append("${java.text.DateFormat.getTimeInstance().format(java.util.Date(row.getLong("time")))}  ${row.getString("event")}\n")
            }
        }
    }
}
