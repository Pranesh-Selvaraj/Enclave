package com.enclave.app.ui

import uniffi.core_api.Folder
import uniffi.core_api.HomeNote
/**
 * The note list, kept for the lifetime of an unlocked session.
 *
 * Rebuilding the home screen on every return meant one frame with the header
 * and no notes, then the list appearing — the layout was cached, the *content*
 * was not. This holds the last list — which already carries each note's emoji
 * and preview — so the first frame after a return is the finished screen; the
 * vault stays the source of truth and overwrites it on every load.
 *
 * Note titles and bodies are vault data, so this is cleared whenever the vault
 * locks (and on restore): nothing survives the lock screen.
 */
object NotesCache {
    @Volatile
    var documents: List<HomeNote>? = null

    @Volatile
    var folders: List<Folder>? = null

    fun clear() {
        documents = null
        folders = null
    }
}
