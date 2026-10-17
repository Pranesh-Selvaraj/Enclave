package com.enclave.app.editor

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import org.json.JSONArray
import org.json.JSONObject

/**
 * The editor model for the core's block contract: a TipTap/ProseMirror document
 * (`Block.contentJson`). It parses the JSON into editable nodes and serialises
 * back to exactly the same shape, so the vault, sync and the web app keep
 * reading what the native editor writes (ADR 0006).
 */

/** The inline marks of one character, mirroring the ProseMirror mark set. */
data class CharStyle(
    val bold: Boolean = false,
    val italic: Boolean = false,
    val strike: Boolean = false,
    val code: Boolean = false,
    val link: String? = null,
) {
    fun toMarks(): JSONArray = JSONArray().apply {
        if (bold) put(JSONObject().put("type", "bold"))
        if (italic) put(JSONObject().put("type", "italic"))
        if (strike) put(JSONObject().put("type", "strike"))
        if (code) put(JSONObject().put("type", "code"))
        if (link != null) {
            put(JSONObject().put("type", "link").put("attrs", JSONObject().put("href", link)))
        }
    }

    companion object {
        val NONE = CharStyle()

        fun fromMarks(marks: JSONArray?): CharStyle {
            if (marks == null) return NONE
            var style = NONE
            for (i in 0 until marks.length()) {
                val mark = marks.optJSONObject(i) ?: continue
                style = when (mark.optString("type")) {
                    "bold" -> style.copy(bold = true)
                    "italic" -> style.copy(italic = true)
                    "strike" -> style.copy(strike = true)
                    "code" -> style.copy(code = true)
                    "link" -> style.copy(link = mark.optJSONObject("attrs")?.optString("href"))
                    else -> style
                }
            }
            return style
        }
    }
}

/**
 * Editable inline content: plain text plus one [CharStyle] per character.
 *
 * Compose state lives here on purpose — the field mutates this from
 * `onValueChange`, and the per-character model keeps existing marks stable
 * while the user types (validated by the Phase 0 Avalonia spike).
 */
class EditableText {
    var text by mutableStateOf("")
        private set

    val styles = mutableStateListOf<CharStyle>()

    fun replace(newText: String) {
        val old = text
        if (old == newText) return

        var prefix = 0
        val maxPrefix = minOf(old.length, newText.length)
        while (prefix < maxPrefix && old[prefix] == newText[prefix]) prefix++

        var suffix = 0
        val maxSuffix = minOf(old.length - prefix, newText.length - prefix)
        while (suffix < maxSuffix && old[old.length - 1 - suffix] == newText[newText.length - 1 - suffix]) suffix++

        val removed = old.length - prefix - suffix
        val inserted = newText.length - prefix - suffix
        val inherited = when {
            prefix > 0 -> styles.getOrElse(prefix - 1) { CharStyle.NONE }
            styles.isNotEmpty() -> styles.first()
            else -> CharStyle.NONE
        }

        repeat(removed) { if (prefix < styles.size) styles.removeAt(prefix) }
        repeat(inserted) { styles.add(prefix, inherited) }

        while (styles.size < newText.length) styles.add(inherited)
        while (styles.size > newText.length) styles.removeAt(styles.lastIndex)

        text = newText
    }

    /**
     * Serialise to text nodes. Paragraphs and headings split on newlines into
     * `hardBreak` nodes; code blocks keep a single text node with `\n` in it,
     * because ProseMirror's `code_block` content is text only.
     */
    fun toJsonNodes(splitNewlines: Boolean = true): JSONArray {
        val nodes = JSONArray()
        if (text.isEmpty()) return nodes

        if (!splitNewlines) {
            appendRuns(nodes, 0, text.length)
            return nodes
        }

        var lineStart = 0
        while (true) {
            val newline = text.indexOf('\n', lineStart)
            val lineEnd = if (newline == -1) text.length else newline
            appendRuns(nodes, lineStart, lineEnd)
            if (newline == -1) break
            nodes.put(JSONObject().put("type", "hardBreak"))
            lineStart = newline + 1
        }
        return nodes
    }

    private fun appendRuns(nodes: JSONArray, start: Int, end: Int) {
        var i = start
        while (i < end) {
            val style = styles.getOrElse(i) { CharStyle.NONE }
            var j = i + 1
            while (j < end && styles.getOrElse(j) { CharStyle.NONE } == style) j++
            val node = JSONObject().put("type", "text").put("text", text.substring(i, j))
            val marks = style.toMarks()
            if (marks.length() > 0) node.put("marks", marks)
            nodes.put(node)
            i = j
        }
    }

    companion object {
        fun fromNodes(content: JSONArray?): EditableText {
            val editable = EditableText()
            if (content == null) return editable

            val builder = StringBuilder()
            val loaded = mutableListOf<CharStyle>()
            for (i in 0 until content.length()) {
                val node = content.optJSONObject(i) ?: continue
                when (node.optString("type")) {
                    "text" -> {
                        val value = node.optString("text")
                        val style = CharStyle.fromMarks(node.optJSONArray("marks"))
                        builder.append(value)
                        repeat(value.length) { loaded.add(style) }
                    }

                    "hardBreak" -> {
                        builder.append('\n')
                        loaded.add(CharStyle.NONE)
                    }
                }
            }
            editable.text = builder.toString()
            editable.styles.addAll(loaded)
            return editable
        }
    }
}

sealed interface EditorBlock {
    val type: String
    val attrs: MutableMap<String, Any?>
}

/** A leaf that holds editable text: paragraph, heading or code block. */
class TextBlock(
    override val type: String,
    override val attrs: MutableMap<String, Any?>,
    val content: EditableText,
) : EditorBlock

/** A node with child blocks: lists, task lists, quotes, tables, list items. */
class ContainerBlock(
    override val type: String,
    override val attrs: MutableMap<String, Any?>,
    val children: MutableList<EditorBlock>,
) : EditorBlock

/** A node with no editable content: horizontal rule, image. */
class AtomBlock(
    override val type: String,
    override val attrs: MutableMap<String, Any?>,
) : EditorBlock

/** The whole document, serialisable back to the ProseMirror contract. */
class EditorDocument(val nodes: MutableList<EditorBlock>) {

    fun toJson(): String {
        val content = JSONArray()
        nodes.forEach { content.put(it.toJson()) }
        return JSONObject().put("type", "doc").put("content", content).toString()
    }

    companion object {
        fun empty(): EditorDocument =
            EditorDocument(mutableListOf(TextBlock("paragraph", mutableMapOf(), EditableText())))

        fun parse(json: String?): EditorDocument {
            val root = json?.let { runCatching { JSONObject(it) }.getOrNull() } ?: return empty()
            if (root.optString("type") != "doc") return empty()

            val content = root.optJSONArray("content") ?: return empty()
            val nodes = mutableListOf<EditorBlock>()
            for (i in 0 until content.length()) {
                val node = content.optJSONObject(i) ?: continue
                nodes.add(parseBlock(node))
            }
            if (nodes.isEmpty()) nodes.add(TextBlock("paragraph", mutableMapOf(), EditableText()))
            return EditorDocument(nodes)
        }

        private fun parseBlock(node: JSONObject): EditorBlock {
            val type = node.optString("type")
            val attrs = attrsOf(node)
            val content = node.optJSONArray("content")
            return when (type) {
                "paragraph", "heading", "codeBlock" ->
                    TextBlock(type, attrs, EditableText.fromNodes(content))

                "horizontalRule", "image" -> AtomBlock(type, attrs)

                else -> {
                    val children = mutableListOf<EditorBlock>()
                    if (content != null) {
                        for (i in 0 until content.length()) {
                            val child = content.optJSONObject(i) ?: continue
                            children.add(parseBlock(child))
                        }
                    }
                    ContainerBlock(type, attrs, children)
                }
            }
        }
    }
}

private fun EditorBlock.toJson(): JSONObject = JSONObject().put("type", type).apply {
    if (attrs.isNotEmpty()) put("attrs", jsonOf(attrs))
    when (this@toJson) {
        is TextBlock -> {
            val nodes = content.toJsonNodes(splitNewlines = type != "codeBlock")
            if (nodes.length() > 0) put("content", nodes)
        }

        is ContainerBlock -> {
            val children = JSONArray()
            this@toJson.children.forEach { children.put(it.toJson()) }
            if (children.length() > 0) put("content", children)
        }

        is AtomBlock -> Unit
    }
}

private fun attrsOf(node: JSONObject): MutableMap<String, Any?> {
    val attrs = node.optJSONObject("attrs") ?: return mutableMapOf()
    val map = mutableMapOf<String, Any?>()
    val keys = attrs.keys()
    while (keys.hasNext()) {
        val key = keys.next()
        // `get` keeps org.json's NULL sentinel, so an explicit `null` attr
        // (e.g. image title) round-trips instead of being dropped.
        map[key] = attrs.get(key)
    }
    return map
}

private fun jsonOf(map: Map<String, Any?>): JSONObject {
    val obj = JSONObject()
    map.forEach { (key, value) -> obj.put(key, value) }
    return obj
}
