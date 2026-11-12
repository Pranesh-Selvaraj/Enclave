package com.enclave.app.ui

import com.enclave.app.core.Core
import uniffi.core_api.Document

/** The four notes layouts, matching the shipping app. */
enum class NotesLayout(val id: String, val label: String, val emoji: String) {
    LIST("list", "List", "📄"),
    GALLERY("gallery", "Gallery", "🖼"),
    CARDS("cards", "Cards", "🗂"),
    ICONS("icons", "Icons", "🔳");

    companion object {
        fun fromId(id: String?): NotesLayout = entries.firstOrNull { it.id == id } ?: CARDS
    }
}

/** The three sort orders. */
enum class NotesSort(val id: String, val label: String) {
    UPDATED("updated", "Recently updated"),
    CREATED("created", "Date created"),
    TITLE("title", "Title");

    companion object {
        fun fromId(id: String?): NotesSort = entries.firstOrNull { it.id == id } ?: UPDATED
    }
}

fun NotesSort.comparator(): Comparator<Document> = when (this) {
    NotesSort.UPDATED -> compareByDescending { it.updatedAt }
    NotesSort.CREATED -> compareByDescending { it.createdAt }
    NotesSort.TITLE -> compareBy(String.CASE_INSENSITIVE_ORDER) { it.title.ifBlank { "Untitled note" } }
}

/**
 * Notes-view preferences. They live in the vault's settings table, like every
 * other setting, so they sync with the rest of the vault.
 */
object NotesPrefs {
    private const val LAYOUT_KEY = "android.notes_layout"
    private const val SORT_KEY = "android.notes_sort"

    // Reading a setting goes through the vault, so the first composition after
    // returning from a page used to start on the default layout and visibly
    // flip to the chosen one. These caches let the home start on the right
    // layout/sort; the vault stays the source of truth and refreshes them.
    @Volatile
    private var cachedLayout: NotesLayout? = null

    @Volatile
    private var cachedSort: NotesSort? = null

    /** The last known choice, for a first frame that is already correct. */
    val layoutNow: NotesLayout get() = cachedLayout ?: NotesLayout.CARDS

    val sortNow: NotesSort get() = cachedSort ?: NotesSort.UPDATED

    fun loadLayout(): NotesLayout = NotesLayout.fromId(read(LAYOUT_KEY)).also { cachedLayout = it }

    fun saveLayout(layout: NotesLayout) {
        cachedLayout = layout
        write(LAYOUT_KEY, layout.id)
    }

    fun loadSort(): NotesSort = NotesSort.fromId(read(SORT_KEY)).also { cachedSort = it }

    fun saveSort(sort: NotesSort) {
        cachedSort = sort
        write(SORT_KEY, sort.id)
    }

    private fun read(key: String): String? = runCatching { Core.ffi.getSetting(key) }.getOrNull()

    private fun write(key: String, value: String) {
        runCatching { Core.ffi.setSetting(key, value) }
    }
}
