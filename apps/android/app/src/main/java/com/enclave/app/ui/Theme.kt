package com.enclave.app.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp

/**
 * Matte, fully opaque surfaces with warm neutrals — the Enclave design
 * language. The accent is user-selectable, so the primary colours are derived
 * from [Accent] and the rest of the palette stays fixed.
 */
private val WarmWhite = Color(0xFFF6F5F2)
private val WarmCharcoal = Color(0xFF121211)

private val LightColors = lightColorScheme(
    primary = Color(0xFF6C5CE7),
    onPrimary = Color.White,
    primaryContainer = Color(0xFFE7E2FF),
    onPrimaryContainer = Color(0xFF1D1140),
    secondary = Color(0xFF4C6EF5),
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
    primary = Color(0xFFA99BFF),
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
    mode: ThemeMode = ThemeMode.SYSTEM,
    accent: Accent = Accent.VIOLET,
    content: @Composable () -> Unit,
) {
    val dark = when (mode) {
        ThemeMode.SYSTEM -> isSystemInDarkTheme()
        ThemeMode.LIGHT -> false
        ThemeMode.DARK -> true
    }

    val base = if (dark) DarkColors else LightColors
    val primary = if (dark) accent.dark else accent.light
    val opposite = if (dark) Color.White else Color.Black

    MaterialTheme(
        colorScheme = base.copy(
            primary = primary,
            onPrimary = if (dark) Color(0xFF1B1A18) else Color.White,
            primaryContainer = lerp(primary, opposite, if (dark) 0.55f else 0.85f),
            onPrimaryContainer = lerp(primary, if (dark) Color.White else Color.Black, if (dark) 0.85f else 0.55f),
        ),
        content = content,
    )
}
