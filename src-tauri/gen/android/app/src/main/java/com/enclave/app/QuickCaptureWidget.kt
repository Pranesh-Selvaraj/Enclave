package com.enclave.app

import android.content.Context
import android.content.Intent
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.GlanceId
import androidx.compose.ui.graphics.Color
import androidx.glance.GlanceModifier
import androidx.glance.action.clickable
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.provideContent
import androidx.glance.background
import androidx.glance.layout.Alignment
import androidx.glance.layout.Box
import androidx.glance.layout.Column
import androidx.glance.layout.fillMaxSize
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.color.ColorProvider as DayNightColorProvider
import androidx.glance.text.TextStyle

/**
 * 1×1 quick capture: tapping opens the app straight into a new note
 * (same path as the launcher shortcut). No vault data involved.
 */
class QuickCaptureWidget : GlanceAppWidget() {
    override suspend fun provideGlance(context: Context, id: GlanceId) {
        provideContent {
            val accent = DayNightColorProvider(day = Color(0xFF6B5CE7), night = Color(0xFF8B7CF6))
            val onAccent = DayNightColorProvider(day = Color.White, night = Color.White)
            Box(
                modifier = GlanceModifier
                    .fillMaxSize()
                    .background(accent)
                    .cornerRadius(20.dp)
                    .clickable(actionStartActivity(quickCaptureIntent(context))),
                contentAlignment = Alignment.Center,
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text(
                        "+",
                        style = TextStyle(color = onAccent, fontSize = 26.sp, fontWeight = FontWeight.Bold),
                    )
                    Text(
                        "New note",
                        style = TextStyle(color = onAccent, fontSize = 11.sp),
                    )
                }
            }
        }
    }
}

class QuickCaptureWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = QuickCaptureWidget()
}

internal fun quickCaptureIntent(context: Context): Intent =
    EnclaveIntents.capture(context)

internal fun openNoteIntent(context: Context, docId: String): Intent =
    EnclaveIntents.openNote(context, docId)
