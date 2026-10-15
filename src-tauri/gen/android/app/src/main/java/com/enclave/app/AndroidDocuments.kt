package com.enclave.app

import android.content.Context
import android.net.Uri
import android.util.Log

/**
 * ContentResolver bridge for user-picked files (issue #38).
 *
 * The Tauri dialog plugin returns `content://` URIs on Android (both the
 * open and the create-document pickers), but the vault's import/export code
 * used plain filesystem paths, so every phone import/export failed. Rust
 * detects `content://` and routes the bytes through these methods instead.
 */
internal object AndroidDocuments {
    private const val TAG = "EnclaveFiles"

    @Volatile
    private var appContext: Context? = null

    fun install(context: Context) {
        appContext = context.applicationContext
    }

    /** Read the bytes behind a SAF URI; null on any failure. */
    @JvmStatic
    fun readUri(uri: String): ByteArray? = try {
        appContext?.contentResolver?.openInputStream(Uri.parse(uri))?.use { it.readBytes() }
    } catch (t: Throwable) {
        Log.e(TAG, "failed to read $uri", t)
        null
    }

    /** Write/truncate the bytes behind a SAF URI; false on any failure. */
    @JvmStatic
    fun writeUri(uri: String, data: ByteArray): Boolean = try {
        appContext?.contentResolver?.openOutputStream(Uri.parse(uri), "wt")?.use {
            it.write(data)
        } != null
    } catch (t: Throwable) {
        Log.e(TAG, "failed to write $uri", t)
        false
    }
}
