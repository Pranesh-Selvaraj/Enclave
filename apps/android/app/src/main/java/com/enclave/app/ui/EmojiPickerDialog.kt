package com.enclave.app.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.runtime.LaunchedEffect

/**
 * Pick the emoji a note shows in the Icons layout: a field for any emoji the
 * keyboard can produce (the phone's own emoji picker is the real browser), plus
 * a few common ones for a single tap. "Remove" clears the icon.
 */
@Composable
fun EmojiPickerDialog(
    current: String?,
    onPick: (String) -> Unit,
    onDismiss: () -> Unit,
) {
    var custom by remember { mutableStateOf(current.orEmpty()) }
    val focus = remember { FocusRequester() }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Note icon") },
        text = {
            Column {
                Text(
                    "Shown in the Icons layout.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Spacer(Modifier.height(12.dp))
                Surface(
                    color = MaterialTheme.colorScheme.surfaceVariant,
                    shape = CircleShape,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    BasicTextField(
                        value = custom,
                        onValueChange = { custom = it.take(8) },
                        singleLine = true,
                        textStyle = MaterialTheme.typography.headlineSmall.copy(
                            color = MaterialTheme.colorScheme.onSurface,
                            textAlign = TextAlign.Center,
                        ),
                        cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 10.dp)
                            .focusRequester(focus),
                        decorationBox = { inner ->
                            Box(contentAlignment = Alignment.Center) {
                                if (custom.isEmpty()) {
                                    Text(
                                        "Type or paste an emoji",
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                }
                                inner()
                            }
                        },
                    )
                }
                Spacer(Modifier.height(14.dp))
                // Four rows of nine: enough choice to feel like a picker, few
                // enough to fit without scrolling inside a dialog.
                PresetEmojis.chunked(9).forEach { row ->
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(vertical = 2.dp),
                        horizontalArrangement = Arrangement.SpaceEvenly,
                    ) {
                        row.forEach { emoji ->
                            Box(
                                modifier = Modifier
                                    .size(36.dp)
                                    .clickable { custom = emoji },
                                contentAlignment = Alignment.Center,
                            ) {
                                Text(emoji, fontSize = 22.sp)
                            }
                        }
                    }
                }
                LaunchedEffect(Unit) { focus.requestFocus() }
            }
        },
        confirmButton = {
            TextButton(
                enabled = custom.isNotBlank(),
                onClick = { onPick(custom.trim()) },
            ) { Text("Save") }
        },
        dismissButton = {
            Row {
                if (current != null) {
                    TextButton(onClick = { onPick("") }) {
                        Text("Remove", color = MaterialTheme.colorScheme.error)
                    }
                }
                TextButton(onClick = onDismiss) { Text("Cancel") }
            }
        },
    )
}

/** A small, varied default set — the field above accepts anything else. */
private val PresetEmojis = listOf(
    "📝", "✅", "📋", "📌", "💡", "🎯", "🚀", "🔥", "⭐",
    "📚", "🗂️", "🧩", "🎵", "🎨", "🧪", "🏠", "💼", "🛠️",
    "🌱", "☕", "🍀", "📅", "✈️", "❤️", "🧠", "🔒", "📈",
)
