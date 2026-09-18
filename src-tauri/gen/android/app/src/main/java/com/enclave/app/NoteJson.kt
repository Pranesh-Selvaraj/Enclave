package com.enclave.app

import org.json.JSONArray
import org.json.JSONObject

// Note content helpers shared by the Keep shell and the widget cache.
// Everything is plain TipTap/ProseMirror JSON — the same shape the web
// editor reads and writes.

/** One checklist row. */
internal data class TaskItem(val text: String, val checked: Boolean)

/** Recursively collect `text` leaves from a TipTap node. */
internal fun jsonText(node: JSONObject, out: StringBuilder = StringBuilder()): String {
    when (node.optString("type")) {
        "text" -> out.append(node.optString("text"))
        "hardBreak" -> out.append("\n")
        // Checklist rows are siblings; keep their texts apart in snippets.
        "taskItem" -> out.append(" ")
    }
    node.optJSONArray("content")?.let { children ->
        for (i in 0 until children.length()) {
            (children.opt(i) as? JSONObject)?.let { jsonText(it, out) }
            if (children.opt(i) is String) out.append(children.optString(i))
        }
    }
    return out.toString()
}

/** Paragraph text (blocks joined by blank lines) for the flat editor. */
internal fun docBodyText(docJson: String?): String {
    if (docJson.isNullOrBlank()) return ""
    return try {
        val nodes = JSONObject(docJson).optJSONArray("content") ?: return ""
        val out = StringBuilder()
        for (i in 0 until nodes.length()) {
            val node = nodes.optJSONObject(i) ?: continue
            if (node.optString("type") == "taskList") continue
            val text = jsonText(node).trim()
            if (text.isNotEmpty()) {
                if (out.isNotEmpty()) out.append("\n\n")
                out.append(text)
            }
        }
        out.toString()
    } catch (_: Exception) {
        ""
    }
}

/** Checklist items from a doc JSON (taskList node), empty when none. */
internal fun docChecklist(docJson: String?): List<TaskItem> {
    if (docJson.isNullOrBlank()) return emptyList()
    return try {
        val nodes = JSONObject(docJson).optJSONArray("content") ?: return emptyList()
        val taskList = (0 until nodes.length())
            .mapNotNull { nodes.optJSONObject(it) }
            .firstOrNull { it.optString("type") == "taskList" } ?: return emptyList()
        val items = taskList.optJSONArray("content") ?: return emptyList()
        (0 until items.length()).mapNotNull { i ->
            val item = items.optJSONObject(i) ?: return@mapNotNull null
            if (item.optString("type") != "taskItem") return@mapNotNull null
            TaskItem(
                text = jsonText(item).trim(),
                checked = item.optJSONObject("attrs")?.optBoolean("checked", false) ?: false,
            )
        }
    } catch (_: Exception) {
        emptyList()
    }
}

/** Single-line snippet: text note text, or checklist items joined. */
internal fun docSnippet(docJson: String?): String {
    val body = docBodyText(docJson)
    val text = body.ifBlank { docChecklist(docJson).joinToString(" ") { it.text } }
    return text.replace(Regex("\\s+"), " ").trim()
}

/** Labels from a tags block JSON (`{"tags":[...]}`). */
internal fun tagsOf(blockJson: String?): List<String> {
    if (blockJson.isNullOrBlank()) return emptyList()
    return try {
        val arr = JSONObject(blockJson).optJSONArray("tags") ?: return emptyList()
        (0 until arr.length()).mapNotNull { arr.optString(it).takeIf { s -> s.isNotBlank() } }
    } catch (_: Exception) {
        emptyList()
    }
}

/** Flat text → the doc JSON shape the web editor writes. */
internal fun docJson(text: String): String {
    val paragraphs = text.split("\n\n").map { paragraph ->
        JSONObject().apply {
            put("type", "paragraph")
            val content = JSONArray()
            paragraph.split("\n").forEachIndexed { i, line ->
                if (i > 0) content.put(JSONObject().put("type", "hardBreak"))
                if (line.isNotEmpty()) content.put(JSONObject().put("type", "text").put("text", line))
            }
            put("content", content)
        }
    }
    return JSONObject().put("type", "doc").put("content", JSONArray(paragraphs)).toString()
}

/** Checklist items → a `doc` whose content is one `taskList` node. */
internal fun taskListDocJson(tasks: List<TaskItem>): String {
    val content = JSONArray()
    tasks.forEach { item ->
        content.put(
            JSONObject()
                .put("type", "taskItem")
                .put("attrs", JSONObject().put("checked", item.checked))
                .put(
                    "content",
                    JSONArray().put(
                        JSONObject()
                            .put("type", "paragraph")
                            .put(
                                "content",
                                JSONArray().put(JSONObject().put("type", "text").put("text", item.text)),
                            ),
                    ),
                ),
        )
    }
    val taskList = JSONObject().put("type", "taskList").put("content", content)
    return JSONObject()
        .put("type", "doc")
        .put("content", JSONArray().put(taskList))
        .toString()
}

/** True when the note opted into widgets (a `widget` block with enabled). */
internal fun widgetBlockEnabled(blockJson: String?): Boolean {
    if (blockJson.isNullOrBlank()) return false
    return try {
        JSONObject(blockJson).optBoolean("enabled", false)
    } catch (_: Exception) {
        false
    }
}
