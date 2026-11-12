package com.enclave.app.ui

import android.content.Intent
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.togetherWith
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.FastOutLinearInEasing
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.foundation.Image
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Check
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Create
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Face
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SmallFloatingActionButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberDrawerState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.BlurEffect
import androidx.compose.ui.graphics.TileMode
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.enclave.app.R
import com.enclave.app.WidgetCacheBridge
import com.enclave.app.core.Core
import com.enclave.app.editor.CreateKind
import com.enclave.app.editor.CreateTarget
import com.enclave.app.editor.TemplateStore
import com.enclave.app.editor.UserTemplate
import com.enclave.app.editor.EditorDocument
import com.enclave.app.editor.NoteFactory
import com.enclave.app.sync.SyncController
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import uniffi.core_api.Document
import uniffi.core_api.HomeNote
import uniffi.core_api.Folder
import java.util.Calendar
import java.util.UUID
import java.util.TimeZone

private enum class HomeSheet { LAYOUT, SORT }

/** What the folder name dialog is currently editing. */
private sealed interface FolderDialog {
    data object New : FolderDialog
    data class Rename(val folder: Folder) : FolderDialog
}

/**
 * Notes home: search pill with the layout and sort controls, folder chips, the
 * chosen notes layout and the create stack (Text / Checklist / Journal /
 * Meeting notes / Folder). Preferences live in the vault.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(
    onOpenNote: (String) -> Unit,
    onOpenSync: () -> Unit,
    onOpenSettings: () -> Unit,
    onNewTemplate: () -> Unit,
    onLock: () -> Unit,
) {
    val scope = rememberCoroutineScope()
    // Seeded from the session cache: the content was what still blinked after
    // the layout was cached, because the list arrived a frame later.
    var documents by remember { mutableStateOf(NotesCache.documents ?: emptyList<HomeNote>()) }
    var folders by remember { mutableStateOf(NotesCache.folders ?: emptyList()) }
    var query by remember { mutableStateOf("") }
    var loading by remember { mutableStateOf(NotesCache.documents == null) }
    // Start from the last known choice so returning from a page does not paint
    // the default layout first and then switch to the chosen one.
    var layout by remember { mutableStateOf(NotesPrefs.layoutNow) }
    var sort by remember { mutableStateOf(NotesPrefs.sortNow) }
    var folderFilter by remember { mutableStateOf<String?>(null) }
    // A State object rather than a value: only the button, the scrim and the
    // wrapper read it, so toggling it cannot recompose the note list.
    val createOpen = remember { mutableStateOf(false) }
    var sheet by remember { mutableStateOf<HomeSheet?>(null) }
    var folderDialog by remember { mutableStateOf<FolderDialog?>(null) }
    var folderName by remember { mutableStateOf("") }
    var folderMenu by remember { mutableStateOf<Folder?>(null) }
    var folderToDelete by remember { mutableStateOf<Folder?>(null) }
    var noteMenu by remember { mutableStateOf<Document?>(null) }
    var templates by remember { mutableStateOf<List<UserTemplate>>(emptyList()) }
    var templateToDelete by remember { mutableStateOf<UserTemplate?>(null) }
    var noteMenuShared by remember { mutableStateOf(false) }
    var iconTarget by remember { mutableStateOf<Document?>(null) }
    var noteToDelete by remember { mutableStateOf<Document?>(null) }
    // Tiles key their lazy icon load on this, so setting an icon from this
    // screen refreshes the grid without reloading the whole list.
    var helpOpen by remember { mutableStateOf(false) }
    val context = LocalContext.current
    val drawerState = rememberDrawerState(DrawerValue.Closed)
    // The drawer covers the screen, so Back closes it instead of leaving
    // the vault: the system default would finish the activity and drop the
    // user on the launcher with the drawer restored open next launch.
    BackHandler(enabled = drawerState.isOpen) {
        scope.launch { drawerState.close() }
    }

    suspend fun reload() {
        // Refreshing under a list that is already on screen must not blank it:
        // only a cold start (nothing cached) shows the progress bar.
        loading = NotesCache.documents == null
        // One call for the whole list: titles, timestamps, emoji and previews.
        documents = withContext(Dispatchers.IO) { Core.ffi.homeNotes() }
        folders = withContext(Dispatchers.IO) { Core.ffi.listFolders() }
        layout = withContext(Dispatchers.IO) { NotesPrefs.loadLayout() }
        sort = withContext(Dispatchers.IO) { NotesPrefs.loadSort() }
        templates = TemplateStore.load()
        NotesCache.documents = documents
        NotesCache.folders = folders
        loading = false
    }

    LaunchedEffect(Unit) { reload() }

    fun openNewFolder() {
        folderName = ""
        folderDialog = FolderDialog.New
    }

    fun saveFolder() {
        val name = folderName.trim()
        val target = folderDialog ?: return
        if (name.isEmpty()) return
        folderDialog = null
        scope.launch {
            withContext(Dispatchers.IO) {
                when (target) {
                    FolderDialog.New -> Core.ffi.createFolder(name)
                    is FolderDialog.Rename -> Core.ffi.renameFolder(target.folder.id, name)
                }
            }
            // Re-read rather than patching the list: a rename also re-sorts the
            // drawer, and the core is the only source of truth for the name.
            folders = withContext(Dispatchers.IO) { Core.ffi.listFolders() }
        }
    }

    fun deleteFolder(folder: Folder) {
        scope.launch {
            withContext(Dispatchers.IO) { Core.ffi.deleteFolder(folder.id) }
            // The core moves the folder's notes to the root (see
            // core_db::delete_folder), so nothing is lost; reload both lists.
            if (folderFilter == folder.id) folderFilter = null
            folders = withContext(Dispatchers.IO) { Core.ffi.listFolders() }
            documents = withContext(Dispatchers.IO) { Core.ffi.homeNotes() }
            WidgetCacheBridge.requestRefresh()
        }
    }

    fun openNoteMenu(note: HomeNote) {
        val doc = note.document
        noteMenu = doc
        noteMenuShared = false
        scope.launch {
            noteMenuShared = withContext(Dispatchers.IO) {
                runCatching {
                    Core.ffi.getBlocks(doc.id)
                        .firstOrNull { it.blockType == "widget" }
                        ?.let { JSONObject(it.contentJson).optBoolean("enabled", false) }
                        ?: false
                }.getOrDefault(false)
            }
        }
    }

    fun togglePin(doc: Document) {
        scope.launch {
            withContext(Dispatchers.IO) { Core.ffi.toggleFavorite(doc.id) }
            documents = withContext(Dispatchers.IO) { Core.ffi.homeNotes() }
            WidgetCacheBridge.requestRefresh()
        }
    }

    fun shareDocument(doc: Document) {
        scope.launch {
            val body = withContext(Dispatchers.IO) {
                val text = runCatching {
                    Core.ffi.getBlocks(doc.id)
                        .firstOrNull { it.blockType == "doc" }
                        ?.let { Core.ffi.previewText(it.contentJson) }
                        .orEmpty()
                }.getOrDefault("")
                listOf(doc.title, text).filter { it.isNotBlank() }.joinToString("\n\n")
            }
            if (body.isBlank()) return@launch
            val send = Intent(Intent.ACTION_SEND).apply {
                type = "text/plain"
                putExtra(Intent.EXTRA_SUBJECT, doc.title)
                putExtra(Intent.EXTRA_TEXT, body)
            }
            context.startActivity(Intent.createChooser(send, "Send note"))
        }
    }

    fun setWidgetSharing(doc: Document, enabled: Boolean) {
        noteMenuShared = enabled
        scope.launch {
            withContext(Dispatchers.IO) {
                // Same block NoteScreen writes, so the toggle agrees with the
                // note's own sheet: the core projects shared notes to widgets.
                val existing = Core.ffi.getBlocks(doc.id).firstOrNull { it.blockType == "widget" }
                val id = existing?.id ?: UUID.randomUUID().toString()
                val content = JSONObject()
                    .put("enabled", enabled)
                    .put("editable", enabled)
                    .toString()
                Core.ffi.upsertBlock(id, doc.id, "widget", content, existing?.sortOrder ?: 1.0)
            }
            WidgetCacheBridge.requestRefresh()
        }
    }

    fun duplicate(doc: Document) {
        scope.launch {
            withContext(Dispatchers.IO) { Core.ffi.duplicateDocument(doc.id) }
            documents = withContext(Dispatchers.IO) { Core.ffi.homeNotes() }
            WidgetCacheBridge.requestRefresh()
        }
    }

    fun deleteNote(doc: Document) {
        scope.launch {
            withContext(Dispatchers.IO) { Core.ffi.deleteDocument(doc.id) }
            documents = withContext(Dispatchers.IO) { Core.ffi.homeNotes() }
            WidgetCacheBridge.requestRefresh()
        }
    }

    fun create(target: CreateTarget) {
        scope.launch {
            createOpen.value = false
            if (target is CreateTarget.Builtin && target.kind == CreateKind.FOLDER) {
                openNewFolder()
                return@launch
            }

            val document = when (target) {
                is CreateTarget.Builtin -> NoteFactory.create(target.kind, folderId = folderFilter)
                is CreateTarget.Custom -> NoteFactory.createFromTemplate(target.template, folderId = folderFilter)
            }
            WidgetCacheBridge.requestRefresh()
            documents = withContext(Dispatchers.IO) { Core.ffi.homeNotes() }
            onOpenNote(document.id)
        }
    }

    val visible = remember(documents, query, folderFilter, sort) {
        val byDocument = sort.comparator()
        documents
            .filter { folderFilter == null || it.document.folderId == folderFilter }
            .filter { query.isBlank() || it.document.title.contains(query, ignoreCase = true) }
            .sortedWith { a, b ->
                // Pinned notes lead in every layout; the chosen sort decides
                // the rest.
                val pinned = b.document.isFavorite.compareTo(a.document.isFavorite)
                if (pinned != 0) pinned else byDocument.compare(a.document, b.document)
            }
    }

    val drawerDark = MaterialTheme.colorScheme.background.luminance() < 0.5f

    ModalNavigationDrawer(
        drawerState = drawerState,
        drawerContent = {
            ModalDrawerSheet(drawerContainerColor = MaterialTheme.colorScheme.surface) {
                Image(
                    painter = painterResource(
                        if (drawerDark) R.drawable.ic_enclave_lockup_paper else R.drawable.ic_enclave_lockup_ink,
                    ),
                    contentDescription = "Enclave",
                    modifier = Modifier
                        .padding(horizontal = 28.dp, vertical = 24.dp)
                        .width(152.dp),
                )
                DrawerRow(
                    label = "Notes",
                    icon = Icons.Filled.Create,
                    selected = folderFilter == null,
                    onClick = {
                        folderFilter = null
                        scope.launch { drawerState.close() }
                    },
                )
                DrawerRow(
                    label = "New folder",
                    icon = Icons.Filled.Add,
                    selected = false,
                    onClick = {
                        scope.launch { drawerState.close() }
                        openNewFolder()
                    },
                )
                if (folders.isNotEmpty()) {
                    Text(
                        "FOLDERS",
                        style = MaterialTheme.typography.labelMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(start = 28.dp, top = 18.dp, bottom = 4.dp),
                    )
                    folders.forEach { folder ->
                        DrawerRow(
                            label = folder.name,
                            icon = Icons.AutoMirrored.Filled.List,
                            selected = folderFilter == folder.id,
                            onClick = {
                                folderFilter = folder.id
                                scope.launch { drawerState.close() }
                            },
                            onLongClick = { folderMenu = folder },
                        )
                    }
                }
                Spacer(Modifier.height(12.dp))
                DrawerRow(
                    label = "Sync",
                    icon = Icons.Filled.Refresh,
                    selected = false,
                    onClick = {
                        scope.launch { drawerState.close() }
                        onOpenSync()
                    },
                )
                DrawerRow(
                    label = "Settings",
                    icon = Icons.Filled.Settings,
                    selected = false,
                    onClick = {
                        scope.launch { drawerState.close() }
                        onOpenSettings()
                    },
                )
                DrawerRow(
                    label = "Lock now",
                    icon = Icons.Filled.Lock,
                    selected = false,
                    onClick = {
                        scope.launch { drawerState.close() }
                        onLock()
                    },
                )
                DrawerRow(
                    label = "Help & feedback",
                    icon = Icons.Filled.Info,
                    selected = false,
                    onClick = {
                        scope.launch { drawerState.close() }
                        helpOpen = true
                    },
                )
            }
        },
    ) {
        Scaffold(containerColor = MaterialTheme.colorScheme.background) { padding ->
            Box(Modifier.fillMaxSize().padding(padding)) {
                // The stack is an overlay rather than a FAB slot: while it is
                // open the home blurs behind a scrim that swallows taps, so a
                // stray tap cannot open a note hidden under the pills.
                HomeSurface(pressed = createOpen) {
                Column(Modifier.fillMaxSize()) {
                // Menu, search and lock share one row: the greeting cost a
                // whole line of vertical space and said nothing the notes
                // below did not already say.
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(start = 4.dp, end = 4.dp, top = 6.dp, bottom = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    IconButton(onClick = { scope.launch { drawerState.open() } }) {
                        Icon(Icons.Filled.Menu, contentDescription = "Menu")
                    }
                    SearchPill(
                        query = query,
                        onChange = { query = it },
                        onLayout = { sheet = HomeSheet.LAYOUT },
                        onSort = { sheet = HomeSheet.SORT },
                        modifier = Modifier.weight(1f).padding(horizontal = 4.dp),
                    )
                    IconButton(onClick = onLock) {
                        Icon(Icons.Filled.Lock, contentDescription = "Lock vault")
                    }
                }

                if (folders.isNotEmpty()) {
                    LazyRow(
                        contentPadding = PaddingValues(horizontal = 16.dp),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        modifier = Modifier.fillMaxWidth().padding(bottom = 6.dp),
                    ) {
                        item { FolderChip("All", folderFilter == null) { folderFilter = null } }
                        items(folders, key = { it.id }) { folder ->
                            FolderChip(folder.name, folderFilter == folder.id) { folderFilter = folder.id }
                        }
                    }
                }

                if (loading) {
                    LinearProgressIndicator(Modifier.fillMaxWidth())
                }

                if (visible.isNotEmpty() && !loading) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(start = 20.dp, end = 20.dp, top = 12.dp, bottom = 4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            sectionTitle(folders, folderFilter).uppercase(),
                            style = MaterialTheme.typography.labelMedium,
                            fontWeight = FontWeight.SemiBold,
                            letterSpacing = 0.6.sp,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Spacer(Modifier.width(8.dp))
                        Surface(
                            color = MaterialTheme.colorScheme.surfaceVariant,
                            shape = RoundedCornerShape(999.dp),
                        ) {
                            Text(
                                "${visible.size}",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 1.dp),
                            )
                        }
                    }
                }

                if (visible.isEmpty() && !loading) {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        // Distinguish the three reasons the list can be empty:
                        // a search with no hits, an empty folder, and a vault
                        // with no notes at all. "No notes yet" for a filtered
                        // folder reads as if the notes had disappeared.
                        val folderName = folders.firstOrNull { it.id == folderFilter }?.name
                        Text(
                            when {
                                query.isNotBlank() -> "No notes match “$query”."
                                folderName != null -> "No notes in “$folderName” yet — tap + to start."
                                else -> "No notes yet — tap + to start."
                            },
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            style = MaterialTheme.typography.bodyMedium,
                        )
                    }
                } else {
                    NotesContent(layout, visible, onOpenNote, ::openNoteMenu)
                }
            }

            Scrim(visible = createOpen, onDismiss = { createOpen.value = false })

            }
            CreateStack(
                open = createOpen,
                onToggle = { createOpen.value = !createOpen.value },
                templates = templates,
                onCreate = ::create,
                onNewTemplate = {
                    createOpen.value = false
                    onNewTemplate()
                },
                onDeleteTemplate = { templateToDelete = it },
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .padding(end = 16.dp, bottom = 16.dp),
            )
            }
        }
    }

    if (helpOpen) {
        ActionSheet(
            title = "Help & feedback",
            subtitle = "Opens in the GitHub app or your browser",
            actions = listOf(
                ActionItem(Icons.Filled.Warning, "Report a problem") {
                    openUrl(context, "$PROJECT_URL/issues/new/choose")
                },
                ActionItem(Icons.AutoMirrored.Filled.List, "Browse open issues") {
                    openUrl(context, "$PROJECT_URL/issues")
                },
                ActionItem(Icons.Filled.Lock, "Security policy") {
                    openUrl(context, "$PROJECT_URL/security/policy")
                },
            ),
            onDismiss = { helpOpen = false },
        )
    }

    noteToDelete?.let { doc ->
        AlertDialog(
            onDismissRequest = { noteToDelete = null },
            title = { Text("Delete “${doc.title.ifBlank { "Untitled note" }}”?") },
            text = {
                Text(
                    "Its content is removed from this vault. This cannot be undone.",
                    style = MaterialTheme.typography.bodyMedium,
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        noteToDelete = null
                        deleteNote(doc)
                    },
                ) {
                    Text("Delete", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { noteToDelete = null }) { Text("Cancel") }
            },
        )
    }

    iconTarget?.let { doc ->
        EmojiPickerDialog(
            current = null,
            onPick = { picked ->
                iconTarget = null
                scope.launch {
                    NoteIcon.save(doc.id, picked)
                    documents = withContext(Dispatchers.IO) { Core.ffi.homeNotes() }
                }
            },
            onDismiss = { iconTarget = null },
        )
    }

    noteMenu?.let { doc ->
        ActionSheet(
            title = doc.title.ifBlank { "Untitled note" },
            subtitle = timeAgo(doc.updatedAt),
            actions = listOf(
                ActionItem(Icons.AutoMirrored.Filled.List, "Open") { onOpenNote(doc.id) },
                ActionItem(
                    Icons.Filled.Star,
                    if (doc.isFavorite) "Unpin" else "Pin to favorites",
                ) { togglePin(doc) },
                ActionItem(Icons.AutoMirrored.Filled.Send, "Send") { shareDocument(doc) },
                ActionItem(
                    Icons.AutoMirrored.Filled.List,
                    if (noteMenuShared) "Hide from widgets" else "Show in widgets",
                ) { setWidgetSharing(doc, !noteMenuShared) },
                ActionItem(Icons.Filled.Face, "Set icon") { iconTarget = doc },
                ActionItem(Icons.Filled.Create, "Make a copy") { duplicate(doc) },
                ActionItem(Icons.Filled.Delete, "Delete", destructive = true) { noteToDelete = doc },
            ),
            onDismiss = { noteMenu = null },
        )
    }

    folderDialog?.let { dialog ->
        FolderNameDialog(
            title = when (dialog) {
                FolderDialog.New -> "New folder"
                is FolderDialog.Rename -> "Rename folder"
            },
            confirmLabel = when (dialog) {
                FolderDialog.New -> "Create"
                is FolderDialog.Rename -> "Save"
            },
            value = folderName,
            onChange = { folderName = it },
            onConfirm = ::saveFolder,
            onDismiss = { folderDialog = null },
        )
    }

    folderMenu?.let { folder ->
        ActionSheet(
            title = folder.name,
            subtitle = "Folder",
            actions = listOf(
                ActionItem(Icons.Filled.Create, "Rename") {
                    folderName = folder.name
                    folderDialog = FolderDialog.Rename(folder)
                },
                ActionItem(Icons.Filled.Delete, "Delete folder", destructive = true) {
                    folderToDelete = folder
                },
            ),
            onDismiss = { folderMenu = null },
        )
    }

    templateToDelete?.let { template ->
        AlertDialog(
            onDismissRequest = { templateToDelete = null },
            title = { Text("Delete “${template.name}”?") },
            text = {
                Text(
                    "Templates are not notes: deleting one does not touch any page made from it.",
                    style = MaterialTheme.typography.bodyMedium,
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        templateToDelete = null
                        scope.launch { templates = TemplateStore.remove(template.id) }
                    },
                ) {
                    Text("Delete", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { templateToDelete = null }) { Text("Cancel") }
            },
        )
    }

    folderToDelete?.let { folder ->
        AlertDialog(
            onDismissRequest = { folderToDelete = null },
            title = { Text("Delete “${folder.name}”?") },
            text = {
                Text(
                    "The notes inside move back to the root list. Nothing is deleted.",
                    style = MaterialTheme.typography.bodyMedium,
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        folderToDelete = null
                        deleteFolder(folder)
                    },
                ) {
                    Text("Delete", color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { folderToDelete = null }) { Text("Cancel") }
            },
        )
    }

    when (sheet) {
        HomeSheet.LAYOUT -> OptionSheet(
            title = "Layout",
            options = NotesLayout.entries.map {
                it to if (LocalMonochrome.current) it.label else "${it.emoji}  ${it.label}"
            },
            selected = layout,
            onSelect = {
                layout = it
                scope.launch { withContext(Dispatchers.IO) { NotesPrefs.saveLayout(it) } }
                sheet = null
            },
            onDismiss = { sheet = null },
        )

        HomeSheet.SORT -> OptionSheet(
            title = "Sort by",
            options = NotesSort.entries.map { it to it.label },
            selected = sort,
            onSelect = {
                sort = it
                scope.launch { withContext(Dispatchers.IO) { NotesPrefs.saveSort(it) } }
                sheet = null
            },
            onDismiss = { sheet = null },
        )

        null -> Unit
    }
}

/**
 * One drawer row. It follows the drawer's existing language (a filled pill for
 * the selected destination) and adds a long press, which [NavigationDrawerItem]
 * cannot take: folders use it for rename and delete.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun DrawerRow(
    label: String,
    icon: ImageVector,
    selected: Boolean,
    onClick: () -> Unit,
    onLongClick: (() -> Unit)? = null,
) {
    // Selected is an accent wash, not an inverted pill: inverting made a light
    // bar inside a dark theme (and read as a different app in Monochrome, where
    // the accent is already neutral).
    val container = if (selected) MaterialTheme.colorScheme.primary.copy(alpha = 0.16f) else Color.Transparent
    val content = MaterialTheme.colorScheme.onSurface
    val iconTint = if (selected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant
    Surface(
        color = container,
        shape = CircleShape,
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 12.dp, vertical = 2.dp)
            .combinedClickable(onClick = onClick, onLongClick = onLongClick ?: {}),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 16.dp, vertical = 13.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(icon, contentDescription = null, tint = iconTint)
            Spacer(Modifier.width(16.dp))
            Text(
                label,
                style = MaterialTheme.typography.labelLarge,
                color = content,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f),
            )
        }
    }
}

/** Name prompt shared by "New folder" and "Rename folder". */
@Composable
private fun FolderNameDialog(
    title: String,
    confirmLabel: String,
    value: String,
    onChange: (String) -> Unit,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { focus.requestFocus() }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            OutlinedTextField(
                value = value,
                onValueChange = onChange,
                singleLine = true,
                label = { Text("Folder name") },
                modifier = Modifier.fillMaxWidth().focusRequester(focus),
            )
        },
        confirmButton = {
            TextButton(
                enabled = value.isNotBlank(),
                onClick = onConfirm,
            ) { Text(confirmLabel) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel") }
        },
    )
}

@Composable
private fun SearchPill(
    query: String,
    onChange: (String) -> Unit,
    onLayout: () -> Unit,
    onSort: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant,
        shape = RoundedCornerShape(24.dp),
        modifier = modifier.fillMaxWidth().height(48.dp),
    ) {
        Row(
            modifier = Modifier.padding(start = 16.dp, end = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                Icons.Filled.Search,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.width(10.dp))
            BasicTextField(
                value = query,
                onValueChange = onChange,
                singleLine = true,
                textStyle = MaterialTheme.typography.bodyLarge.copy(
                    color = MaterialTheme.colorScheme.onSurface,
                ),
                cursorBrush = SolidColor(MaterialTheme.colorScheme.primary),
                decorationBox = { inner ->
                    Box(contentAlignment = Alignment.CenterStart) {
                        if (query.isEmpty()) {
                            Text(
                                "Search",
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                style = MaterialTheme.typography.bodyLarge,
                            )
                        }
                        inner()
                    }
                },
                modifier = Modifier.weight(1f),
            )
            IconButton(onClick = onLayout) {
                Icon(Icons.AutoMirrored.Filled.List, contentDescription = "Layout")
            }
            IconButton(onClick = onSort) {
                Icon(Icons.Filled.KeyboardArrowDown, contentDescription = "Sort")
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun FolderChip(label: String, selected: Boolean, onClick: () -> Unit) {
    FilterChip(
        selected = selected,
        onClick = onClick,
        label = { Text(label, maxLines = 1, overflow = TextOverflow.Ellipsis) },
    )
}

@Composable
private fun NotesContent(
    layout: NotesLayout,
    documents: List<HomeNote>,
    onOpenNote: (String) -> Unit,
    onLongPressNote: (HomeNote) -> Unit,
) {
    when (layout) {
        NotesLayout.LIST -> LazyColumn(
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 8.dp, bottom = FabClearance),
            verticalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            items(documents, key = { it.document.id }) { note ->
                NoteRow(note, onClick = { onOpenNote(note.document.id) }, onLongClick = { onLongPressNote(note) })
            }
        }

        NotesLayout.CARDS -> LazyVerticalStaggeredGrid(
            // Staggered, not a fixed grid: a card is as tall as its preview, so
            // a short note no longer leaves a hole beside a long one.
            columns = StaggeredGridCells.Fixed(2),
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 16.dp, bottom = FabClearance),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalItemSpacing = 12.dp,
            modifier = Modifier.fillMaxSize(),
        ) {
            items(documents, key = { it.document.id }) { note ->
                NoteCard(
                    note = note,
                    modifier = Modifier.animateItem(),
                    onClick = { onOpenNote(note.document.id) },
                    onLongClick = { onLongPressNote(note) },
                )
            }
        }

        NotesLayout.GALLERY -> LazyVerticalGrid(
            columns = GridCells.Fixed(2),
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 16.dp, bottom = FabClearance),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            items(documents, key = { it.document.id }) { note ->
                GalleryTile(
                    note = note,
                    modifier = Modifier.animateItem(),
                    onClick = { onOpenNote(note.document.id) },
                    onLongClick = { onLongPressNote(note) },
                )
            }
        }

        NotesLayout.ICONS -> LazyVerticalGrid(
            columns = GridCells.Fixed(3),
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, top = 16.dp, bottom = FabClearance),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            items(documents, key = { it.document.id }) { note ->
                IconTile(
                    note = note,
                    modifier = Modifier.animateItem(),
                    onClick = { onOpenNote(note.document.id) },
                    onLongClick = { onLongPressNote(note) },
                )
            }
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun NoteRow(note: HomeNote, onClick: () -> Unit, onLongClick: () -> Unit) {
    val doc = note.document
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(14.dp),
        modifier = Modifier.fillMaxWidth()
            .combinedClickable(onClick = onClick, onLongClick = onLongClick),
    ) {
        Row(Modifier.padding(14.dp), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text(
                    doc.title.ifBlank { "Untitled note" },
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(Modifier.height(4.dp))
                Text(
                    timeAgo(doc.updatedAt),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            if (doc.isFavorite) {
                Icon(
                    Icons.Filled.Star,
                    contentDescription = "Favorite",
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(18.dp),
                )
            }
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun NoteCard(
    note: HomeNote,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
    onLongClick: () -> Unit,
) {
    val doc = note.document
    val preview = note.preview
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(16.dp),
        modifier = modifier
            .fillMaxWidth()
            .heightIn(min = 96.dp)
            .border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(16.dp))
            .combinedClickable(onClick = onClick, onLongClick = onLongClick),
    ) {
        Box {
            // A pinned note leads the list and says so in the corner the eye
            // reaches first.
            if (doc.isFavorite) {
                Icon(
                    Icons.Filled.Star,
                    contentDescription = "Pinned",
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.align(Alignment.TopEnd).padding(10.dp).size(16.dp),
                )
            }
            Column(Modifier.fillMaxWidth().padding(16.dp).padding(end = if (doc.isFavorite) 12.dp else 0.dp)) {
                Text(
                    doc.title.ifBlank { "Untitled note" },
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 2,
                    overflow = TextOverflow.Ellipsis,
                )
                if (preview.isNotBlank()) {
                    Spacer(Modifier.height(8.dp))
                    Text(
                        preview,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 6,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                Spacer(Modifier.height(14.dp))
                Text(
                    timeAgo(doc.updatedAt),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

/**
 * The home content, blurred while the create stack is open.
 *
 * The blur radius is a **fixed value per state**, never animated: animating it
 * re-applies the render effect on every frame, which is what made the toggle
 * look like the background was straining to blur. Applying it here — outside the
 * content lambda — also keeps a toggle from re-running the whole home body.
 */
@Composable
private fun HomeSurface(pressed: androidx.compose.runtime.State<Boolean>, content: @Composable () -> Unit) {
    // A real-time blur of a full-screen Compose layer allocates an offscreen
    // layer on every toggle (measured: one ~100 ms frame on the test device),
    // and that hitch is what read as the background fighting the blur. The
    // depth cue is a cheap transform instead: the content settles back 2% under
    // the scrim, which is one persistent layer and pure GPU interpolation.
    //
    // Set this to true for the old blurred look; it is the one line to change.
    val useBlur = false
    val blurred = pressed.value
    val scale by animateFloatAsState(
        targetValue = if (blurred) 0.98f else 1f,
        animationSpec = tween(170, easing = FastOutSlowInEasing),
        label = "home-scale",
    )
    Box(
        Modifier
            .fillMaxSize()
            .graphicsLayer {
                scaleX = scale
                scaleY = scale
                if (useBlur) {
                    renderEffect = if (blurred) {
                        BlurEffect(40f, 40f, TileMode.Clamp)
                    } else {
                        null
                    }
                }
            },
    ) { content() }
}

/** The dim layer under the stack: only its alpha animates, and only it takes taps. */
@Composable
private fun Scrim(visible: androidx.compose.runtime.State<Boolean>, onDismiss: () -> Unit) {
    val open = visible.value
    BackHandler(enabled = open) { onDismiss() }
    val alpha by animateFloatAsState(
        targetValue = if (open) 0.6f else 0f,
        animationSpec = tween(110),
        label = "create-scrim",
    )
    // The pointer handler is attached only while the stack is open. A
    // full-screen scrim that keeps one while closed swallows every tap on the
    // list underneath — a disabled clickable still takes the event.
    val dismiss = Modifier.pointerInput(Unit) {
        detectTapGestures { onDismiss() }
    }
    if (open || alpha > 0.001f) {
        Box(
            Modifier
                .fillMaxSize()
                // Black, not the background colour: scrimming a dark
                // background with its own colour is invisible, so the stack had
                // no contrast against the list behind it.
                .background(MaterialTheme.colorScheme.scrim.copy(alpha = alpha))
                .then(if (open) dismiss else Modifier),
        )
    }
}

/** Where "Help & feedback" sends people: the repository, not a mailbox. */
private const val PROJECT_URL = "https://github.com/Pranesh-Selvaraj/Enclave"

/**
 * Open [url] in whatever handles links, and say so when nothing does: a phone
 * with no browser (or with the intent filtered out) should not look like the
 * button did nothing.
 */
private fun openUrl(context: android.content.Context, url: String) {
    val intent = Intent(Intent.ACTION_VIEW, android.net.Uri.parse(url))
    runCatching { context.startActivity(intent) }
        .onFailure {
            android.widget.Toast
                .makeText(context, "No app can open $url", android.widget.Toast.LENGTH_LONG)
                .show()
        }
}

/** Room under the last row so the floating create button never covers a note. */
private val FabClearance = 96.dp

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun GalleryTile(
    note: HomeNote,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
    onLongClick: () -> Unit,
) {
    val doc = note.document
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant,
        shape = RoundedCornerShape(18.dp),
        modifier = modifier.fillMaxWidth().height(164.dp)
            .combinedClickable(onClick = onClick, onLongClick = onLongClick),
    ) {
        Box(Modifier.fillMaxSize()) {
            if (doc.isFavorite) {
                Icon(
                    Icons.Filled.Star,
                    contentDescription = "Pinned",
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.align(Alignment.TopEnd).padding(12.dp).size(16.dp),
                )
            }
        Column(
            modifier = Modifier.fillMaxSize().padding(14.dp),
            verticalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(
                doc.title.ifBlank { "Untitled note" },
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.SemiBold,
                maxLines = 4,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.padding(end = if (doc.isFavorite) 14.dp else 0.dp),
            )
            Text(
                timeAgo(doc.updatedAt),
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun IconTile(
    note: HomeNote,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
    onLongClick: () -> Unit,
) {
    val doc = note.document
    val emoji = note.emoji

    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(16.dp),
        modifier = modifier.fillMaxWidth().height(104.dp)
            .combinedClickable(onClick = onClick, onLongClick = onLongClick),
    ) {
        Column(
            modifier = Modifier.fillMaxSize().padding(10.dp),
            verticalArrangement = Arrangement.SpaceBetween,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            val chosen = emoji
            if (chosen != null) {
                if (LocalMonochrome.current) {
                    // An emoji is a colour bitmap; in Monochrome the note shows
                    // its initial instead, in the theme's own foreground.
                    Text(
                        doc.title.trim().take(1).uppercase().ifEmpty { "•" },
                        style = MaterialTheme.typography.titleLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    Text(chosen, fontSize = 24.sp)
                }
            } else {
                Icon(
                    Icons.Filled.Create,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(18.dp),
                )
            }
            Text(
                doc.title.ifBlank { "Untitled" },
                style = MaterialTheme.typography.labelMedium,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                textAlign = TextAlign.Center,
            )
        }
    }
}

@Composable
private fun CreateStack(
    open: androidx.compose.runtime.State<Boolean>,
    onToggle: () -> Unit,
    templates: List<com.enclave.app.editor.UserTemplate>,
    onCreate: (CreateTarget) -> Unit,
    onNewTemplate: () -> Unit,
    onDeleteTemplate: (com.enclave.app.editor.UserTemplate) -> Unit,
    modifier: Modifier = Modifier,
) {
    val isOpen = open.value
    Column(
        modifier = modifier,
        horizontalAlignment = Alignment.End,
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        // The stack grows out of the button's own corner. A vertical slide was
        // wrong: for a tall stack it read as arriving from the bottom of the
        // screen instead of from the + that was just tapped.
        AnimatedVisibility(
            visible = isOpen,
            enter = scaleIn(
                initialScale = 0.45f,
                transformOrigin = TransformOrigin(1f, 1f),
                animationSpec = tween(120, easing = FastOutSlowInEasing),
            ) + fadeIn(tween(70)),
            exit = scaleOut(
                targetScale = 0.45f,
                transformOrigin = TransformOrigin(1f, 1f),
                animationSpec = tween(90, easing = FastOutLinearInEasing),
            ) + fadeOut(tween(70)),
        ) {
        Column(horizontalAlignment = Alignment.End, verticalArrangement = Arrangement.spacedBy(10.dp)) {
            CreateKind.entries.forEach { kind ->
                CreatePill(kind.emoji, kind.label, onClick = { onCreate(CreateTarget.Builtin(kind)) })
            }
            CreatePill("\uD83E\uDDE9", "New template", onClick = onNewTemplate)
            // The user's own templates sit after the built-ins, under one
            // heading, so a long list of them does not bury "Folder".
            if (templates.isNotEmpty()) {
                Text(
                    "YOUR TEMPLATES",
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(end = 4.dp),
                )
                templates.forEach { template ->
                    CreatePill(
                        emoji = "\uD83D\uDCC4",
                        label = template.name,
                        onClick = { onCreate(CreateTarget.Custom(template)) },
                        onLongClick = { onDeleteTemplate(template) },
                    )
                }
            }
        }
        }

        // The glyph pops: the plus shrinks away as the close springs in. No
        // spin — a rotating plus reads as a loading spinner, not a state.
        FloatingActionButton(
            onClick = onToggle,
            containerColor = MaterialTheme.colorScheme.primary,
            contentColor = MaterialTheme.colorScheme.onPrimary,
        ) {
            // The glyph only crossfades: the motion belongs to the stack
            // growing out of the button, not to the icon itself.
            AnimatedContent(
                targetState = isOpen,
                transitionSpec = {
                    fadeIn(tween(90)) togetherWith fadeOut(tween(70))
                },
                label = "create-glyph",
            ) { isOpen ->
                Icon(
                    if (isOpen) Icons.Filled.Close else Icons.Filled.Add,
                    contentDescription = if (isOpen) "Close create menu" else "New",
                )
            }
        }
    }
}

/** One pill of the create stack; the user's templates also take a long press. */
@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun CreatePill(
    emoji: String,
    label: String,
    onClick: () -> Unit,
    onLongClick: (() -> Unit)? = null,
) {
    // A theme surface, not an inverted one: a dark theme must stay dark. The
    // emoji is the only colour in a pill, and Monochrome has none to give — an
    // emoji cannot be tinted, so it is dropped rather than left as the one
    // coloured thing on screen.
    val monochrome = LocalMonochrome.current
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant,
        contentColor = MaterialTheme.colorScheme.onSurface,
        shape = RoundedCornerShape(16.dp),
        shadowElevation = 2.dp,
        modifier = Modifier.combinedClickable(onClick = onClick, onLongClick = onLongClick ?: {}),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = if (monochrome) 20.dp else 18.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            if (!monochrome) {
                Text(emoji, style = MaterialTheme.typography.titleMedium)
                Spacer(Modifier.width(10.dp))
            }
            Text(label, style = MaterialTheme.typography.labelLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}

/** "Recent pages" at the root, the folder name when one is selected. */
private fun sectionTitle(folders: List<Folder>, folderFilter: String?): String =
    folders.firstOrNull { it.id == folderFilter }?.name ?: "Recent pages"

/** `2026-10-17T09:00:00+00:00` -> `just now` / `12m ago` / `3h ago` / `2d ago` / `2026-10-17`. */
/**
 * "just now" / "5m ago" / "3h ago" / "2d ago", and a short date beyond a week.
 *
 * [now] is injectable so the cases are testable without sleeping. Anything
 * older than a week becomes "3 Nov 2026" — the previous fallback printed the
 * raw `2026-11-03T09:12:44+00:00` string, which is what every card showed for
 * notes older than seven days. A timestamp in the future (a device clock that
 * moved backwards) reads as "just now" rather than as a negative age.
 */
internal fun timeAgo(iso: String, now: Long = System.currentTimeMillis()): String {
    val then = parseIsoUtc(iso) ?: return iso.take(10)
    val seconds = (now - then) / 1000
    return when {
        seconds < 60 -> "just now"
        seconds < 3600 -> "${seconds / 60}m ago"
        seconds < 86_400 -> "${seconds / 3600}h ago"
        seconds < 7 * 86_400 -> "${seconds / 86_400}d ago"
        else -> shortDate(then)
    }
}

/**
 * Minimal RFC 3339 / ISO-8601 parser in UTC (`java.time` needs API 26).
 *
 * The core writes `chrono::Utc::now().to_rfc3339()`, which carries nanoseconds
 * and a `+00:00` offset, and older rows in a vault may carry a plain date. Both
 * are accepted here; anything else returns null and the caller falls back.
 */
private fun parseIsoUtc(iso: String): Long? = runCatching {
    require(iso.length >= 10)
    val date = iso.substring(0, 10)
    val time = if (iso.length >= 19 && (iso[10] == 'T' || iso[10] == ' ')) {
        iso.substring(11, 19)
    } else {
        "00:00:00"
    }
    val cal = Calendar.getInstance(TimeZone.getTimeZone("UTC"))
    cal.clear()
    cal.set(
        date.substring(0, 4).toInt(),
        date.substring(5, 7).toInt() - 1,
        date.substring(8, 10).toInt(),
        time.substring(0, 2).toInt(),
        time.substring(3, 5).toInt(),
        time.substring(6, 8).toInt(),
    )
    cal.timeInMillis
}.getOrNull()

/** "3 Nov 2026" — the label for anything older than a week. */
private fun shortDate(utcMillis: Long): String =
    java.text.SimpleDateFormat("d MMM yyyy", java.util.Locale.getDefault())
        .apply { timeZone = TimeZone.getTimeZone("UTC") }
        .format(java.util.Date(utcMillis))
