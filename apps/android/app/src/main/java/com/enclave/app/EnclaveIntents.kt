package com.enclave.app

import android.content.Context
import android.content.Intent

/**
 * Android entry points into the full Enclave app (the Tauri web view).
 * Widgets, the Quick Settings tile, launcher shortcuts and the share sheet all
 * route here; [MainActivity] turns them into app routes.
 */
internal object EnclaveIntents {
    const val ACTION_CAPTURE = "com.enclave.app.CAPTURE"
    const val ACTION_NEW_NOTE = "com.enclave.app.NEW_NOTE"
    const val ACTION_NEW_CHECKLIST = "com.enclave.app.NEW_CHECKLIST"
    const val ACTION_OPEN_NOTE = "com.enclave.app.OPEN_NOTE"
    const val EXTRA_DOC_ID = "enclave:docId"
    const val EXTRA_TEXT = "enclave:text"

    private fun base(context: Context, action: String): Intent =
        Intent(context, MainActivity::class.java)
            .setAction(action)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)

    /** Quick capture screen, optionally pre-filled (share target, widgets). */
    fun capture(context: Context, text: String? = null): Intent =
        base(context, ACTION_CAPTURE).apply { if (!text.isNullOrBlank()) putExtra(EXTRA_TEXT, text) }

    /** Open a specific note in the app. */
    fun openNote(context: Context, docId: String): Intent =
        base(context, ACTION_OPEN_NOTE).putExtra(EXTRA_DOC_ID, docId)
}
