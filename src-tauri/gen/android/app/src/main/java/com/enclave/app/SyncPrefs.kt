package com.enclave.app

import android.content.Context

/**
 * Whether the user turned P2P sync on. Plaintext boolean only — the vault key
 * still has to be entered to actually sync, so this just decides whether the
 * app should try to re-arm the network when it comes back (unlock, network
 * change, resume).
 */
internal object SyncPrefs {
    private const val PREFS = "enclave_sync"
    private const val KEY_ENABLED = "enabled"

    fun setEnabled(context: Context, enabled: Boolean) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit().putBoolean(KEY_ENABLED, enabled).apply()
    }

    fun enabled(context: Context): Boolean =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getBoolean(KEY_ENABLED, false)
}
