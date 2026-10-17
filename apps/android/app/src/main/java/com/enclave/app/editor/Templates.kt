package com.enclave.app.editor

import org.json.JSONArray
import org.json.JSONObject

/** What the create stack offers. */
enum class CreateKind(val label: String, val emoji: String) {
    TEXT("Text", "📝"),
    CHECKLIST("Checklist", "✅"),
    JOURNAL("Journal", "✍️"),
    MEETING("Meeting notes", "📋"),
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

        CreateKind.JOURNAL -> Template(
            "Journal",
            doc(
                h1("Journal"),
                p(),
                h2("What went well?"),
                p(),
                h2("What could be better?"),
                p(),
                h2("Tomorrow"),
                task(),
            ),
        )

        CreateKind.MEETING -> Template(
            "Meeting notes",
            doc(
                h1("Meeting notes"),
                p("Date:  •  Attendees: "),
                h2("Agenda"),
                bullet(),
                bullet(),
                h2("Discussion"),
                p(),
                h2("Action items"),
                task(),
                task(),
            ),
        )

        CreateKind.TEXT, CreateKind.FOLDER -> Template("", doc(p()))
    }

    private fun doc(vararg nodes: JSONObject): JSONObject = JSONObject()
        .put("type", "doc")
        .put("content", JSONArray().apply { nodes.forEach { put(it) } })

    private fun p(text: String = ""): JSONObject = JSONObject().put("type", "paragraph").apply {
        if (text.isNotEmpty()) put("content", JSONArray().put(textNode(text)))
    }

    private fun h1(text: String) = heading(1, text)
    private fun h2(text: String) = heading(2, text)

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
