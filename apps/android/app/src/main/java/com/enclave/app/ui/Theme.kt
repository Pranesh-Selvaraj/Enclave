package com.enclave.app.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

/**
 * Matte, fully opaque surfaces with warm neutrals and the violet accent from
 * the Enclave mark — the same design language as the shipping app.
 */
private val Violet = Color(0xFF6C5CE7)
private val VioletLight = Color(0xFFA99BFF)
private val Indigo = Color(0xFF4C6EF5)
private val WarmWhite = Color(0xFFF6F5F2)
private val WarmCharcoal = Color(0xFF121211)

private val LightColors = lightColorScheme(
    primary = Violet,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFE7E2FF),
    onPrimaryContainer = Color(0xFF1D1140),
    secondary = Indigo,
    onSecondary = Color.White,
    background = WarmWhite,
    onBackground = Color(0xFF1B1A18),
    surface = Color(0xFFFFFFFF),
    onSurface = Color(0xFF1B1A18),
    surfaceVariant = Color(0xFFEFEDE9),
    onSurfaceVariant = Color(0xFF5C5952),
    outline = Color(0xFFD8D5CF),
    outlineVariant = Color(0xFFE7E4DF),
    error = Color(0xFFB3261E),
)

private val DarkColors = darkColorScheme(
    primary = VioletLight,
    onPrimary = Color(0xFF1B1A18),
    primaryContainer = Color(0xFF332A63),
    onPrimaryContainer = Color(0xFFE7E2FF),
    secondary = Color(0xFF8FA8FF),
    onSecondary = Color(0xFF10121F),
    background = WarmCharcoal,
    onBackground = Color(0xFFECE9E4),
    surface = Color(0xFF1B1A19),
    onSurface = Color(0xFFECE9E4),
    surfaceVariant = Color(0xFF242322),
    onSurfaceVariant = Color(0xFFB9B4AC),
    outline = Color(0xFF3A3835),
    outlineVariant = Color(0xFF2A2927),
    error = Color(0xFFF2B8B5),
)

@Composable
fun EnclaveTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    MaterialTheme(
        colorScheme = if (darkTheme) DarkColors else LightColors,
        content = content,
    )
}
