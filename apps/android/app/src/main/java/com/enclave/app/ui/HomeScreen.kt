package com.enclave.app.ui

import android.content.Intent
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
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Star
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
    var documents by remember { mutableStateOf<List<Document>>(emptyList()) }
    var folders by remember { mutableStateOf<List<Folder>>(emptyList()) }
    var query by remember { mutableStateOf("") }
    var loading by remember { mutableStateOf(true) }
    var layout by remember { mutableStateOf(NotesLayout.CARDS) }
    var sort by remember { mutableStateOf(NotesSort.UPDATED) }
    var folderFilter by remember { mutableStateOf<String?>(null) }
    var createOpen by remember { mutableStateOf(false) }
    var sheet by remember { mutableStateOf<HomeSheet?>(null) }
    var folderDialog by remember { mutableStateOf<FolderDialog?>(null) }
    var folderName by remember { mutableStateOf("") }
    var folderMenu by remember { mutableStateOf<Folder?>(null) }
    var folderToDelete by remember { mutableStateOf<Folder?>(null) }
    var noteMenu by remember { mutableStateOf<Document?>(null) }
    var templates by remember { mutableStateOf<List<UserTemplate>>(emptyList()) }
    var templateToDelete by remember { mutableStateOf<UserTemplate?>(null) }
    var noteMenuShared by remember { mutableStateOf(false) }
    val context = LocalContext.current
    val drawerState = rememberDrawerState(DrawerValue.Closed)
    // The drawer covers the screen, so Back closes it instead of leaving
    // the vault: the system default would finish the activity and drop the
    // user on the launcher with the drawer restored open next launch.
    BackHandler(enabled = drawerState.isOpen) {
        scope.launch { drawerState.close() }
    }
    BackHandler(enabled = createOpen) { createOpen = false }

    suspend fun reload() {
        loading = true
        documents = withContext(Dispatchers.IO) { Core.ffi.listDocuments() }
        folders = withContext(Dispatchers.IO) { Core.ffi.listFolders() }
        layout = withContext(Dispatchers.IO) { NotesPrefs.loadLayout() }
        sort = withContext(Dispatchers.IO) { NotesPrefs.loadSort() }
        templates = TemplateStore.load()
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
            documents = withContext(Dispatchers.IO) { Core.ffi.listDocuments() }
            WidgetCacheBridge.requestRefresh()
        }
    }

    fun openNoteMenu(doc: Document) {
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
            val updated = withContext(Dispatchers.IO) { Core.ffi.toggleFavorite(doc.id) }
            documents = documents.map { if (it.id == updated.id) updated else it }
            WidgetCacheBridge.requestRefresh()
        }
    }

    fun shareDocument(doc: Document) {
        scope.launch {
            val body = withContext(Dispatchers.IO) {
                val text = runCatching {
                    Core.ffi.getBlocks(doc.id)
                        .firstOrNull { it.blockType == "doc" }
                        ?.let { EditorDocument.parse(it.contentJson).plainText() }
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
            val copy = withContext(Dispatchers.IO) { Core.ffi.duplicateDocument(doc.id) }
            documents = listOf(copy) + documents
            WidgetCacheBridge.requestRefresh()
        }
    }

    fun deleteNote(doc: Document) {
        scope.launch {
            withContext(Dispatchers.IO) { Core.ffi.deleteDocument(doc.id) }
            documents = documents.filterNot { it.id == doc.id }
            WidgetCacheBridge.requestRefresh()
        }
    }

    fun create(target: CreateTarget) {
        scope.launch {
            createOpen = false
            if (target is CreateTarget.Builtin && target.kind == CreateKind.FOLDER) {
                openNewFolder()
                return@launch
            }

            val document = when (target) {
                is CreateTarget.Builtin -> NoteFactory.create(target.kind, folderId = folderFilter)
                is CreateTarget.Custom -> NoteFactory.createFromTemplate(target.template, folderId = folderFilter)
            }
            WidgetCacheBridge.requestRefresh()
            documents = listOf(document) + documents.filterNot { it.id == document.id }
            onOpenNote(document.id)
        }
    }

    val visible = remember(documents, query, folderFilter, sort) {
        documents
            .filter { folderFilter == null || it.folderId == folderFilter }
            .filter { query.isBlank() || it.title.contains(query, ignoreCase = true) }
            .sortedWith(sort.comparator())
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
                        onOpenSettings()
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
                Column(
                Modifier
                    .fillMaxSize()
                    .blur(if (createOpen) 8.dp else 0.dp),
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(start = 6.dp, end = 8.dp, top = 8.dp, bottom = 2.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    IconButton(onClick = { scope.launch { drawerState.open() } }) {
                        Icon(Icons.Filled.Menu, contentDescription = "Menu")
                    }
                    Text(
                        greeting(),
                        style = MaterialTheme.typography.headlineMedium,
                        fontWeight = FontWeight.SemiBold,
                        modifier = Modifier.weight(1f),
                    )
                    IconButton(onClick = onLock) {
                        Icon(Icons.Filled.Lock, contentDescription = "Lock vault")
                    }
                }

                SearchPill(
                    query = query,
                    onChange = { query = it },
                    onLayout = { sheet = HomeSheet.LAYOUT },
                    onSort = { sheet = HomeSheet.SORT },
                    modifier = Modifier.padding(horizontal = 16.dp, vertical = 10.dp),
                )

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
                        Text(
                            if (query.isBlank()) "No notes yet — tap + to start." else "No notes match “$query”.",
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            style = MaterialTheme.typography.bodyMedium,
                        )
                    }
                } else {
                    NotesContent(layout, visible, onOpenNote, onLongPressNote = ::openNoteMenu)
                }
            }

            if (createOpen) {
                Box(
                    Modifier
                        .fillMaxSize()
                        .background(MaterialTheme.colorScheme.background.copy(alpha = 0.55f))
                        .clickable(
                            interactionSource = remember { MutableInteractionSource() },
                            indication = null,
                        ) { createOpen = false },
                )
            }

            CreateStack(
                open = createOpen,
                onToggle = { createOpen = !createOpen },
                templates = templates,
                onCreate = ::create,
                onNewTemplate = {
                    createOpen = false
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
                ActionItem(Icons.Filled.Create, "Make a copy") { duplicate(doc) },
                ActionItem(Icons.Filled.Delete, "Delete", destructive = true) { deleteNote(doc) },
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
            options = NotesLayout.entries.map { it to "${it.emoji}  ${it.label}" },
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
    val container = if (selected) MaterialTheme.colorScheme.onSurface else Color.Transparent
    val content = if (selected) MaterialTheme.colorScheme.surface else MaterialTheme.colorScheme.onSurface
    val iconTint = if (selected) MaterialTheme.colorScheme.surface else MaterialTheme.colorScheme.onSurfaceVariant
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
        shape = RoundedCornerShape(28.dp),
        modifier = modifier.fillMaxWidth().height(52.dp),
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
                                "Search Enclave",
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
    documents: List<Document>,
    onOpenNote: (String) -> Unit,
    onLongPressNote: (Document) -> Unit,
) {
    when (layout) {
        NotesLayout.LIST -> LazyColumn(
            contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            items(documents, key = { it.id }) { doc ->
                NoteRow(doc, onClick = { onOpenNote(doc.id) }, onLongClick = { onLongPressNote(doc) })
            }
        }

        NotesLayout.CARDS -> LazyVerticalGrid(
            columns = GridCells.Fixed(2),
            contentPadding = PaddingValues(16.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            items(documents, key = { it.id }) { doc ->
                NoteCard(doc, onClick = { onOpenNote(doc.id) }, onLongClick = { onLongPressNote(doc) })
            }
        }

        NotesLayout.GALLERY -> LazyVerticalGrid(
            columns = GridCells.Fixed(2),
            contentPadding = PaddingValues(16.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            items(documents, key = { it.id }) { doc ->
                GalleryTile(doc, onClick = { onOpenNote(doc.id) }, onLongClick = { onLongPressNote(doc) })
            }
        }

        NotesLayout.ICONS -> LazyVerticalGrid(
            columns = GridCells.Fixed(3),
            contentPadding = PaddingValues(16.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            items(documents, key = { it.id }) { doc ->
                IconTile(doc, onClick = { onOpenNote(doc.id) }, onLongClick = { onLongPressNote(doc) })
            }
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun NoteRow(doc: Document, onClick: () -> Unit, onLongClick: () -> Unit) {
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
private fun NoteCard(doc: Document, onClick: () -> Unit, onLongClick: () -> Unit) {
    var preview by remember(doc.id) { mutableStateOf("") }
    LaunchedEffect(doc.id, doc.updatedAt) {
        preview = notePreview(doc.id)
    }
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(16.dp),
        modifier = Modifier
            .fillMaxWidth()
            .heightIn(min = 112.dp)
            .border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(16.dp))
            .combinedClickable(onClick = onClick, onLongClick = onLongClick),
    ) {
        Column(Modifier.fillMaxWidth().padding(16.dp)) {
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
                    maxLines = 5,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Spacer(Modifier.height(14.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    timeAgo(doc.updatedAt),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.weight(1f),
                )
                if (doc.isFavorite) {
                    Icon(
                        Icons.Filled.Star,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(14.dp),
                    )
                }
            }
        }
    }
}

/** Plain-text preview of a note, loaded lazily as cards become visible. */
private suspend fun notePreview(id: String): String = withContext(Dispatchers.IO) {
    val blocks = runCatching { Core.ffi.getBlocks(id) }.getOrDefault(emptyList())
    val doc = blocks.firstOrNull { it.blockType == "doc" } ?: return@withContext ""
    EditorDocument.parse(doc.contentJson).plainText()
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun GalleryTile(doc: Document, onClick: () -> Unit, onLongClick: () -> Unit) {
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant,
        shape = RoundedCornerShape(18.dp),
        modifier = Modifier.fillMaxWidth().height(164.dp)
            .combinedClickable(onClick = onClick, onLongClick = onLongClick),
    ) {
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
            )
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    timeAgo(doc.updatedAt),
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.weight(1f),
                )
                if (doc.isFavorite) {
                    Icon(
                        Icons.Filled.Star,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(14.dp),
                    )
                }
            }
        }
    }
}

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun IconTile(doc: Document, onClick: () -> Unit, onLongClick: () -> Unit) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(16.dp),
        modifier = Modifier.fillMaxWidth().height(104.dp)
            .combinedClickable(onClick = onClick, onLongClick = onLongClick),
    ) {
        Column(
            modifier = Modifier.fillMaxSize().padding(10.dp),
            verticalArrangement = Arrangement.SpaceBetween,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Icon(
                Icons.Filled.Create,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(18.dp),
            )
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
    open: Boolean,
    onToggle: () -> Unit,
    templates: List<com.enclave.app.editor.UserTemplate>,
    onCreate: (CreateTarget) -> Unit,
    onNewTemplate: () -> Unit,
    onDeleteTemplate: (com.enclave.app.editor.UserTemplate) -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(
        modifier = modifier,
        horizontalAlignment = Alignment.End,
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        if (open) {
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

        FloatingActionButton(
            onClick = onToggle,
            containerColor = MaterialTheme.colorScheme.primary,
            contentColor = MaterialTheme.colorScheme.onPrimary,
        ) {
            Icon(
                if (open) Icons.Filled.Close else Icons.Filled.Add,
                contentDescription = if (open) "Close create menu" else "New",
            )
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
    Surface(
        color = MaterialTheme.colorScheme.onSurface,
        contentColor = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(16.dp),
        shadowElevation = 2.dp,
        modifier = Modifier.combinedClickable(onClick = onClick, onLongClick = onLongClick ?: {}),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 18.dp, vertical = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(emoji, style = MaterialTheme.typography.titleMedium)
            Spacer(Modifier.width(10.dp))
            Text(label, style = MaterialTheme.typography.labelLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}

/** The old app's greeting, so the home reads the same way. */
private fun greeting(): String {
    val hour = Calendar.getInstance().get(Calendar.HOUR_OF_DAY)
    return when {
        hour < 12 -> "Good morning"
        hour < 18 -> "Good afternoon"
        else -> "Good evening"
    }
}

/** "Recent pages" at the root, the folder name when one is selected. */
private fun sectionTitle(folders: List<Folder>, folderFilter: String?): String =
    folders.firstOrNull { it.id == folderFilter }?.name ?: "Recent pages"

/** `2026-10-17T09:00:00+00:00` -> `just now` / `12m ago` / `3h ago` / `2d ago` / `2026-10-17`. */
internal fun timeAgo(iso: String): String {
    val then = parseIsoUtc(iso) ?: return iso.take(10)
    val seconds = ((System.currentTimeMillis() - then) / 1000).coerceAtLeast(0)
    return when {
        seconds < 60 -> "just now"
        seconds < 3600 -> "${seconds / 60}m ago"
        seconds < 86_400 -> "${seconds / 3600}h ago"
        seconds < 7 * 86_400 -> "${seconds / 86_400}d ago"
        else -> iso.take(10)
    }
}

/** Minimal RFC 3339 parser (second precision, UTC) — `java.time` needs API 26. */
private fun parseIsoUtc(iso: String): Long? = runCatching {
    val cal = Calendar.getInstance(TimeZone.getTimeZone("UTC"))
    cal.clear()
    cal.set(
        iso.substring(0, 4).toInt(),
        iso.substring(5, 7).toInt() - 1,
        iso.substring(8, 10).toInt(),
        iso.substring(11, 13).toInt(),
        iso.substring(14, 16).toInt(),
        iso.substring(17, 19).toInt(),
    )
    cal.timeInMillis
}.getOrNull()
