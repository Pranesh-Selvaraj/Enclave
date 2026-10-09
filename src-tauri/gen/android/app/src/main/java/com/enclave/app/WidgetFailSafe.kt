package com.enclave.app

import android.util.Log
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.provideContent
import androidx.glance.background
import androidx.glance.color.ColorProvider as DayNightColorProvider
import androidx.glance.layout.Column
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.padding
import androidx.glance.text.Text
import androidx.glance.text.TextStyle

private const val TAG = "EnclaveWidgets"

/**
 * One failure policy for every widget entry point (issue #121).
 *
 * Widget updates run in the app process, so an exception escaping
 * `provideGlance` (or an update broadcast) surfaces as "Problem loading
 * widget" and can take the process down. This wrapper catches `Throwable`,
 * logs the widget name/id and cause to [TAG], and renders the failure
 * placeholder instead — the same visual language as the locked-vault state.
 */
internal suspend fun GlanceAppWidget.provideGlanceSafely(
    name: String,
    id: GlanceId?,
    block: suspend () -> Unit,
) {
    try {
        block()
    } catch (t: Throwable) {
        Log.e(TAG, "$name failed to render (id=$id)", t)
        provideContent { WidgetFailureContent() }
    }
}

/** Rendered when a widget update fails, so a failure never leaves a dead widget. */
@Composable
internal fun WidgetFailureContent() {
    val bg = DayNightColorProvider(day = Color(0xFFFAF7F1), night = Color(0xFF201C17))
    val muted = DayNightColorProvider(day = Color(0xFF6E6557), night = Color(0xFFA39B90))
    val accent = DayNightColorProvider(day = Color(0xFF6B5CE7), night = Color(0xFF8B7CF6))
    Column(
        modifier = GlanceModifier
            .fillMaxSize()
            .background(bg)
            .cornerRadius(20.dp)
            .padding(12.dp),
    ) {
        Text(
            "Enclave",
            style = TextStyle(color = accent, fontSize = 13.sp, fontWeight = androidx.glance.text.FontWeight.Bold),
        )
        Text(
            "🔒 Widget unavailable\nOpen Enclave to refresh.",
            style = TextStyle(color = muted, fontSize = 12.sp),
        )
    }
}

/**
 * Runs [block] for a widget action/update path, logging any [Throwable] with
 * the widget name instead of letting it escape into the app process.
 */
internal suspend fun runWidgetSafely(name: String, block: suspend () -> Unit) {
    try {
        block()
    } catch (t: Throwable) {
        Log.e(TAG, "$name failed", t)
    }
}
