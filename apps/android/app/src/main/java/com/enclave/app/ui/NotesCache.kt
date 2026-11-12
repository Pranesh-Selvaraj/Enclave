package com.enclave.app.ui

import uniffi.core_api.Document
import uniffi.core_api.Folder
import java.util.concurrent.ConcurrentHashMap

/**
 * The note list and its previews, kept for the lifetime of an unlocked session.
 *
 * Rebuilding the home screen on every return meant one frame with the header
 * and no notes, then the list appearing — the layout was cached, the *content*
 * was not. This holds the last list, the folders and the rendered previews so
 * the first frame after a return is the finished screen; the vault stays the
 * source of truth and overwrites all of it on every load.
 *
 * Note titles and bodies are vault data, so this is cleared whenever the vault
 * locks (and on restore): nothing survives the lock screen.
 */
object NotesCache {
    @Volatile
    var documents: List<Document>? = null

    @Volatile
    var folders: List<Folder>? = null

    /** document id -> (updatedAt, plain text) so an unchanged note reuses its preview. */
    private val previews = ConcurrentHashMap<String, Pair<String, String>>()

    fun preview(documentId: String, updatedAt: String): String? =
        previews[documentId]?.takeIf { it.first == updatedAt }?.second

    fun putPreview(documentId: String, updatedAt: String, text: String) {
        previews[documentId] = updatedAt to text
    }

    fun forget(documentId: String) {
        previews.remove(documentId)
    }

    fun clear() {
        documents = null
        folders = null
        previews.clear()
    }
}
