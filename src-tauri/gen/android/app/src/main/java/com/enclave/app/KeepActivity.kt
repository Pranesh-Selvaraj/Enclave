package com.enclave.app

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items as lazyRowItems
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.items
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.List
import androidx.compose.material.icons.outlined.Star
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExtendedFloatingActionButton
import androidx.compose.material3.FilterChip
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
import androidx.compose.runtime.MutableState
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
 * Vault gate (create / password / recovery phrase) → note list → note detail.
 * Supports text notes and checklists, labels (tags), pin/archive, search and
 * the Android share target. The full block editor (whiteboards, tables,
 * databases) stays in the WebView editor island.
 */
class KeepActivity : ComponentActivity() {
    /** Text shared from another app, consumed once the vault is unlocked. */
    private val pendingShare = mutableStateOf<String?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // JNA finds libcore_api.so through the app's native library dir.
        System.setProperty("jna.library.path", applicationInfo.nativeLibraryDir)
        pendingShare.value = extractSharedText(intent)
        setContent { EnclaveTheme { KeepApp(filesDir.absolutePath, pendingShare) } }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        extractSharedText(intent)?.let { pendingShare.value = it }
    }

    private fun extractSharedText(intent: Intent?): String? {
        if (intent?.action != Intent.ACTION_SEND) return null
        @Suppress("DEPRECATION")
        return intent.getStringExtra(Intent.EXTRA_TEXT)?.trim()?.takeIf { it.isNotEmpty() }
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
private data class TaskItem(val text: String, val checked: Boolean)

@Composable
private fun KeepApp(appDir: String, pendingShare: MutableState<String?>) {
    val scope = rememberCoroutineScope()
    val core = remember { FfiCore(appDir) }
    var stage by remember { mutableStateOf<Stage>(Stage.Loading) }
    var error by remember { mutableStateOf<String?>(null) }
    var openDocId by remember { mutableStateOf<String?>(null) }
    var refreshKey by remember { mutableStateOf(0) }

    LaunchedEffect(Unit) {
        stage = withContext(Dispatchers.IO) {
            if (core.isVaultInitialized()) Stage.Unlock else Stage.CreateVault
        }
    }

    // Shared text becomes a note as soon as the vault is open (share target).
    LaunchedEffect(stage, pendingShare.value) {
        val shared = pendingShare.value
        if (stage is Stage.Notes && shared != null) {
            pendingShare.value = null
            val doc = withContext(Dispatchers.IO) {
                val title = shared.lineSequence().firstOrNull()?.take(80)?.ifBlank { null } ?: "Shared note"
                val created = core.createDocument(title)
                core.upsertBlock("${created.id}-content", created.id, "doc", docJson(shared), 0.0)
                created
            }
            refreshKey++
            openDocId = doc.id
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
            refreshKey = refreshKey,
            openDocId = openDocId,
            onOpen = { openDocId = it },
            onCloseDetail = {
                openDocId = null
                refreshKey++
            },
            onLock = {
                scope.launch {
                    withContext(Dispatchers.IO) { core.lockVault() }
                    error = null
                    openDocId = null
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
            Text(phrase, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.padding(16.dp))
        }
        Spacer(Modifier.height(16.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Checkbox(checked = saved, onCheckedChange = { saved = it })
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
private fun NotesScreen(
    core: FfiCore,
    refreshKey: Int,
    openDocId: String?,
    onOpen: (String) -> Unit,
    onCloseDetail: () -> Unit,
    onLock: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    var notes by remember { mutableStateOf<List<NoteRow>>(emptyList()) }
    var allTags by remember { mutableStateOf<List<String>>(emptyList()) }
    var selectedTag by remember { mutableStateOf<String?>(null) }
    var query by remember { mutableStateOf("") }
    var loading by remember { mutableStateOf(true) }

    suspend fun reload() {
        val (rows, tags) = withContext(Dispatchers.IO) {
            val tagRows = core.getAllTags()
            val byDoc = tagRows.associate { it.docId to it.tags }
            val docs = core.listDocuments().map { doc ->
                NoteRow(doc, snippetOf(core, doc.id), byDoc[doc.id] ?: emptyList())
            }
            docs to tagRows.flatMap { it.tags }.distinct().sorted()
        }
        notes = rows
        allTags = tags
        loading = false
    }

    LaunchedEffect(refreshKey) { reload() }

    if (openDocId != null) {
        NoteDetailScreen(
            core = core,
            docId = openDocId,
            onBack = onCloseDetail,
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
                        onOpen(newDoc.id)
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

            if (allTags.isNotEmpty()) {
                LazyRow(
                    contentPadding = PaddingValues(horizontal = 14.dp),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    item {
                        FilterChip(
                            selected = selectedTag == null,
                            onClick = { selectedTag = null },
                            label = { Text("All") },
                        )
                    }
                    lazyRowItems(allTags, key = { it }) { tag ->
                        FilterChip(
                            selected = selectedTag == tag,
                            onClick = { selectedTag = if (selectedTag == tag) null else tag },
                            label = { Text("#$tag") },
                        )
                    }
                }
            }

            if (loading) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { CircularProgressIndicator() }
                return@Column
            }

            var shown = notes
            selectedTag?.let { tag -> shown = shown.filter { it.tags.contains(tag) } }
            if (query.isNotBlank()) {
                shown = shown.filter {
                    it.doc.title.contains(query, ignoreCase = true) || it.snippet.contains(query, ignoreCase = true)
                }
            }

            if (shown.isEmpty()) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(
                        if (notes.isEmpty()) "No notes yet — tap New note." else "No matches.",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                return@Column
            }

            LazyVerticalStaggeredGrid(
                columns = StaggeredGridCells.Fixed(2),
                contentPadding = PaddingValues(10.dp, 6.dp, 10.dp, 96.dp),
                verticalItemSpacing = 10.dp,
                horizontalArrangement = Arrangement.spacedBy(10.dp),
                modifier = Modifier.fillMaxSize(),
            ) {
                items(shown, key = { it.doc.id }) { row ->
                    NoteCard(
                        row = row,
                        onOpen = { onOpen(row.doc.id) },
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

// ── Note detail: text or checklist, tags, favorite, archive ────────────────

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun NoteDetailScreen(core: FfiCore, docId: String, onBack: () -> Unit) {
    val scope = rememberCoroutineScope()
    var title by remember { mutableStateOf("") }
    var body by remember { mutableStateOf("") }
    var tasks by remember { mutableStateOf<List<TaskItem>>(emptyList()) }
    var checklist by remember { mutableStateOf(false) }
    var tags by remember { mutableStateOf<List<String>>(emptyList()) }
    var tagInput by remember { mutableStateOf("") }
    var favorite by remember { mutableStateOf(false) }
    var loaded by remember { mutableStateOf(false) }
    var dirty by remember { mutableStateOf(false) }

    LaunchedEffect(docId) {
        val loadedState = withContext(Dispatchers.IO) {
            val doc = core.getDocument(docId)
            val docTasks = tasksOf(core, docId)
            val docTags = core.getAllTags().firstOrNull { it.docId == docId }?.tags ?: emptyList()
            listOf(doc.title, bodyOf(core, docId), docTasks, docTags, doc.isFavorite)
        }
        title = loadedState[0] as String
        body = loadedState[1] as String
        tasks = loadedState[2] as List<TaskItem>
        tags = loadedState[3] as List<String>
        favorite = loadedState[4] as Boolean
        checklist = tasks.isNotEmpty()
        loaded = true
    }

    fun saveTags(next: List<String>) {
        tags = next
        scope.launch {
            withContext(Dispatchers.IO) {
                core.upsertBlock(
                    "$docId-tags",
                    docId,
                    "tags",
                    JSONObject().put("tags", JSONArray(next)).toString(),
                    2.0,
                )
            }
        }
    }

    // Debounced autosave after edits.
    LaunchedEffect(title, body, tasks) {
        if (!loaded || !dirty) return@LaunchedEffect
        delay(700)
        withContext(Dispatchers.IO) {
            core.updateDocumentTitle(docId, title)
            when {
                checklist -> core.upsertBlock("$docId-checklist", docId, "taskList", taskJson(tasks), 1.0)
                else -> core.upsertBlock("$docId-content", docId, "doc", docJson(body), 0.0)
            }
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
                        // Switch text ↔ checklist, converting the content.
                        if (checklist) {
                            body = tasks.joinToString("\n\n") { it.text }
                            checklist = false
                        } else {
                            tasks = body.split("\n").filter { it.isNotBlank() }.map { TaskItem(it, false) }
                            checklist = true
                        }
                        dirty = true
                    }) {
                        Icon(
                            if (checklist) Icons.Default.Edit else Icons.Default.List,
                            contentDescription = if (checklist) "Text note" else "Checklist",
                        )
                    }
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

            Spacer(Modifier.height(8.dp))
            if (checklist) {
                Box(Modifier.fillMaxWidth().weight(1f)) {
                    ChecklistEditor(tasks, onChange = { tasks = it; dirty = true })
                }
            } else {
                OutlinedTextField(
                    value = body,
                    onValueChange = { body = it; dirty = true },
                    placeholder = { Text("Start typing…") },
                    minLines = 10,
                    modifier = Modifier.fillMaxWidth().weight(1f),
                )
            }

            Spacer(Modifier.height(10.dp))
            TagEditor(
                tags = tags,
                input = tagInput,
                onInput = { tagInput = it },
                onAdd = {
                    val t = tagInput.trim().removePrefix("#")
                    if (t.isNotEmpty() && !tags.contains(t)) saveTags(tags + t)
                    tagInput = ""
                },
                onRemove = { saveTags(tags - it) },
            )
        }
    }
}

@Composable
private fun ChecklistEditor(tasks: List<TaskItem>, onChange: (List<TaskItem>) -> Unit) {
    var draft by remember { mutableStateOf("") }
    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()),
    ) {
        tasks.forEachIndexed { index, item ->
            Row(verticalAlignment = Alignment.CenterVertically) {
                Checkbox(
                    checked = item.checked,
                    onCheckedChange = { checked ->
                        onChange(tasks.toMutableList().also { it[index] = item.copy(checked = checked) })
                    },
                )
                Text(item.text, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
                IconButton(onClick = {
                    onChange(tasks.toMutableList().also { it.removeAt(index) })
                }) { Icon(Icons.Default.Delete, contentDescription = "Remove item") }
            }
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            OutlinedTextField(
                value = draft,
                onValueChange = { draft = it },
                placeholder = { Text("Add item…") },
                singleLine = true,
                modifier = Modifier.weight(1f),
            )
            Spacer(Modifier.width(6.dp))
            IconButton(
                onClick = {
                    if (draft.isNotBlank()) {
                        onChange(tasks + TaskItem(draft.trim(), false))
                        draft = ""
                    }
                },
                enabled = draft.isNotBlank(),
            ) { Icon(Icons.Default.Add, contentDescription = "Add item") }
        }
        Spacer(Modifier.height(8.dp))
    }
}

@Composable
private fun TagEditor(
    tags: List<String>,
    input: String,
    onInput: (String) -> Unit,
    onAdd: () -> Unit,
    onRemove: (String) -> Unit,
) {
    Column(Modifier.fillMaxWidth()) {
        if (tags.isNotEmpty()) {
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                tags.forEach { tag ->
                    AssistChip(
                        onClick = { onRemove(tag) },
                        label = { Text("#$tag  ✕") },
                    )
                }
            }
            Spacer(Modifier.height(6.dp))
        }
        OutlinedTextField(
            value = input,
            onValueChange = onInput,
            label = { Text("Add label") },
            singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
            modifier = Modifier.fillMaxWidth(),
        )
        TextButton(onClick = onAdd, enabled = input.isNotBlank()) { Text("Add") }
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

/** Checklist items from the document's `taskList` block. */
private fun tasksOf(core: FfiCore, docId: String): List<TaskItem> {
    return try {
        val block = core.getBlocks(docId).firstOrNull { it.blockType == "taskList" } ?: return emptyList()
        val content = JSONObject(block.contentJson).optJSONArray("content") ?: return emptyList()
        (0 until content.length()).mapNotNull { i ->
            val item = content.optJSONObject(i) ?: return@mapNotNull null
            if (item.optString("type") != "taskItem") return@mapNotNull null
            TaskItem(
                text = jsonText(item).trim(),
                checked = item.optJSONObject("attrs")?.optBoolean("checked", false) ?: false,
            )
        }
    } catch (_: Exception) {
        emptyList()
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
            val content = JSONArray()
            paragraph.split("\n").forEachIndexed { i, line ->
                if (i > 0) content.put(JSONObject().put("type", "hardBreak"))
                if (line.isNotEmpty()) content.put(JSONObject().put("type", "text").put("text", line))
            }
            put("content", content)
        }
    }
    return JSONObject().put("type", "doc").put("content", JSONArray(paragraphs)).toString()
}

/** Checklist items → TipTap `taskList` JSON (what the web editor renders). */
private fun taskJson(tasks: List<TaskItem>): String {
    val content = JSONArray()
    tasks.forEach { item ->
        content.put(
            JSONObject()
                .put("type", "taskItem")
                .put("attrs", JSONObject().put("checked", item.checked))
                .put(
                    "content",
                    JSONArray().put(
                        JSONObject()
                            .put("type", "paragraph")
                            .put(
                                "content",
                                JSONArray().put(JSONObject().put("type", "text").put("text", item.text)),
                            ),
                    ),
                ),
        )
    }
    return JSONObject().put("type", "taskList").put("content", content).toString()
}
