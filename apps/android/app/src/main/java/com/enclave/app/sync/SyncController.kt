package com.enclave.app.sync

import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.enclave.app.SyncService
import com.enclave.app.core.Core
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

/**
 * Owns the P2P sync lifecycle: the core's network, its sync loop and the
 * foreground service that keeps both alive while the app is backgrounded.
 *
 * The user's choice is device-local (SharedPreferences) — it is a property of
 * this phone, not of the vault, so it is not stored in the vault settings.
 */
object SyncController {
    private const val PREFS = "enclave-sync"
    private const val KEY_ENABLED = "enabled"

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var loop: Job? = null

    /** Exposed as Compose state so the UI can react to the toggle. */
    var enabled by mutableStateOf(false)
        private set

    fun refresh(context: Context) {
        enabled = prefs(context).getBoolean(KEY_ENABLED, false)
    }

    /** Start (or restart) the network, the sync loop and the service. */
    fun enable(context: Context) {
        prefs(context).edit().putBoolean(KEY_ENABLED, true).apply()
        enabled = true
        scope.launch {
            runCatching { Core.ffi.startNetwork(deviceName()) }
            if (loop?.isActive != true) {
                loop = scope.launch { runCatching { Core.ffi.runSyncLoop() } }
            }
        }
        startService(context)
    }

    /** Turn sync off: stop the loop, the network and the service. */
    fun disable(context: Context) {
        prefs(context).edit().putBoolean(KEY_ENABLED, false).apply()
        enabled = false
        stopRuntime(context)
    }

    /** Stop for a locked vault but keep the choice, so unlock can re-arm it. */
    fun pause(context: Context) {
        stopRuntime(context)
    }

    /** Re-arm after unlock or a process restart when sync was on. */
    fun resumeIfEnabled(context: Context) {
        refresh(context)
        if (enabled && Core.ffi.isUnlocked()) enable(context)
    }

    private fun stopRuntime(context: Context) {
        loop?.cancel()
        loop = null
        scope.launch { runCatching { Core.ffi.stopNetwork() } }
        context.stopService(Intent(context, SyncService::class.java))
    }

    private fun startService(context: Context) {
        val intent = Intent(context, SyncService::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            context.startForegroundService(intent)
        } else {
            context.startService(intent)
        }
    }

    private fun prefs(context: Context) =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    private fun deviceName(): String = (Build.MODEL ?: "Android").take(40)
}
