package com.enclave.app.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
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
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SmallFloatingActionButton
import androidx.compose.material3.Surface
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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.enclave.app.WidgetCacheBridge
import com.enclave.app.core.Core
import com.enclave.app.editor.CreateKind
import com.enclave.app.editor.NoteFactory
import com.enclave.app.sync.SyncController
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.core_api.Document
import uniffi.core_api.Folder

private enum class HomeSheet { LAYOUT, SORT }

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

    suspend fun reload() {
        loading = true
        documents = withContext(Dispatchers.IO) { Core.ffi.listDocuments() }
        folders = withContext(Dispatchers.IO) { Core.ffi.listFolders() }
        layout = withContext(Dispatchers.IO) { NotesPrefs.loadLayout() }
        sort = withContext(Dispatchers.IO) { NotesPrefs.loadSort() }
        loading = false
    }

    LaunchedEffect(Unit) { reload() }

    fun create(kind: CreateKind) {
        scope.launch {
            createOpen = false
            if (kind == CreateKind.FOLDER) {
                val folder = withContext(Dispatchers.IO) { Core.ffi.createFolder("New folder") }
                folders = folders + folder
                return@launch
            }

            val document = NoteFactory.create(kind, folderId = folderFilter)
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

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        floatingActionButton = {
            CreateStack(
                open = createOpen,
                onToggle = { createOpen = !createOpen },
                onCreate = ::create,
            )
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(start = 20.dp, end = 8.dp, top = 12.dp, bottom = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    "Enclave",
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.weight(1f),
                )
                IconButton(onClick = onOpenSync) {
                    Box(contentAlignment = Alignment.TopEnd) {
                        Icon(Icons.Filled.Refresh, contentDescription = "Sync")
                        if (SyncController.enabled) {
                            Box(
                                Modifier
                                    .padding(top = 3.dp, end = 3.dp)
                                    .size(8.dp)
                                    .background(Color(0xFF2E7D32), CircleShape),
                            )
                        }
                    }
                }
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

            if (visible.isEmpty() && !loading) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text(
                        if (query.isBlank()) "No notes yet — tap + to start." else "No notes match “$query”.",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        style = MaterialTheme.typography.bodyMedium,
                    )
                }
            } else {
                NotesContent(layout, visible, onOpenNote)
            }
        }
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
) {
    when (layout) {
        NotesLayout.LIST -> LazyColumn(
            contentPadding = PaddingValues(horizontal = 16.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            items(documents, key = { it.id }) { doc -> NoteRow(doc) { onOpenNote(doc.id) } }
        }

        NotesLayout.CARDS -> LazyVerticalGrid(
            columns = GridCells.Fixed(2),
            contentPadding = PaddingValues(16.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            items(documents, key = { it.id }) { doc -> NoteCard(doc) { onOpenNote(doc.id) } }
        }

        NotesLayout.GALLERY -> LazyVerticalGrid(
            columns = GridCells.Fixed(2),
            contentPadding = PaddingValues(16.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            items(documents, key = { it.id }) { doc -> GalleryTile(doc) { onOpenNote(doc.id) } }
        }

        NotesLayout.ICONS -> LazyVerticalGrid(
            columns = GridCells.Fixed(3),
            contentPadding = PaddingValues(16.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            items(documents, key = { it.id }) { doc -> IconTile(doc) { onOpenNote(doc.id) } }
        }
    }
}

@Composable
private fun NoteRow(doc: Document, onClick: () -> Unit) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(14.dp),
        modifier = Modifier.fillMaxWidth().clickable(onClick = onClick),
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
                    shortDate(doc.updatedAt),
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

@Composable
private fun NoteCard(doc: Document, onClick: () -> Unit) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(16.dp),
        modifier = Modifier.fillMaxWidth().heightIn(min = 112.dp).clickable(onClick = onClick),
    ) {
        Column(Modifier.fillMaxWidth().padding(14.dp)) {
            Text(
                doc.title.ifBlank { "Untitled note" },
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
                maxLines = 3,
                overflow = TextOverflow.Ellipsis,
            )
            Spacer(Modifier.height(18.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    shortDate(doc.updatedAt),
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

@Composable
private fun GalleryTile(doc: Document, onClick: () -> Unit) {
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant,
        shape = RoundedCornerShape(18.dp),
        modifier = Modifier.fillMaxWidth().height(164.dp).clickable(onClick = onClick),
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
                    shortDate(doc.updatedAt),
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

@Composable
private fun IconTile(doc: Document, onClick: () -> Unit) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(16.dp),
        modifier = Modifier.fillMaxWidth().height(104.dp).clickable(onClick = onClick),
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
private fun CreateStack(open: Boolean, onToggle: () -> Unit, onCreate: (CreateKind) -> Unit) {
    Column(horizontalAlignment = Alignment.End, verticalArrangement = Arrangement.spacedBy(10.dp)) {
        if (open) {
            CreateKind.entries.forEach { kind ->
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Surface(
                        color = MaterialTheme.colorScheme.surface,
                        shape = RoundedCornerShape(20.dp),
                        shadowElevation = 2.dp,
                    ) {
                        Text(
                            kind.label,
                            modifier = Modifier.padding(horizontal = 14.dp, vertical = 8.dp),
                            style = MaterialTheme.typography.labelLarge,
                        )
                    }
                    Spacer(Modifier.width(10.dp))
                    SmallFloatingActionButton(
                        onClick = { onCreate(kind) },
                        containerColor = MaterialTheme.colorScheme.surface,
                        contentColor = MaterialTheme.colorScheme.primary,
                    ) {
                        Text(kind.emoji, style = MaterialTheme.typography.titleMedium)
                    }
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

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun <T> OptionSheet(
    title: String,
    options: List<Pair<T, String>>,
    selected: T,
    onSelect: (T) -> Unit,
    onDismiss: () -> Unit,
) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 32.dp),
        ) {
            Text(
                title,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
            )
            Spacer(Modifier.height(8.dp))
            options.forEach { (value, label) ->
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { onSelect(value) }
                        .padding(vertical = 14.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        label,
                        style = MaterialTheme.typography.bodyLarge,
                        modifier = Modifier.weight(1f),
                    )
                    if (value == selected) {
                        Icon(
                            Icons.Filled.Check,
                            contentDescription = null,
                            tint = MaterialTheme.colorScheme.primary,
                        )
                    }
                }
            }
        }
    }
}

/** `2026-10-17T…` -> `2026-10-17`; relative formatting lands with the sync work. */
private fun shortDate(iso: String): String = iso.take(10)
