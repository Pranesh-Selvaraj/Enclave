package com.enclave.app.ui

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

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
    ROSE("rose", "Rose", Color(0xFFBE123C), Color(0xFFFDA4AF)),
    CUSTOM("custom", "Custom", Color(0xFF6C5CE7), Color(0xFFA99BFF));

    companion object {
        fun fromId(id: String?): Accent = entries.firstOrNull { it.id == id } ?: VIOLET
    }
}

/** How long the app may stay in the background before auto-locking. */
enum class AutoLock(val id: String, val label: String, val minutes: Int) {
    OFF("off", "Off", 0),
    ONE_MINUTE("1", "1 minute", 1),
    FIVE_MINUTES("5", "5 minutes", 5),
    FIFTEEN_MINUTES("15", "15 minutes", 15),
    THIRTY_MINUTES("30", "30 minutes", 30);

    companion object {
        fun fromId(id: String?): AutoLock = entries.firstOrNull { it.id == id } ?: FIVE_MINUTES
    }
}

/** Reading scale for the note title and editor, on top of the system font scale. */
enum class EditorTextSize(val id: String, val label: String, val scale: Float) {
    SMALL("small", "Small", 0.9f),
    DEFAULT("default", "Default", 1f),
    LARGE("large", "Large", 1.15f),
    EXTRA_LARGE("xl", "Extra large", 1.3f);

    companion object {
        fun fromId(id: String?): EditorTextSize = entries.firstOrNull { it.id == id } ?: DEFAULT
    }
}

/** Maximum width of the note content column. */
enum class EditorWidth(val id: String, val label: String, val maxWidth: Dp) {
    NARROW("narrow", "Narrow", 360.dp),
    STANDARD("standard", "Standard", 720.dp),
    WIDE("wide", "Wide", 960.dp);

    companion object {
        fun fromId(id: String?): EditorWidth = entries.firstOrNull { it.id == id } ?: STANDARD
    }
}

/** How the vault is unlocked on this device. PIN is the Android default. */
enum class AuthMode(val id: String, val label: String) {
    PIN("pin", "PIN"),
    PASSWORD("password", "Password");

    companion object {
        fun fromId(id: String?): AuthMode = entries.firstOrNull { it.id == id } ?: PIN
    }
}

/**
 * Device-local appearance and unlock preferences. Theme, accent and the
 * PIN/password choice describe *this* screen, so they are not vault data
 * (which would sync them to every device).
 */
object AppPrefs {
    private const val PREFS = "enclave-app"
    private const val KEY_THEME = "theme_mode"
    private const val KEY_ACCENT = "accent"
    private const val KEY_AUTH_MODE = "auth_mode"
    private const val KEY_AUTO_LOCK = "auto_lock"
    private const val KEY_TEXT_SIZE = "editor_text_size"
    private const val KEY_PAGE_WIDTH = "editor_page_width"
    private const val KEY_CUSTOM_ACCENT = "custom_accent"

    var themeMode by mutableStateOf(ThemeMode.SYSTEM)
        private set

    var accent by mutableStateOf(Accent.VIOLET)
        private set

    /** ARGB chosen in the custom picker; used only when [accent] is CUSTOM. */
    var customAccent by mutableStateOf<Color?>(null)
        private set

    /** The light-theme primary for the current accent (preset or custom). */
    val accentLight: Color
        get() = if (accent == Accent.CUSTOM) customAccent ?: Accent.VIOLET.light else accent.light

    /** A lightened variant for dark surfaces; presets carry their own. */
    val accentDark: Color
        get() = if (accent == Accent.CUSTOM) {
            lerp(customAccent ?: Accent.VIOLET.dark, Color.White, 0.35f)
        } else {
            accent.dark
        }

    var authMode by mutableStateOf(AuthMode.PIN)
        private set

    var autoLock by mutableStateOf(AutoLock.FIVE_MINUTES)
        private set

    var editorTextSize by mutableStateOf(EditorTextSize.DEFAULT)
        private set

    var editorWidth by mutableStateOf(EditorWidth.STANDARD)
        private set

    fun load(context: Context) {
        val prefs = prefs(context)
        themeMode = ThemeMode.fromId(prefs.getString(KEY_THEME, null))
        accent = Accent.fromId(prefs.getString(KEY_ACCENT, null))
        customAccent = prefs.getInt(KEY_CUSTOM_ACCENT, 0).takeIf { it != 0 }?.let { Color(it) }
        authMode = AuthMode.fromId(prefs.getString(KEY_AUTH_MODE, null))
        autoLock = AutoLock.fromId(prefs.getString(KEY_AUTO_LOCK, null))
        editorTextSize = EditorTextSize.fromId(prefs.getString(KEY_TEXT_SIZE, null))
        editorWidth = EditorWidth.fromId(prefs.getString(KEY_PAGE_WIDTH, null))
    }

    fun setThemeMode(context: Context, mode: ThemeMode) {
        themeMode = mode
        prefs(context).edit().putString(KEY_THEME, mode.id).apply()
    }

    fun setAccent(context: Context, value: Accent) {
        accent = value
        prefs(context).edit().putString(KEY_ACCENT, value.id).apply()
    }

    fun setCustomAccent(context: Context, value: Color) {
        customAccent = value
        accent = Accent.CUSTOM
        prefs(context).edit()
            .putString(KEY_ACCENT, Accent.CUSTOM.id)
            .putInt(KEY_CUSTOM_ACCENT, value.toArgb())
            .apply()
    }

    fun setAuthMode(context: Context, mode: AuthMode) {
        authMode = mode
        prefs(context).edit().putString(KEY_AUTH_MODE, mode.id).apply()
    }

    fun setAutoLock(context: Context, value: AutoLock) {
        autoLock = value
        prefs(context).edit().putString(KEY_AUTO_LOCK, value.id).apply()
    }

    fun setEditorTextSize(context: Context, value: EditorTextSize) {
        editorTextSize = value
        prefs(context).edit().putString(KEY_TEXT_SIZE, value.id).apply()
    }

    fun setEditorWidth(context: Context, value: EditorWidth) {
        editorWidth = value
        prefs(context).edit().putString(KEY_PAGE_WIDTH, value.id).apply()
    }

    private fun prefs(context: Context) =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
}
