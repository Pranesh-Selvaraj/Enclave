package com.enclave.app.ui

import android.content.Intent
import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Create
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Face
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import com.enclave.app.WidgetCacheBridge
import com.enclave.app.core.Core
import com.enclave.app.editor.TemplateStore
import com.enclave.app.editor.EditableDocument
import com.enclave.app.editor.EditorDocument
import com.enclave.app.editor.EditableText
import com.enclave.app.editor.TextBlock
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.util.UUID

/**
 * A note: editable title and a block editor over the core's document contract.
 *
 * Edits write through `upsertBlock` on the canonical `doc` block after a short
 * debounce, so rev bumps and sync treat them like any other save. A note
 * created by the core still holds its placeholder block; the first edit
 * promotes that block id to `doc` instead of leaving a second block behind.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NoteScreen(documentId: String, onBack: () -> Unit) {
    var title by remember { mutableStateOf("") }
    var savedTitle by remember { mutableStateOf<String?>(null) }
    var updatedAt by remember { mutableStateOf("") }
    var isFavorite by remember { mutableStateOf(false) }
    var editor by remember { mutableStateOf<EditorDocument?>(null) }
    var docBlockId by remember { mutableStateOf<String?>(null) }
    var sortOrder by remember { mutableStateOf(1.0) }
    var loaded by remember { mutableStateOf(false) }
    var revision by remember { mutableIntStateOf(0) }
    var sheetOpen by remember { mutableStateOf(false) }
    var deleteOpen by remember { mutableStateOf(false) }
    var templateOpen by remember { mutableStateOf(false) }
    var iconOpen by remember { mutableStateOf(false) }
    var icon by remember { mutableStateOf<String?>(null) }
    var templateName by remember { mutableStateOf("") }
    var widgetEnabled by remember { mutableStateOf(false) }
    var widgetEditable by remember { mutableStateOf(true) }
    var widgetBlockId by remember { mutableStateOf<String?>(null) }
    var widgetSortOrder by remember { mutableStateOf(1.0) }
    val scope = rememberCoroutineScope()
    val context = LocalContext.current

    LaunchedEffect(documentId) {
        withContext(Dispatchers.IO) {
            val document = Core.ffi.getDocument(documentId)
            val blocks = Core.ffi.getBlocks(documentId)
            // Per-page widget policy: the `widget` block decides whether this
            // note may be shown in home-screen widgets and edited from them.
            val widgetBlock = blocks.firstOrNull { it.blockType == "widget" }
            widgetBlockId = widgetBlock?.id
            widgetSortOrder = widgetBlock?.sortOrder
                ?: ((blocks.maxOfOrNull { it.sortOrder } ?: 0.0) + 1.0)
            if (widgetBlock != null) {
                val widgetJson = runCatching { JSONObject(widgetBlock.contentJson) }.getOrNull()
                widgetEnabled = widgetJson?.optBoolean("enabled", false) ?: false
                widgetEditable = if (widgetEnabled) {
                    widgetJson?.optBoolean("editable", true) ?: true
                } else {
                    false
                }
            }
            val docBlock = blocks.firstOrNull { it.blockType == "doc" }
            if (docBlock != null) {
                docBlockId = docBlock.id
                sortOrder = docBlock.sortOrder
                editor = EditorDocument.parse(docBlock.contentJson)
            } else {
                val seed = blocks.firstOrNull()
                docBlockId = seed?.id ?: UUID.randomUUID().toString()
                sortOrder = seed?.sortOrder ?: 1.0
                editor = EditorDocument.empty()
            }
            icon = NoteIcon.load(documentId)
            title = document.title
            savedTitle = document.title
            updatedAt = document.updatedAt
            isFavorite = document.isFavorite
        }
        loaded = true
    }

    // Debounced title save.
    LaunchedEffect(title) {
        val baseline = savedTitle ?: return@LaunchedEffect
        if (title == baseline) return@LaunchedEffect
        delay(500)
        withContext(Dispatchers.IO) { Core.ffi.updateDocumentTitle(documentId, title) }
        WidgetCacheBridge.requestRefresh()
        savedTitle = title
    }

    // Debounced document save.
    LaunchedEffect(revision) {
        if (revision == 0) return@LaunchedEffect
        delay(800)
        val document = editor ?: return@LaunchedEffect
        val blockId = docBlockId ?: return@LaunchedEffect
        withContext(Dispatchers.IO) {
            Core.ffi.upsertBlock(blockId, documentId, "doc", document.toJson(), sortOrder)
        }
        WidgetCacheBridge.requestRefresh()
    }

    fun updateWidget(enabled: Boolean, editable: Boolean) {
        widgetEnabled = enabled
        widgetEditable = if (enabled) editable else false
        scope.launch {
            val id = widgetBlockId ?: UUID.randomUUID().toString()
            widgetBlockId = id
            val content = JSONObject()
                .put("enabled", enabled)
                .put("editable", if (enabled) editable else false)
                .toString()
            withContext(Dispatchers.IO) {
                Core.ffi.upsertBlock(id, documentId, "widget", content, widgetSortOrder)
            }
            WidgetCacheBridge.requestRefresh()
        }
    }

    fun shareNote() {
        val body = buildString {
            if (title.isNotBlank()) append(title).append("\n\n")
            // The core projects the document to text, so the share body and
            // the card previews can never drift apart.
            append(editor?.let { Core.ffi.previewText(it.toJson()) }.orEmpty())
        }.trim()
        if (body.isBlank()) return
        val send = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_SUBJECT, title)
            putExtra(Intent.EXTRA_TEXT, body)
        }
        context.startActivity(Intent.createChooser(send, "Send note"))
    }

    fun toggleFavorite() {
        scope.launch {
            runCatching {
                withContext(Dispatchers.IO) { Core.ffi.toggleFavorite(documentId) }
            }.onSuccess {
                isFavorite = it.isFavorite
                WidgetCacheBridge.requestRefresh()
            }
        }
    }

    fun duplicateNote() {
        scope.launch {
            sheetOpen = false
            runCatching {
                withContext(Dispatchers.IO) { Core.ffi.duplicateDocument(documentId) }
            }.onSuccess {
                Toast.makeText(context, "Copy created", Toast.LENGTH_SHORT).show()
            }.onFailure {
                Toast.makeText(context, "Could not copy the note", Toast.LENGTH_SHORT).show()
            }
        }
    }

    fun addBlock() {
        val document = editor ?: return
        document.nodes.add(TextBlock("paragraph", mutableMapOf(), EditableText()))
        revision++
        Toast.makeText(context, "Block added", Toast.LENGTH_SHORT).show()
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = {},
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = {
                    CircleAction(
                        icon = Icons.Filled.Star,
                        description = if (isFavorite) "Unpin note" else "Pin note",
                        tint = if (isFavorite) {
                            MaterialTheme.colorScheme.primary
                        } else {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        },
                        onClick = ::toggleFavorite,
                    )
                    CircleAction(
                        icon = Icons.Filled.Share,
                        description = "Send note",
                        onClick = ::shareNote,
                    )
                    CircleAction(
                        icon = Icons.Filled.MoreVert,
                        description = "Note options",
                        onClick = { sheetOpen = true },
                    )
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.background,
                ),
            )
        },
    ) { padding ->
        val document = editor
        if (!loaded || document == null) {
            Box(
                Modifier.fillMaxSize().padding(padding),
                contentAlignment = Alignment.Center,
            ) { CircularProgressIndicator() }
            return@Scaffold
        }

        val density = LocalDensity.current
        val fontScale = (density.fontScale * AppPrefs.editorTextSize.scale).coerceIn(0.8f, 3f)
        Box(
            modifier = Modifier.fillMaxSize().padding(padding),
            contentAlignment = Alignment.TopCenter,
        ) {
            CompositionLocalProvider(
                LocalDensity provides Density(density = density.density, fontScale = fontScale),
            ) {
                LazyColumn(
                    modifier = Modifier
                        .widthIn(max = AppPrefs.editorWidth.maxWidth)
                        .fillMaxHeight(),
                    contentPadding = PaddingValues(bottom = 16.dp),
                ) {
                    item {
                        BasicTextField(
                            value = title,
                            onValueChange = { title = it },
                            textStyle = MaterialTheme.typography.headlineMedium.copy(
                                color = MaterialTheme.colorScheme.onBackground,
                                fontWeight = FontWeight.SemiBold,
                            ),
                            cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                            modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp),
                            decorationBox = { inner ->
                                if (title.isBlank()) {
                                    Text(
                                        "Untitled note",
                                        style = MaterialTheme.typography.headlineMedium,
                                        fontWeight = FontWeight.SemiBold,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
                                    )
                                }
                                inner()
                            },
                        )
                    }

                    item {
                        EditableDocument(
                            doc = document,
                            onChanged = { revision++ },
                            modifier = Modifier.padding(horizontal = 20.dp, vertical = 4.dp),
                        )
                    }
                }
            }
        }
    }

    if (sheetOpen) {
        ModalBottomSheet(
            onDismissRequest = { sheetOpen = false },
            containerColor = MaterialTheme.colorScheme.surface,
        ) {
            Text(
                if (updatedAt.isBlank()) "Note options" else "Edited ${timeAgo(updatedAt)}",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
            )
            SheetAction(
                icon = Icons.Filled.Add,
                label = "Add block",
                checked = false,
            ) {
                sheetOpen = false
                addBlock()
            }
            SheetAction(
                icon = Icons.AutoMirrored.Filled.List,
                label = "Show in widgets",
                checked = widgetEnabled,
            ) {
                sheetOpen = false
                updateWidget(enabled = !widgetEnabled, editable = true)
            }
            SheetAction(
                icon = Icons.AutoMirrored.Filled.List,
                label = "Allow editing from widgets",
                enabled = widgetEnabled,
                checked = widgetEnabled && widgetEditable,
            ) {
                sheetOpen = false
                updateWidget(enabled = true, editable = !widgetEditable)
            }
            SheetAction(
                icon = Icons.Filled.Create,
                label = "Make a copy",
                onClick = ::duplicateNote,
            )
            SheetAction(
                icon = Icons.Filled.Face,
                label = icon?.let { "Icon: $it" } ?: "Set icon",
            ) {
                sheetOpen = false
                iconOpen = true
            }
            SheetAction(
                icon = Icons.Filled.Star,
                label = "Save as template",
            ) {
                sheetOpen = false
                templateName = title.trim()
                templateOpen = true
            }
            SheetAction(
                icon = Icons.Filled.Share,
                label = "Send",
            ) {
                sheetOpen = false
                shareNote()
            }
            SheetAction(
                icon = Icons.Filled.Delete,
                label = "Delete",
                tint = MaterialTheme.colorScheme.error,
            ) {
                sheetOpen = false
                deleteOpen = true
            }
            Spacer(Modifier.height(24.dp))
        }
    }

    if (iconOpen) {
        EmojiPickerDialog(
            current = icon,
            onPick = { picked ->
                iconOpen = false
                scope.launch {
                    NoteIcon.save(documentId, picked)
                    icon = picked.ifBlank { null }
                }
            },
            onDismiss = { iconOpen = false },
        )
    }

    if (templateOpen) {
        val nameFocus = remember { FocusRequester() }
        LaunchedEffect(templateOpen) { if (templateOpen) nameFocus.requestFocus() }
        AlertDialog(
            onDismissRequest = { templateOpen = false },
            title = { Text("Save as template") },
            text = {
                Column {
                    Text(
                        "The create stack will start a new note from this page's blocks. The note itself stays where it is.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.height(12.dp))
                    OutlinedTextField(
                        value = templateName,
                        onValueChange = { templateName = it },
                        singleLine = true,
                        label = { Text("Template name") },
                        modifier = Modifier.fillMaxWidth().focusRequester(nameFocus),
                    )
                }
            },
            confirmButton = {
                TextButton(
                    enabled = templateName.isNotBlank(),
                    onClick = {
                        val name = templateName.trim()
                        val json = editor?.toJson() ?: return@TextButton
                        templateOpen = false
                        scope.launch {
                            TemplateStore.add(name, json)
                            Toast.makeText(context, "Template saved", Toast.LENGTH_SHORT).show()
                        }
                    },
                ) { Text("Save") }
            },
            dismissButton = {
                TextButton(onClick = { templateOpen = false }) { Text("Cancel") }
            },
        )
    }

    if (deleteOpen) {
        AlertDialog(
            onDismissRequest = { deleteOpen = false },
            title = { Text("Delete note?") },
            text = {
                Text(
                    "This deletes “${title.ifBlank { "Untitled note" }}” from this vault.",
                    style = MaterialTheme.typography.bodyMedium,
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        deleteOpen = false
                        scope.launch {
                            val deleted = runCatching {
                                withContext(Dispatchers.IO) { Core.ffi.deleteDocument(documentId) }
                            }.isSuccess
                            if (deleted) {
                                WidgetCacheBridge.requestRefresh()
                                onBack()
                            } else {
                                Toast.makeText(context, "Could not delete the note", Toast.LENGTH_SHORT).show()
                            }
                        }
                    },
                ) {
                    Text("Delete", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { deleteOpen = false }) { Text("Cancel") }
            },
        )
    }
}

@Composable
private fun CircleAction(
    icon: ImageVector,
    description: String,
    tint: Color = MaterialTheme.colorScheme.onSurfaceVariant,
    onClick: () -> Unit,
) {
    Box(
        modifier = Modifier
            .padding(end = 6.dp)
            .size(40.dp)
            .clip(CircleShape)
            .background(MaterialTheme.colorScheme.surfaceVariant)
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(icon, contentDescription = description, tint = tint, modifier = Modifier.size(20.dp))
    }
}

@Composable
private fun SheetAction(
    icon: ImageVector,
    label: String,
    enabled: Boolean = true,
    checked: Boolean? = null,
    tint: Color = MaterialTheme.colorScheme.onSurfaceVariant,
    onClick: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(enabled = enabled, onClick = onClick)
            .padding(horizontal = 24.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            icon,
            contentDescription = null,
            tint = if (enabled) tint else tint.copy(alpha = 0.38f),
        )
        Spacer(Modifier.size(16.dp))
        Text(
            label,
            style = MaterialTheme.typography.bodyLarge,
            color = if (enabled) {
                MaterialTheme.colorScheme.onSurface
            } else {
                MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f)
            },
            modifier = Modifier.weight(1f),
        )
        if (checked == true) {
            Icon(Icons.Filled.Check, contentDescription = null, tint = MaterialTheme.colorScheme.primary)
        }
    }
}
