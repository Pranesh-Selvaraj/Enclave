package com.enclave.app

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

/** Enclave palette for the native Android surfaces (widget config, …). */
@Composable
internal fun EnclaveTheme(content: @Composable () -> Unit) {
    val dark = isSystemInDarkTheme()
    val scheme = if (dark) {
        darkColorScheme(
            primary = Color(0xFF8B7CF6),
            background = Color(0xFF16130F),
            surface = Color(0xFF201C17),
            onBackground = Color(0xFFECE7DF),
            onSurface = Color(0xFFECE7DF),
        )
    } else {
        lightColorScheme(
            primary = Color(0xFF6B5CE7),
            background = Color(0xFFF1EDE5),
            surface = Color(0xFFFAF7F1),
            onBackground = Color(0xFF211D17),
            onSurface = Color(0xFF211D17),
        )
    }
    MaterialTheme(colorScheme = scheme, content = content)
}
