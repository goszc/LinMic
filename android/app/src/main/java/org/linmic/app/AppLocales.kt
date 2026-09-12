package org.linmic.app
import android.content.Context
import android.content.res.Configuration
import android.os.LocaleList
object AppLocales {
    val tags=arrayOf("", "pt-BR", "en", "es", "ru", "zh-CN", "ja")
    val names=arrayOf("System / Sistema", "Português (Brasil)", "English", "Español", "Русский", "简体中文", "日本語")
    fun selected(context:Context)=context.getSharedPreferences("settings",Context.MODE_PRIVATE).getString("language","")!!
    fun wrap(context:Context):Context {
        val language=selected(context)
        if(language.isEmpty())return context
        val config=Configuration(context.resources.configuration)
        config.setLocales(LocaleList.forLanguageTags(language));return context.createConfigurationContext(config)
    }
    fun save(context:Context,language:String){context.getSharedPreferences("settings",Context.MODE_PRIVATE).edit().putString("language",language).apply()}
}
