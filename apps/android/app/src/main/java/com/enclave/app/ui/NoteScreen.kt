package com.enclave.app.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.enclave.app.core.Core
import com.enclave.app.editor.EditableDocument
import com.enclave.app.editor.EditorDocument
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
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

    LaunchedEffect(documentId) {
        withContext(Dispatchers.IO) {
            val document = Core.ffi.getDocument(documentId)
            val blocks = Core.ffi.getBlocks(documentId)
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

        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding),
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
