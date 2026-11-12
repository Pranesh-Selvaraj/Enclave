package com.enclave.app.ui

import com.enclave.app.core.Core
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.util.UUID

/**
 * The emoji a note shows in the Icons layout.
 *
 * It is stored as an `icon` block — the same mechanism the per-page widget
 * policy uses (`widget`), so it needs no schema change and rides along with the
 * document through sync and backup. The block is synthetic in the same sense:
 * the editor never renders it, and a build that does not know the type simply
 * ignores it.
 */
object NoteIcon {
    private const val BLOCK_TYPE = "icon"

    suspend fun load(documentId: String): String? = withContext(Dispatchers.IO) {
        runCatching {
            Core.ffi.getBlocks(documentId)
                .firstOrNull { it.blockType == BLOCK_TYPE }
                ?.let { JSONObject(it.contentJson).optString("emoji") }
                ?.takeIf { it.isNotBlank() }
        }.getOrNull()
    }

    /** Store [emoji] for the note, or remove the icon when it is blank. */
    suspend fun save(documentId: String, emoji: String) = withContext(Dispatchers.IO) {
        runCatching {
            val existing = Core.ffi.getBlocks(documentId).firstOrNull { it.blockType == BLOCK_TYPE }
            when {
                emoji.isBlank() -> existing?.let { Core.ffi.deleteBlock(it.id) }
                else -> Core.ffi.upsertBlock(
                    existing?.id ?: UUID.randomUUID().toString(),
                    documentId,
                    BLOCK_TYPE,
                    JSONObject().put("emoji", emoji).toString(),
                    existing?.sortOrder ?: 1.0,
                )
            }
        }
        Unit
    }
}
