package com.enclave.app

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.outlined.Star
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import uniffi.core_api.Document
import uniffi.core_api.EnclaveException
import uniffi.core_api.FfiCore

/**
 * Keep-like native shell — Phase 1 of the Android-native plan.
 *
 * Vault gate (create / password / recovery phrase) → note list → note detail,
 * all talking to the shared Rust core through the UniFFI bindings. The full
 * block editor stays in the WebView editor island (next slice); this screen is
 * for the 90% mobile flow: read, capture, check off, tag, pin.
 */
class KeepActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // JNA finds libcore_api.so through the app's native library dir.
        System.setProperty("jna.library.path", applicationInfo.nativeLibraryDir)
        setContent { EnclaveTheme { KeepApp(filesDir.absolutePath) } }
    }
}

@Composable
private fun EnclaveTheme(content: @Composable () -> Unit) {
    val dark = isSystemInDarkTheme()
    val scheme = if (dark) {
        darkColorScheme(
            primary = Color(0xFF8B7CF6),
            background = Color(0xFF16130F),
            surface = Color(0xFF201C17),
            onBackground = Color(0xFFECE7DF),
            onSurface = Color(0xFFECE7DF),
        )
    } else {
        lightColorScheme(
            primary = Color(0xFF6B5CE7),
            background = Color(0xFFF1EDE5),
            surface = Color(0xFFFAF7F1),
            onBackground = Color(0xFF211D17),
            onSurface = Color(0xFF211D17),
        )
    }
    MaterialTheme(colorScheme = scheme, content = content)
}

// ── App state / navigation ──────────────────────────────────────────────────

private sealed interface Stage {
    data object Loading : Stage
    data object CreateVault : Stage
    data class ShowPhrase(val phrase: String) : Stage
    data object Unlock : Stage
    data object Notes : Stage
}

private data class NoteRow(val doc: Document, val snippet: String, val tags: List<String>)

@Composable
private fun KeepApp(appDir: String) {
    val scope = rememberCoroutineScope()
    val core = remember { FfiCore(appDir) }
    var stage by remember { mutableStateOf<Stage>(Stage.Loading) }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        stage = withContext(Dispatchers.IO) {
            if (core.isVaultInitialized()) Stage.Unlock else Stage.CreateVault
        }
    }

    when (val s = stage) {
        Stage.Loading -> Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            CircularProgressIndicator()
        }

        Stage.CreateVault -> CreateVaultScreen(error) { password ->
            scope.launch {
                error = null
                try {
                    // Argon2id: always off the main thread.
                    val phrase = withContext(Dispatchers.IO) { core.createVault(password) }
                    stage = Stage.ShowPhrase(phrase)
                } catch (e: EnclaveException) {
                    error = e.message
                }
            }
        }

        is Stage.ShowPhrase -> ShowPhraseScreen(s.phrase) { stage = Stage.Notes }

        Stage.Unlock -> UnlockScreen(error) { password, mnemonic ->
            scope.launch {
                error = null
                try {
                    withContext(Dispatchers.IO) {
                        if (mnemonic != null) core.unlockWithMnemonic(mnemonic)
                        else core.unlockWithPassword(password!!)
                    }
                    stage = Stage.Notes
                } catch (e: EnclaveException) {
                    error = e.message
                }
            }
        }

        Stage.Notes -> NotesScreen(
            core = core,
            onLock = {
                scope.launch {
                    withContext(Dispatchers.IO) { core.lockVault() }
                    error = null
                    stage = Stage.Unlock
                }
            },
        )
    }
}

// ── Vault gate ──────────────────────────────────────────────────────────────

@Composable
private fun CreateVaultScreen(error: String?, onCreate: (String) -> Unit) {
    var password by remember { mutableStateOf("") }
    var confirm by remember { mutableStateOf("") }
    val valid = password.length >= 4 && password == confirm

    Column(
        Modifier.fillMaxSize().padding(28.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text("Welcome to Enclave", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Spacer(Modifier.height(8.dp))
        Text(
            "Your notes are encrypted and live only on this device.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(24.dp))
        OutlinedTextField(
            value = password,
            onValueChange = { password = it },
            label = { Text("Vault password") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
            modifier = Modifier.fillMaxWidth(),
        )
        Spacer(Modifier.height(10.dp))
        OutlinedTextField(
            value = confirm,
            onValueChange = { confirm = it },
            label = { Text("Confirm password") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
            modifier = Modifier.fillMaxWidth(),
        )
        if (error != null) {
            Spacer(Modifier.height(10.dp))
            Text(error, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
        }
        Spacer(Modifier.height(18.dp))
        Button(onClick = { onCreate(password) }, enabled = valid, modifier = Modifier.fillMaxWidth()) {
            Text("Create vault")
        }
    }
}

@Composable
private fun ShowPhraseScreen(phrase: String, onDone: () -> Unit) {
    var saved by remember { mutableStateOf(false) }
    Column(
        Modifier.fillMaxSize().padding(28.dp),
        verticalArrangement = Arrangement.Center,
    ) {
        Text("Save your recovery phrase", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
        Spacer(Modifier.height(8.dp))
        Text(
            "These 12 words are the only way back in if you forget your password.",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(16.dp))
        Card(colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)) {
            Text(
                phrase,
                style = MaterialTheme.typography.bodyLarge,
                modifier = Modifier.padding(16.dp),
            )
        }
        Spacer(Modifier.height(16.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            androidx.compose.material3.Checkbox(checked = saved, onCheckedChange = { saved = it })
            Text("I saved my recovery phrase", style = MaterialTheme.typography.bodyMedium)
        }
        Spacer(Modifier.height(8.dp))
        Button(onClick = onDone, enabled = saved, modifier = Modifier.fillMaxWidth()) { Text("Start writing") }
    }
}

@Composable
private fun UnlockScreen(error: String?, onUnlock: (String?, String?) -> Unit) {
    var password by remember { mutableStateOf("") }
    var mnemonic by remember { mutableStateOf("") }
    var usePhrase by remember { mutableStateOf(false) }

    Column(
        Modifier.fillMaxSize().padding(28.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text("Enclave", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
        Spacer(Modifier.height(6.dp))
        Text(
            if (usePhrase) "Enter your 12-word recovery phrase" else "Unlock your vault",
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(Modifier.height(22.dp))
        if (usePhrase) {
            OutlinedTextField(
                value = mnemonic,
                onValueChange = { mnemonic = it },
                label = { Text("Recovery phrase") },
                minLines = 3,
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(12.dp))
            Button(
                onClick = { onUnlock(null, mnemonic.trim()) },
                enabled = mnemonic.trim().split(" ").size >= 12,
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Unlock") }
        } else {
            OutlinedTextField(
                value = password,
                onValueChange = { password = it },
                label = { Text("Password") },
                singleLine = true,
                visualTransformation = PasswordVisualTransformation(),
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password, imeAction = ImeAction.Done),
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(12.dp))
            Button(
                onClick = { onUnlock(password, null) },
                enabled = password.isNotEmpty(),
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Unlock") }
        }
        if (error != null) {
            Spacer(Modifier.height(10.dp))
            Text(error, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
        }
        Spacer(Modifier.height(6.dp))
        TextButton(onClick = { usePhrase = !usePhrase }) {
            Text(if (usePhrase) "Back to password" else "Forgot password? Use recovery phrase")
        }
    }
}

// ── Note list ───────────────────────────────────────────────────────────────

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun NotesScreen(core: FfiCore, onLock: () -> Unit) {
    val scope = rememberCoroutineScope()
    var notes by remember { mutableStateOf<List<NoteRow>>(emptyList()) }
    var query by remember { mutableStateOf("") }
    var openDoc by remember { mutableStateOf<Document?>(null) }
    var loading by remember { mutableStateOf(true) }

    suspend fun reload() {
        val rows = withContext(Dispatchers.IO) {
            val tags = core.getAllTags().associate { it.docId to it.tags }
            core.listDocuments().map { doc ->
                NoteRow(doc, snippetOf(core, doc.id), tags[doc.id] ?: emptyList())
            }
        }
        notes = rows
        loading = false
    }

    LaunchedEffect(Unit) { reload() }

    val doc = openDoc
    if (doc != null) {
        NoteDetailScreen(
            core = core,
            docId = doc.id,
            onBack = { openDoc = null; scope.launch { reload() } },
        )
        return
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Enclave", fontWeight = FontWeight.Bold) },
                actions = {
                    IconButton(onClick = onLock) { Icon(Icons.Default.Lock, contentDescription = "Lock vault") }
                },
            )
        },
        floatingActionButton = {
            ExtendedFloatingActionButton(
                onClick = {
                    scope.launch {
                        val newDoc = withContext(Dispatchers.IO) { core.createDocument("Untitled") }
                        reload()
                        openDoc = newDoc
                    }
                },
                icon = { Icon(Icons.Default.Add, contentDescription = null) },
                text = { Text("New note") },
            )
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            OutlinedTextField(
                value = query,
                onValueChange = { query = it },
                placeholder = { Text("Search notes…") },
                leadingIcon = { Icon(Icons.Default.Search, contentDescription = null) },
                singleLine = true,
                modifier = Modifier.fillMaxWidth().padding(horizontal = 14.dp, vertical = 6.dp),
            )

            if (loading) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { CircularProgressIndicator() }
                return@Column
            }

            val shown = if (query.isBlank()) notes else notes.filter {
                it.doc.title.contains(query, ignoreCase = true) || it.snippet.contains(query, ignoreCase = true)
            }

            if (shown.isEmpty()) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(
                        if (query.isBlank()) "No notes yet — tap New note." else "No matches.",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                return@Column
            }

            LazyVerticalStaggeredGrid(
                columns = StaggeredGridCells.Fixed(2),
                contentPadding = androidx.compose.foundation.layout.PaddingValues(10.dp, 6.dp, 10.dp, 96.dp),
                verticalItemSpacing = 10.dp,
                horizontalArrangement = Arrangement.spacedBy(10.dp),
                modifier = Modifier.fillMaxSize(),
            ) {
                items(shown, key = { it.doc.id }) { row ->
                    NoteCard(
                        row = row,
                        onOpen = { openDoc = row.doc },
                        onToggleFavorite = {
                            scope.launch {
                                withContext(Dispatchers.IO) { core.toggleFavorite(row.doc.id) }
                                reload()
                            }
                        },
                    )
                }
            }
        }
    }
}

@Composable
private fun NoteCard(row: NoteRow, onOpen: () -> Unit, onToggleFavorite: () -> Unit) {
    Card(
        onClick = onOpen,
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    row.doc.title.ifBlank { "Untitled" },
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
                IconButton(onClick = onToggleFavorite, modifier = Modifier.size(28.dp)) {
                    Icon(
                        if (row.doc.isFavorite) Icons.Filled.Star else Icons.Outlined.Star,
                        contentDescription = "Favorite",
                        tint = if (row.doc.isFavorite) Color(0xFFD9A851) else MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.size(18.dp),
                    )
                }
            }
            if (row.snippet.isNotBlank()) {
                Spacer(Modifier.height(4.dp))
                Text(
                    row.snippet,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 6,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            if (row.tags.isNotEmpty()) {
                Spacer(Modifier.height(8.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    row.tags.take(2).forEach { tag ->
                        AssistChip(onClick = {}, label = { Text("#$tag", style = MaterialTheme.typography.labelSmall) })
                    }
                }
            }
        }
    }
}

// ── Note detail (flat Keep-style editor; full block editor is the WebView) ──

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun NoteDetailScreen(core: FfiCore, docId: String, onBack: () -> Unit) {
    val scope = rememberCoroutineScope()
    var title by remember { mutableStateOf("") }
    var body by remember { mutableStateOf("") }
    var favorite by remember { mutableStateOf(false) }
    var loaded by remember { mutableStateOf(false) }
    var dirty by remember { mutableStateOf(false) }

    LaunchedEffect(docId) {
        val (doc, text) = withContext(Dispatchers.IO) {
            core.getDocument(docId) to bodyOf(core, docId)
        }
        title = doc.title
        favorite = doc.isFavorite
        body = text
        loaded = true
    }

    // Debounced autosave (title + body) after edits.
    LaunchedEffect(title, body) {
        if (!loaded || !dirty) return@LaunchedEffect
        delay(700)
        withContext(Dispatchers.IO) {
            core.updateDocumentTitle(docId, title)
            core.upsertBlock("$docId-content", docId, "doc", docJson(body), 0.0)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                navigationIcon = {
                    IconButton(onClick = onBack) { Icon(Icons.Default.ArrowBack, contentDescription = "Back") }
                },
                title = {},
                actions = {
                    IconButton(onClick = {
                        favorite = !favorite
                        scope.launch { withContext(Dispatchers.IO) { core.toggleFavorite(docId) } }
                    }) {
                        Icon(
                            if (favorite) Icons.Filled.Star else Icons.Outlined.Star,
                            contentDescription = "Favorite",
                            tint = if (favorite) Color(0xFFD9A851) else MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    IconButton(onClick = {
                        scope.launch {
                            withContext(Dispatchers.IO) { core.archiveDocument(docId) }
                            onBack()
                        }
                    }) { Icon(Icons.Default.Delete, contentDescription = "Move to trash") }
                },
            )
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding).padding(horizontal = 16.dp)) {
            OutlinedTextField(
                value = title,
                onValueChange = { title = it; dirty = true },
                placeholder = { Text("Title") },
                textStyle = MaterialTheme.typography.titleLarge,
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(10.dp))
            OutlinedTextField(
                value = body,
                onValueChange = { body = it; dirty = true },
                placeholder = { Text("Start typing…") },
                minLines = 12,
                modifier = Modifier.fillMaxWidth().weight(1f),
            )
        }
    }
}

// ── Block JSON helpers (the same document format the web editor uses) ───────

/** Plain-text snippet from the document's `doc` block. */
private fun snippetOf(core: FfiCore, docId: String): String {
    return try {
        val blocks = core.getBlocks(docId)
        val doc = blocks.firstOrNull { it.blockType == "doc" } ?: return ""
        jsonText(JSONObject(doc.contentJson)).trim().replace(Regex("\\s+"), " ")
    } catch (_: Exception) {
        ""
    }
}

/** Full body text (paragraphs joined by blank lines) for the flat editor. */
private fun bodyOf(core: FfiCore, docId: String): String {
    return try {
        val blocks = core.getBlocks(docId)
        val doc = blocks.firstOrNull { it.blockType == "doc" } ?: return ""
        val root = JSONObject(doc.contentJson)
        val out = StringBuilder()
        root.optJSONArray("content")?.let { nodes ->
            for (i in 0 until nodes.length()) {
                val node = nodes.optJSONObject(i) ?: continue
                val text = jsonText(node).trim()
                if (text.isNotEmpty()) {
                    if (out.isNotEmpty()) out.append("\n\n")
                    out.append(text)
                }
            }
        }
        out.toString()
    } catch (_: Exception) {
        ""
    }
}

/** Recursively collect `text` leaves from a TipTap JSON node. */
private fun jsonText(node: JSONObject, out: StringBuilder = StringBuilder()): String {
    when (node.optString("type")) {
        "text" -> out.append(node.optString("text"))
        "hardBreak" -> out.append("\n")
    }
    node.optJSONArray("content")?.let { children ->
        for (i in 0 until children.length()) {
            (children.opt(i) as? JSONObject)?.let { jsonText(it, out) }
            if (children.opt(i) is String) out.append(children.optString(i))
        }
    }
    return out.toString()
}

/** Flat text → the same doc JSON shape the web editor writes. */
private fun docJson(text: String): String {
    val paragraphs = text.split("\n\n").map { paragraph ->
        JSONObject().apply {
            put("type", "paragraph")
            val lines = paragraph.split("\n")
            val content = JSONArray()
            lines.forEachIndexed { i, line ->
                if (i > 0) content.put(JSONObject().put("type", "hardBreak"))
                if (line.isNotEmpty()) {
                    content.put(JSONObject().put("type", "text").put("text", line))
                }
            }
            put("content", content)
        }
    }
    return JSONObject()
        .put("type", "doc")
        .put("content", JSONArray(paragraphs))
        .toString()
}
