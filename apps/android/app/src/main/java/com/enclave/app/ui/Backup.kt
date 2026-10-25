package com.enclave.app.ui

import android.content.Context
import android.net.Uri
import java.io.ByteArrayOutputStream

/** Hard ceiling accepted from the SAF picker before the bytes reach the core. */
internal const val MAX_BACKUP_BYTES = 512L * 1024 * 1024

/** Reads a picked vault backup, refusing anything absurd before it reaches the core. */
internal fun readBackupBytes(context: Context, uri: Uri): ByteArray {
    context.contentResolver.openInputStream(uri).use { input ->
        requireNotNull(input) { "the chosen file could not be opened" }
        val out = ByteArrayOutputStream()
        val buffer = ByteArray(64 * 1024)
        var total = 0L
        while (true) {
            val read = input.read(buffer)
            if (read <= 0) break
            total += read
            require(total <= MAX_BACKUP_BYTES) { "that file is too large to be a vault backup" }
            out.write(buffer, 0, read)
        }
        return out.toByteArray()
    }
}
