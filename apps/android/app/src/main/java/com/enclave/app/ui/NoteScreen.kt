package com.enclave.app.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
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
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.dp
import com.enclave.app.WidgetCacheBridge
import com.enclave.app.core.Core
import com.enclave.app.editor.EditableDocument
import com.enclave.app.editor.EditorDocument
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
    var editor by remember { mutableStateOf<EditorDocument?>(null) }
    var docBlockId by remember { mutableStateOf<String?>(null) }
    var sortOrder by remember { mutableStateOf(1.0) }
    var loaded by remember { mutableStateOf(false) }
    var revision by remember { mutableIntStateOf(0) }
    var menuOpen by remember { mutableStateOf(false) }
    var widgetEnabled by remember { mutableStateOf(false) }
    var widgetEditable by remember { mutableStateOf(true) }
    var widgetBlockId by remember { mutableStateOf<String?>(null) }
    var widgetSortOrder by remember { mutableStateOf(1.0) }
    val scope = rememberCoroutineScope()

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
                widgetEditable = widgetJson?.optBoolean("editable", true) ?: true
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
            title = document.title
            savedTitle = document.title
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
                    IconButton(onClick = { menuOpen = true }) {
                        Icon(Icons.Filled.MoreVert, contentDescription = "Note options")
                    }
                    DropdownMenu(
                        expanded = menuOpen,
                        onDismissRequest = { menuOpen = false },
                    ) {
                        DropdownMenuItem(
                            text = { Text("Show in widgets") },
                            trailingIcon = {
                                if (widgetEnabled) {
                                    Icon(Icons.Filled.Check, contentDescription = null)
                                }
                            },
                            onClick = {
                                menuOpen = false
                                updateWidget(
                                    enabled = !widgetEnabled,
                                    editable = if (!widgetEnabled) true else widgetEditable,
                                )
                            },
                        )
                        DropdownMenuItem(
                            text = { Text("Allow editing from widgets") },
                            enabled = widgetEnabled,
                            trailingIcon = {
                                if (widgetEditable) {
                                    Icon(Icons.Filled.Check, contentDescription = null)
                                }
                            },
                            onClick = {
                                menuOpen = false
                                updateWidget(enabled = true, editable = !widgetEditable)
                            },
                        )
                    }
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
                    contentPadding = PaddingValues(bottom = 64.dp),
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
}
