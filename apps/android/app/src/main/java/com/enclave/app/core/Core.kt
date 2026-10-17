package com.enclave.app.core

import android.content.Context
import uniffi.core_api.FfiCore

/**
 * The one Rust core instance for the process.
 *
 * [FfiCore] is backed by `EnclaveCore::global(appDir)`, so this is the same
 * vault, network stack and sync loop the shipping app used. The data directory
 * is the app's own (`com.enclave.app`), which is what makes an existing vault
 * carry over without a migration.
 */
object Core {
    @Volatile
    private var instance: FfiCore? = null

    val ffi: FfiCore
        get() = instance ?: error("Core.init() has not run")

    fun init(context: Context) {
        if (instance == null) {
            synchronized(this) {
                if (instance == null) {
                    instance = FfiCore(context.applicationInfo.dataDir)
                }
            }
        }
    }
}
