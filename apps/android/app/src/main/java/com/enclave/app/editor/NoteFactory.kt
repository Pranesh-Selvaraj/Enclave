package com.enclave.app.editor

import com.enclave.app.core.Core
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.core_api.Document
import org.json.JSONObject
import java.util.UUID

/**
 * Creates a note the way every entry point expects: the document plus its
 * canonical `doc` block. The block the core seeds on create is promoted in
 * place, so a note never carries two content blocks.
 */
object NoteFactory {
    suspend fun create(
        kind: CreateKind,
        folderId: String? = null,
        prefill: String? = null,
    ): Document = create(template = Templates.forKind(kind), folderId = folderId, prefill = prefill)

    /** Start a note from a template the user saved. */
    suspend fun createFromTemplate(
        template: UserTemplate,
        folderId: String? = null,
    ): Document = create(
        template = Templates.Template(
            title = template.name,
            doc = JSONObject(template.doc),
        ),
        folderId = folderId,
    )

    /**
     * Shared text becomes the first non-empty line, capped like a title in
     * the rest of the UI. If the text starts with blank lines, fall back to
     * the template title instead of "Untitled note" and a hidden body cue.
     */
    internal fun sharedTitle(text: String?, fallback: String): String {
        val first = text
            ?.lineSequence()
            ?.map { it.trim() }
            ?.firstOrNull { it.isNotEmpty() }
            .orEmpty()
        return first.take(80).ifBlank { fallback }
    }

    private suspend fun create(
        template: Templates.Template,
        folderId: String? = null,
        prefill: String? = null,
    ): Document = withContext(Dispatchers.IO) {
        val title = if (prefill.isNullOrBlank()) template.title else sharedTitle(prefill, template.title)
        var doc = Core.ffi.createDocument(title)

        val seed = Core.ffi.getBlocks(doc.id).firstOrNull()
        val blockId = seed?.id ?: UUID.randomUUID().toString()
        val content = if (prefill.isNullOrBlank()) template.doc else Templates.prefilled(prefill)
        Core.ffi.upsertBlock(
            blockId,
            doc.id,
            "doc",
            content.toString(),
            seed?.sortOrder ?: 1.0,
        )

        if (folderId != null) doc = Core.ffi.moveDocument(doc.id, folderId)
        doc
    }
}
