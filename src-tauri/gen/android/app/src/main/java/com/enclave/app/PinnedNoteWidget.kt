package com.enclave.app

import android.appwidget.AppWidgetManager
import android.content.Context
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.action.ActionParameters
import androidx.glance.action.actionParametersOf
import androidx.glance.action.clickable
import androidx.glance.appwidget.AppWidgetId
import androidx.glance.appwidget.action.ActionCallback
import androidx.glance.appwidget.action.actionRunCallback
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.provideContent
import androidx.glance.appwidget.state.getAppWidgetState
import androidx.glance.appwidget.state.updateAppWidgetState
import androidx.glance.background
import androidx.glance.color.ColorProvider as DayNightColorProvider
import androidx.glance.layout.Alignment as GlanceAlignment
import androidx.glance.layout.Column as GlanceColumn
import androidx.glance.layout.Row as GlanceRow
import androidx.glance.layout.Spacer as GlanceSpacer
import androidx.glance.layout.fillMaxSize as glanceFillMaxSize
import androidx.glance.layout.height as glanceHeight
import androidx.glance.layout.padding as glancePadding
import androidx.glance.state.PreferencesGlanceStateDefinition
import androidx.glance.text.FontWeight as GlanceFontWeight
import androidx.glance.text.Text as GlanceText
import androidx.glance.text.TextStyle as GlanceTextStyle
import androidx.lifecycle.lifecycleScope
import uniffi.core_api.FfiCore
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

/**
 * Pinned-note widget: one note, shown entirely, chosen at add time from the
 * notes shared to widgets. Renders from the Keystore-wrapped cache, so it
 * keeps working while the vault is locked.
 */
class PinnedNoteWidget : GlanceAppWidget() {
    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val prefs: Preferences = getAppWidgetState(context, PreferencesGlanceStateDefinition, id)
        var docId = prefs[KEY_DOC_ID]
        if (docId == null) {
            // Freshly pinned from the note editor: bind the note it carried.
            docId = WidgetStore.consumePendingPin(context)
            if (docId != null) {
                updateAppWidgetState(context, id) { state -> state[KEY_DOC_ID] = docId!! }
            }
        }
        val wanted = docId
        val hidden = WidgetStore.shouldHide(context)
        val note = if (hidden) null else wanted?.let { id2 -> WidgetStore.notes(context).firstOrNull { it.id == id2 } }
        provideContent { PinnedNoteContent(note, hidden) }
    }

    companion object {
        val KEY_DOC_ID = stringPreferencesKey("doc_id")
    }
}

class PinnedNoteWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = PinnedNoteWidget()
}

/**
 * Success callback for "pin this note": the system tells us the new widget id,
 * we bind the chosen document to it and render. This is the flow used from the
 * note editor (the launcher's widget picker uses the configure activity).
 */
class PinResultReceiver : android.content.BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val widgetId = intent.getIntExtra(
            AppWidgetManager.EXTRA_APPWIDGET_ID,
            AppWidgetManager.INVALID_APPWIDGET_ID,
        )
        val docId = intent.getStringExtra(EXTRA_DOC_ID)
        if (widgetId == AppWidgetManager.INVALID_APPWIDGET_ID || docId == null) return
        val pending = goAsync()
        CoroutineScope(Dispatchers.IO).launch {
            try {
                val glanceId = AppWidgetId(widgetId)
                updateAppWidgetState(context, glanceId) { prefs ->
                    prefs[PinnedNoteWidget.KEY_DOC_ID] = docId
                }
                PinnedNoteWidget().update(context, glanceId)
                android.util.Log.d("EnclaveWidgets", "PinResult bound widget " + widgetId)
            } catch (e: Throwable) {
                android.util.Log.e("EnclaveWidgets", "PinResult failed: " + e.message, e)
            } finally {
                pending.finish()
            }
        }
    }

    companion object {
        const val EXTRA_DOC_ID = "enclave:pinDocId"
    }
}

@Composable
private fun PinnedNoteContent(note: WidgetStore.WidgetNote?, hidden: Boolean = false) {
    val bg = DayNightColorProvider(day = Color(0xFFFAF7F1), night = Color(0xFF201C17))
    val fg = DayNightColorProvider(day = Color(0xFF211D17), night = Color(0xFFECE7DF))
    val muted = DayNightColorProvider(day = Color(0xFF6E6557), night = Color(0xFFA39B90))
    val accent = DayNightColorProvider(day = Color(0xFF6B5CE7), night = Color(0xFF8B7CF6))

    GlanceColumn(
        modifier = GlanceModifier
            .glanceFillMaxSize()
            .background(bg)
            .cornerRadius(20.dp)
            .glancePadding(12.dp),
    ) {
        if (hidden) {
            GlanceText(
                "🔒 Vault locked\nUnlock Enclave to see this note.",
                style = GlanceTextStyle(color = muted, fontSize = 12.sp),
            )
            return@GlanceColumn
        }

        if (note == null) {
            GlanceText(
                "No note pinned yet.\nLong-press this widget → reconfigure.",
                style = GlanceTextStyle(color = muted, fontSize = 12.sp),
            )
            return@GlanceColumn
        }

        GlanceText(
            note.title.ifBlank { "Untitled" },
            style = GlanceTextStyle(color = fg, fontSize = 15.sp, fontWeight = GlanceFontWeight.Bold),
            maxLines = 2,
        )

        if (note.checklist.isNotEmpty()) {
            note.checklist.forEachIndexed { index, item ->
                GlanceRow(
                    verticalAlignment = GlanceAlignment.CenterVertically,
                    // Tap a row to tick it: toggles in place while the vault is
                    // unlocked, opens the note otherwise.
                    modifier = GlanceModifier.clickable(
                        actionRunCallback<ToggleCheckAction>(
                            actionParametersOf(DOC_ID_KEY to note.id, ITEM_INDEX_KEY to index),
                        ),
                    ),
                ) {
                    GlanceText(
                        if (item.checked) "☑" else "☐",
                        style = GlanceTextStyle(color = accent, fontSize = 14.sp),
                    )
                    GlanceText(
                        item.text,
                        style = GlanceTextStyle(color = fg, fontSize = 13.sp),
                        maxLines = 2,
                        modifier = GlanceModifier.glancePadding(start = 6.dp),
                    )
                }
                GlanceSpacer(GlanceModifier.glanceHeight(2.dp))
            }
        } else if (note.text.isNotBlank()) {
            GlanceText(
                note.text,
                style = GlanceTextStyle(color = fg, fontSize = 13.sp),
                maxLines = 14,
            )
        }

        if (note.tags.isNotEmpty()) {
            GlanceRow(verticalAlignment = GlanceAlignment.CenterVertically) {
                note.tags.take(4).forEach { tag ->
                    GlanceText(
                        "#$tag",
                        style = GlanceTextStyle(color = muted, fontSize = 10.sp),
                        modifier = GlanceModifier.glancePadding(end = 6.dp),
                    )
                }
            }
        }
    }
}

/** Configuration: pick which shared note this widget instance shows. */
class PinnedNoteWidgetConfigureActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val widgetId = intent?.extras?.getInt(
            AppWidgetManager.EXTRA_APPWIDGET_ID,
            AppWidgetManager.INVALID_APPWIDGET_ID,
        ) ?: AppWidgetManager.INVALID_APPWIDGET_ID
        if (widgetId == AppWidgetManager.INVALID_APPWIDGET_ID) {
            finish()
            return
        }
        setContent {
            EnclaveTheme {
                val notes = remember { WidgetStore.notes(this) }
                ConfigureScreen(
                    notes = notes,
                    onPick = { note ->
                        lifecycleScope.launch {
                            val glanceId = AppWidgetId(widgetId)
                            updateAppWidgetState(this@PinnedNoteWidgetConfigureActivity, glanceId) { prefs ->
                                prefs[PinnedNoteWidget.KEY_DOC_ID] = note.id
                            }
                            PinnedNoteWidget().update(this@PinnedNoteWidgetConfigureActivity, glanceId)
                            setResult(RESULT_OK, Intent().putExtra(AppWidgetManager.EXTRA_APPWIDGET_ID, widgetId))
                            finish()
                        }
                    },
                    onCancel = {
                        setResult(RESULT_CANCELED)
                        finish()
                    },
                )
            }
        }
    }
}

/** Parameters for the widget checkbox action. */
internal val DOC_ID_KEY = ActionParameters.Key<String>("docId")
internal val ITEM_INDEX_KEY = ActionParameters.Key<Int>("itemIndex")

/**
 * Toggle one checklist item from a widget. Requires the vault unlocked in this
 * process (the widget otherwise just opens the note) — the write goes through
 * the same core API as the app, then the cache and widget re-render.
 */
class ToggleCheckAction : ActionCallback {
    override suspend fun onAction(
        context: Context,
        glanceId: GlanceId,
        parameters: ActionParameters,
    ) {
        val docId = parameters[DOC_ID_KEY] ?: return
        val index = parameters[ITEM_INDEX_KEY] ?: return
        try {
            val core = FfiCore(context.applicationInfo.dataDir)
            if (!core.isUnlocked()) {
                context.startActivity(openNoteIntent(context, docId))
                return
            }
            val blocks = core.getBlocks(docId)
            val docJson = blocks.firstOrNull { it.blockType == "doc" }?.contentJson ?: return
            val items = docChecklist(docJson).toMutableList()
            if (index !in items.indices) return
            items[index] = items[index].copy(checked = !items[index].checked)
            core.upsertBlock("$docId-content", docId, "doc", taskListDocJson(items), 0.0)
            WidgetStore.refreshAndUpdate(context, core)
            PinnedNoteWidget().update(context, glanceId)
        } catch (t: Throwable) {
            // Never let a widget interaction take the process down (a locked
            // vault or a mid-flight core just falls back to opening the app).
            android.util.Log.e("EnclaveWidgets", "widget toggle failed", t)
            context.startActivity(openNoteIntent(context, docId))
        }
    }
}

@Composable
private fun ConfigureScreen(
    notes: List<WidgetStore.WidgetNote>,
    onPick: (WidgetStore.WidgetNote) -> Unit,
    onCancel: () -> Unit,
) {
    Column(Modifier.fillMaxSize().padding(20.dp)) {
        Text("Pick a note", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Text(
            "Only notes with “Show in widgets” enabled appear here.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(12.dp))
        if (notes.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("No notes shared to widgets yet.", color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        } else {
            LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                items(notes, key = { it.id }) { note ->
                    Card(
                        onClick = { onPick(note) },
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Column(Modifier.padding(12.dp)) {
                            Text(note.title.ifBlank { "Untitled" }, fontWeight = FontWeight.SemiBold)
                            val preview = if (note.checklist.isNotEmpty()) {
                                note.checklist.take(2).joinToString("  ") { (if (it.checked) "☑ " else "☐ ") + it.text }
                            } else {
                                note.text.replace(Regex("\\s+"), " ").take(80)
                            }
                            if (preview.isNotBlank()) {
                                Text(
                                    preview,
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                }
            }
        }
        TextButton(onClick = onCancel) { Text("Cancel") }
    }
}
