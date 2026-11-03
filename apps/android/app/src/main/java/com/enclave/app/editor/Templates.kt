package com.enclave.app.editor

import com.enclave.app.core.Core
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.util.UUID

/** What the create stack offers. */
enum class CreateKind(val label: String, val emoji: String) {
    TEXT("Text", "📝"),
    CHECKLIST("Checklist", "✅"),
    FOLDER("Folder", "📁"),
}

/**
 * Starter documents for the create stack. They are the same plain ProseMirror
 * JSON the core stores, mirroring `packages/editor/src/templates.ts`, so a note
 * started on the phone opens identically on the desktop.
 */
object Templates {
    data class Template(val title: String, val doc: JSONObject)

    fun forKind(kind: CreateKind): Template = when (kind) {
        CreateKind.CHECKLIST -> Template("Checklist", doc(h1("Checklist"), task(), task(), task()))

        CreateKind.TEXT, CreateKind.FOLDER -> Template("", doc(p()))
    }

    /** A document holding one paragraph with [text] (share target, capture). */
    fun prefilled(text: String): JSONObject = doc(p(text))

    private fun doc(vararg nodes: JSONObject): JSONObject = JSONObject()
        .put("type", "doc")
        .put("content", JSONArray().apply { nodes.forEach { put(it) } })

    private fun p(text: String = ""): JSONObject = JSONObject().put("type", "paragraph").apply {
        if (text.isNotEmpty()) put("content", JSONArray().put(textNode(text)))
    }

    private fun h1(text: String) = heading(1, text)
    private fun heading(level: Int, text: String): JSONObject = JSONObject()
        .put("type", "heading")
        .put("attrs", JSONObject().put("level", level))
        .put("content", JSONArray().put(textNode(text)))

    private fun bullet(text: String = ""): JSONObject = JSONObject()
        .put("type", "bulletList")
        .put("content", JSONArray().put(listItem(text)))

    private fun listItem(text: String): JSONObject = JSONObject()
        .put("type", "listItem")
        .put("content", JSONArray().put(p(text)))

    private fun task(text: String = ""): JSONObject = JSONObject()
        .put("type", "taskList")
        .put("content", JSONArray().put(taskItem(text)))

    private fun taskItem(text: String): JSONObject = JSONObject()
        .put("type", "taskItem")
        .put("attrs", JSONObject().put("checked", false))
        .put("content", JSONArray().put(p(text)))

    private fun textNode(text: String): JSONObject =
        JSONObject().put("type", "text").put("text", text)
}

/**
 * A template the user made from a note. [doc] is the note's canonical document
 * JSON, so a page started from it is byte-identical to the note it came from.
 */
data class UserTemplate(val id: String, val name: String, val doc: String) {
    fun toJson(): JSONObject = JSONObject()
        .put("id", id)
        .put("name", name)
        .put("doc", doc)

    companion object {
        fun fromJson(json: JSONObject): UserTemplate? {
            val id = json.optString("id")
            val name = json.optString("name")
            val doc = json.optString("doc")
            if (id.isBlank() || name.isBlank() || doc.isBlank()) return null
            // A template must still parse as a document; a half-migrated or
            // hand-edited vault should lose the template, not break the sheet.
            val parsed = runCatching { JSONObject(doc) }.getOrNull() ?: return null
            if (parsed.optString("type") != "doc") return null
            return UserTemplate(id, name, doc)
        }
    }
}

/** Where the create stack can start from. */
sealed interface CreateTarget {
    data class Builtin(val kind: CreateKind) : CreateTarget
    data class Custom(val template: UserTemplate) : CreateTarget
}

/**
 * User templates live in the vault's settings table, next to the notes layout
 * and sort order, so they travel with the vault and need no schema change
 * (ADR 0010). The whole list is one JSON value under one key: templates are
 * small, edited rarely, and a single key keeps the contract untouched.
 */
object TemplateStore {
    private const val KEY = "android.templates.v1"
    private const val MAX_TEMPLATES = 50
    private const val MAX_NAME = 60

    suspend fun load(): List<UserTemplate> = withContext(Dispatchers.IO) {
        val raw = runCatching { Core.ffi.getSetting(KEY) }.getOrNull() ?: return@withContext emptyList()
        val array = runCatching { JSONArray(raw) }.getOrNull() ?: return@withContext emptyList()
        (0 until array.length()).mapNotNull { index ->
            array.optJSONObject(index)?.let(UserTemplate::fromJson)
        }
    }

    /** Store [name] + [docJson] as a new template and return the saved list. */
    suspend fun add(name: String, docJson: String): List<UserTemplate> = withContext(Dispatchers.IO) {
        val clean = name.trim().take(MAX_NAME).ifBlank { "Template" }
        val next = (load() + UserTemplate(UUID.randomUUID().toString(), clean, docJson))
            .takeLast(MAX_TEMPLATES)
        save(next)
        next
    }

    suspend fun remove(id: String): List<UserTemplate> = withContext(Dispatchers.IO) {
        val next = load().filterNot { it.id == id }
        save(next)
        next
    }

    private fun save(templates: List<UserTemplate>) {
        val array = JSONArray()
        templates.forEach { array.put(it.toJson()) }
        runCatching { Core.ffi.setSetting(KEY, array.toString()) }
    }
}
