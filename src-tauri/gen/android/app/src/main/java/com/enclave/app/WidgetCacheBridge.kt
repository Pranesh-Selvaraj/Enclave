package com.enclave.app

import android.content.Context
import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import uniffi.core_api.FfiCore

/**
 * Core → widget refresh bridge (issue #127).
 *
 * Widget updates used to be a 20 s poll in MainActivity that woke the process
 * even when nothing changed. The Rust shell now calls [onCoreChanged] after
 * vault writes and locks (JNI), and this object performs one refresh per burst
 * (the channel is conflated) — so the cache is fresh exactly when the vault
 * changed. A slow fallback tick in MainActivity covers anything that did not
 * emit.
 */
internal object WidgetCacheBridge {
    private const val TAG = "EnclaveWidgets"
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private val signal = Channel<Unit>(Channel.CONFLATED)

    @Volatile
    private var appContext: Context? = null

    /** Called once by MainActivity; safe to call again. */
    fun install(context: Context) {
        appContext = context.applicationContext
        scope.launch {
            while (isActive) {
                signal.receive()
                val ctx = appContext ?: continue
                try {
                    WidgetStore.syncFromCore(ctx, FfiCore(ctx.applicationInfo.dataDir))
                } catch (t: Throwable) {
                    // A locked or mid-flight core must never take the process
                    // down; the next signal (or the fallback) retries.
                    Log.w(TAG, "widget refresh skipped: " + t.message, t)
                }
            }
        }
    }

    /** Ask for a refresh; a burst of saves collapses into one. */
    fun requestRefresh() {
        signal.trySend(Unit)
    }

    /** JNI entry point called from Rust (`android_sync::widgets_changed`). */
    @JvmStatic
    fun onCoreChanged() = requestRefresh()
}
