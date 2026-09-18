package com.enclave.app

import android.content.Context
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
import androidx.glance.layout.Column
import androidx.glance.layout.Row
import androidx.glance.layout.Spacer
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.fillMaxWidth
import androidx.glance.layout.height
import androidx.glance.layout.padding
import androidx.glance.layout.width
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.color.ColorProvider as DayNightColorProvider
import androidx.glance.text.TextStyle
import androidx.glance.unit.ColorProvider

/**
 * Note list widget: the notes the user explicitly shared to widgets — readable
 * while the vault is locked because it renders the Keystore-wrapped
 * [WidgetStore] cache, never the vault itself.
 */
class NoteListWidget : GlanceAppWidget() {
    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val notes = WidgetStore.notes(context)
        provideContent { NoteListContent(context, notes) }
    }
}

class NoteListWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = NoteListWidget()
}

@Composable
private fun NoteListContent(context: Context, notes: List<WidgetStore.WidgetNote>) {
    val bg = DayNightColorProvider(day = Color(0xFFFAF7F1), night = Color(0xFF201C17))
    val fg = DayNightColorProvider(day = Color(0xFF211D17), night = Color(0xFFECE7DF))
    val muted = DayNightColorProvider(day = Color(0xFF6E6557), night = Color(0xFFA39B90))
    val accent = DayNightColorProvider(day = Color(0xFF6B5CE7), night = Color(0xFF8B7CF6))
    Column(
        modifier = GlanceModifier
            .fillMaxSize()
            .background(bg)
            .cornerRadius(20.dp)
            .padding(12.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                "Enclave",
                style = TextStyle(color = accent, fontSize = 13.sp, fontWeight = FontWeight.Bold),
            )
        }

        if (notes.isEmpty()) {
            Spacer(GlanceModifier.height(6.dp))
            Text(
                "No notes shared to widgets yet.\nOpen a note → “Show in widgets”.",
                style = TextStyle(color = muted, fontSize = 12.sp),
            )
            return@Column
        }

        Spacer(GlanceModifier.height(6.dp))
        notes.take(8).forEach { note ->
            NoteRow(context, note, fg, muted)
            Spacer(GlanceModifier.height(4.dp))
        }
    }
}

@Composable
private fun NoteRow(
    context: Context,
    note: WidgetStore.WidgetNote,
    fg: ColorProvider,
    muted: ColorProvider,
) {
    Column(
        modifier = GlanceModifier
            .fillMaxWidth()
            .clickable(actionStartActivity(openNoteIntent(context, note.id)))
            .padding(vertical = 3.dp),
    ) {
        Text(
            note.title.ifBlank { "Untitled" },
            style = TextStyle(color = fg, fontSize = 14.sp, fontWeight = FontWeight.Medium),
            maxLines = 1,
        )
        val preview = when {
            note.checklist.isNotEmpty() ->
                note.checklist.take(3).joinToString("  ") { (if (it.checked) "☑ " else "☐ ") + it.text }
            note.text.isNotBlank() -> note.text.replace(Regex("\\s+"), " ")
            else -> ""
        }
        if (preview.isNotBlank()) {
            Text(
                preview,
                style = TextStyle(color = muted, fontSize = 12.sp),
                maxLines = 2,
            )
        }
        if (note.tags.isNotEmpty()) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                note.tags.take(3).forEach { tag ->
                    Text(
                        "#$tag",
                        style = TextStyle(color = muted, fontSize = 10.sp),
                        modifier = GlanceModifier.padding(end = 6.dp),
                    )
                }
            }
        }
    }
}
