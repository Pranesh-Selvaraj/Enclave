package com.enclave.app.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import com.enclave.app.R

/**
 * Matte, fully opaque surfaces with warm neutrals — the Enclave design
 * language. The accent is user-selectable, so the primary colours are derived
 * from [Accent] and the rest of the palette stays fixed.
 */
private val WarmWhite = Color(0xFFF6F5F2)
private val WarmCharcoal = Color(0xFF121211)

/**
 * Brand typefaces, bundled so text is identical on every device (the same
 * Inter and JetBrains Mono the web editor ships).
 */
val EnclaveSans: FontFamily = FontFamily(
    Font(R.font.inter_regular, FontWeight.Normal),
    Font(R.font.inter_medium, FontWeight.Medium),
    Font(R.font.inter_semibold, FontWeight.SemiBold),
    Font(R.font.inter_bold, FontWeight.Bold),
    Font(R.font.inter_italic, FontWeight.Normal, FontStyle.Italic),
)

val EnclaveMono: FontFamily = FontFamily(
    Font(R.font.jetbrains_mono_regular, FontWeight.Normal),
    Font(R.font.jetbrains_mono_medium, FontWeight.Medium),
)

/** Material typography re-based on Inter; headings keep the same family. */
private fun typographyWithInter(base: Typography = Typography()): Typography = Typography(
    displayLarge = base.displayLarge.copy(fontFamily = EnclaveSans),
    displayMedium = base.displayMedium.copy(fontFamily = EnclaveSans),
    displaySmall = base.displaySmall.copy(fontFamily = EnclaveSans),
    headlineLarge = base.headlineLarge.copy(fontFamily = EnclaveSans),
    headlineMedium = base.headlineMedium.copy(fontFamily = EnclaveSans),
    headlineSmall = base.headlineSmall.copy(fontFamily = EnclaveSans),
    titleLarge = base.titleLarge.copy(fontFamily = EnclaveSans),
    titleMedium = base.titleMedium.copy(fontFamily = EnclaveSans),
    titleSmall = base.titleSmall.copy(fontFamily = EnclaveSans),
    bodyLarge = base.bodyLarge.copy(fontFamily = EnclaveSans),
    bodyMedium = base.bodyMedium.copy(fontFamily = EnclaveSans),
    bodySmall = base.bodySmall.copy(fontFamily = EnclaveSans),
    labelLarge = base.labelLarge.copy(fontFamily = EnclaveSans),
    labelMedium = base.labelMedium.copy(fontFamily = EnclaveSans),
    labelSmall = base.labelSmall.copy(fontFamily = EnclaveSans),
)

val EnclaveTypography = typographyWithInter()

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

private val MonoColors = darkColorScheme(
    primary = Color(0xFFE8E8E8),
    onPrimary = Color(0xFF141414),
    primaryContainer = Color(0xFF2A2A2A),
    onPrimaryContainer = Color(0xFFF2F2F2),
    secondary = Color(0xFFBDBDBD),
    onSecondary = Color(0xFF141414),
    background = Color(0xFF0E0E0E),
    onBackground = Color(0xFFEDEDED),
    surface = Color(0xFF1A1A1A),
    onSurface = Color(0xFFEDEDED),
    surfaceVariant = Color(0xFF242424),
    onSurfaceVariant = Color(0xFFB5B5B5),
    outline = Color(0xFF3A3A3A),
    outlineVariant = Color(0xFF2A2A2A),
    error = Color(0xFFF2B8B5),
)

@Composable
fun EnclaveTheme(
    mode: ThemeMode = ThemeMode.SYSTEM,
    accentLight: Color = Accent.VIOLET.light,
    accentDark: Color = Accent.VIOLET.dark,
    content: @Composable () -> Unit,
) {
    val mono = mode == ThemeMode.MONOCHROME
    val dark = when (mode) {
        ThemeMode.SYSTEM -> isSystemInDarkTheme()
        ThemeMode.LIGHT -> false
        ThemeMode.DARK, ThemeMode.MONOCHROME -> true
    }

    val base = when {
        mono -> MonoColors
        dark -> DarkColors
        else -> LightColors
    }
    val primary = if (mono) base.primary else if (dark) accentDark else accentLight
    // Pick the label colour from the accent's luminance, not the theme mode:
    // a custom accent can be light in the light theme or dark in the dark one.
    val onPrimary = if (mono) base.onPrimary else if (primary.luminance() > 0.18f) Color(0xFF1B1A18) else Color.White
    val opposite = if (dark) Color.White else Color.Black

    MaterialTheme(
        colorScheme = base.copy(
            primary = primary,
            onPrimary = onPrimary,
            primaryContainer = if (mono) base.primaryContainer else lerp(primary, opposite, if (dark) 0.55f else 0.85f),
            onPrimaryContainer = if (mono) base.onPrimaryContainer else lerp(primary, if (dark) Color.White else Color.Black, if (dark) 0.85f else 0.55f),
        ),
        typography = EnclaveTypography,
        content = content,
    )
}
