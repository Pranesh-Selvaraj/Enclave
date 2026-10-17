package com.enclave.app.ui

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.Color

/** Theme mode preference. */
enum class ThemeMode(val id: String, val label: String) {
    SYSTEM("system", "System"),
    LIGHT("light", "Light"),
    DARK("dark", "Dark");

    companion object {
        fun fromId(id: String?): ThemeMode = entries.firstOrNull { it.id == id } ?: SYSTEM
    }
}

/** Accent presets; the light/dark variants keep contrast in both themes. */
enum class Accent(
    val id: String,
    val label: String,
    val light: Color,
    val dark: Color,
) {
    VIOLET("violet", "Violet", Color(0xFF6C5CE7), Color(0xFFA99BFF)),
    INDIGO("indigo", "Indigo", Color(0xFF4C6EF5), Color(0xFF8FA8FF)),
    TEAL("teal", "Teal", Color(0xFF0F766E), Color(0xFF5EEAD4)),
    AMBER("amber", "Amber", Color(0xFFB45309), Color(0xFFFBBF24)),
    ROSE("rose", "Rose", Color(0xFFBE123C), Color(0xFFFDA4AF));

    companion object {
        fun fromId(id: String?): Accent = entries.firstOrNull { it.id == id } ?: VIOLET
    }
}

/**
 * Device-local appearance preferences. Theme and accent describe *this*
 * screen, so they are not vault data (which would sync them to every device).
 */
object AppPrefs {
    private const val PREFS = "enclave-app"
    private const val KEY_THEME = "theme_mode"
    private const val KEY_ACCENT = "accent"

    var themeMode by mutableStateOf(ThemeMode.SYSTEM)
        private set

    var accent by mutableStateOf(Accent.VIOLET)
        private set

    fun load(context: Context) {
        val prefs = prefs(context)
        themeMode = ThemeMode.fromId(prefs.getString(KEY_THEME, null))
        accent = Accent.fromId(prefs.getString(KEY_ACCENT, null))
    }

    fun setThemeMode(context: Context, mode: ThemeMode) {
        themeMode = mode
        prefs(context).edit().putString(KEY_THEME, mode.id).apply()
    }

    fun setAccent(context: Context, value: Accent) {
        accent = value
        prefs(context).edit().putString(KEY_ACCENT, value.id).apply()
    }

    private fun prefs(context: Context) =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
