package com.enclave.app.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import org.json.JSONArray
import org.json.JSONObject
import uniffi.core_api.Block

/**
 * Read-only renderer for the core's block contract: a TipTap/ProseMirror JSON
 * document (`Block.contentJson`). The same node set the native editor must
 * produce and consume, so this is also the reference for the editor work.
 */
@Composable
fun BlockView(block: Block) {
    val root = remember(block.contentJson) {
        runCatching { JSONObject(block.contentJson) }.getOrNull()
    } ?: return
    if (root.optString("type") != "doc") return
    val content = root.optJSONArray("content") ?: return

    Column(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 20.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        for (i in 0 until content.length()) {
            val node = content.optJSONObject(i) ?: continue
            NodeView(node)
        }
    }
}

@Composable
private fun NodeView(node: JSONObject) {
    when (node.optString("type")) {
        "paragraph" -> Text(
            Inline(node),
            style = MaterialTheme.typography.bodyLarge,
            color = MaterialTheme.colorScheme.onBackground,
        )

        "heading" -> {
            val level = node.optJSONObject("attrs")?.optInt("level", 1) ?: 1
            val style = when (level) {
                1 -> MaterialTheme.typography.headlineSmall
                2 -> MaterialTheme.typography.titleLarge
                else -> MaterialTheme.typography.titleMedium
            }
            Text(
                Inline(node),
                style = style,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.onBackground,
            )
        }

        "bulletList" -> ListNode(node, ordered = false)
        "orderedList" -> ListNode(node, ordered = true)
        "taskList" -> TaskListNode(node)
        "blockquote" -> QuoteNode(node)
        "codeBlock" -> CodeNode(node)
        "horizontalRule" -> HorizontalDivider(
            modifier = Modifier.padding(vertical = 6.dp),
            color = MaterialTheme.colorScheme.outlineVariant,
        )
        "table" -> TableNode(node)
        "image" -> Text("[image]", color = MaterialTheme.colorScheme.onSurfaceVariant)
        else -> node.optJSONArray("content")?.let { children ->
            for (i in 0 until children.length()) {
                val child = children.optJSONObject(i) ?: continue
                NodeView(child)
            }
        }
    }
}

@Composable
private fun Inline(node: JSONObject): AnnotatedString {
    val codeFg = MaterialTheme.colorScheme.primary
    val codeBg = MaterialTheme.colorScheme.surfaceVariant
    val link = MaterialTheme.colorScheme.secondary
    return remember(node, codeFg, codeBg, link) { buildInline(node, codeFg, codeBg, link) }
}

private fun buildInline(
    node: JSONObject,
    codeFg: Color,
    codeBg: Color,
    link: Color,
): AnnotatedString = buildAnnotatedString {
    node.optJSONArray("content")?.let { appendInline(it, codeFg, codeBg, link) }
}

private fun AnnotatedString.Builder.appendInline(
    content: JSONArray,
    codeFg: Color,
    codeBg: Color,
    link: Color,
) {
    for (i in 0 until content.length()) {
        val child = content.optJSONObject(i) ?: continue
        when (child.optString("type")) {
            "hardBreak" -> append("\n")

            "text" -> {
                val start = length
                append(child.optString("text"))
                val marks = child.optJSONArray("marks") ?: continue
                for (m in 0 until marks.length()) {
                    when (marks.optJSONObject(m)?.optString("type")) {
                        "bold" -> addStyle(SpanStyle(fontWeight = FontWeight.Bold), start, length)
                        "italic" -> addStyle(SpanStyle(fontStyle = FontStyle.Italic), start, length)
                        "strike" -> addStyle(SpanStyle(textDecoration = TextDecoration.LineThrough), start, length)
                        "code" -> addStyle(
                            SpanStyle(fontFamily = FontFamily.Monospace, color = codeFg, background = codeBg),
                            start,
                            length,
                        )
                        "link" -> addStyle(
                            SpanStyle(color = link, textDecoration = TextDecoration.Underline),
                            start,
                            length,
                        )
                    }
                }
            }

            else -> child.optJSONArray("content")?.let { appendInline(it, codeFg, codeBg, link) }
        }
    }
}

@Composable
private fun NodeChildren(container: JSONObject) {
    val children = container.optJSONArray("content") ?: return
    for (i in 0 until children.length()) {
        val child = children.optJSONObject(i) ?: continue
        NodeView(child)
    }
}

@Composable
private fun ListNode(node: JSONObject, ordered: Boolean) {
    val items = node.optJSONArray("content") ?: return
    var index = node.optJSONObject("attrs")?.optInt("start", 1) ?: 1
    Column(
        modifier = Modifier.padding(start = 4.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        for (i in 0 until items.length()) {
            val item = items.optJSONObject(i) ?: continue
            Row {
                Text(
                    if (ordered) "$index." else "•",
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.width(24.dp),
                )
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) { NodeChildren(item) }
            }
            index++
        }
    }
}

@Composable
private fun TaskListNode(node: JSONObject) {
    val items = node.optJSONArray("content") ?: return
    Column(
        modifier = Modifier.padding(start = 4.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        for (i in 0 until items.length()) {
            val item = items.optJSONObject(i) ?: continue
            val checked = item.optJSONObject("attrs")?.optBoolean("checked") == true
            Row(verticalAlignment = Alignment.CenterVertically) {
                Checkbox(
                    checked = checked,
                    onCheckedChange = null,
                    modifier = Modifier.size(20.dp),
                )
                Spacer(Modifier.width(10.dp))
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) { NodeChildren(item) }
            }
        }
    }
}

@Composable
private fun QuoteNode(node: JSONObject) {
    Row(Modifier.height(IntrinsicSize.Min)) {
        Box(
            Modifier
                .width(3.dp)
                .fillMaxHeight()
                .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.55f), RoundedCornerShape(2.dp)),
        )
        Spacer(Modifier.width(12.dp))
        Column(verticalArrangement = Arrangement.spacedBy(4.dp)) { NodeChildren(node) }
    }
}

@Composable
private fun CodeNode(node: JSONObject) {
    val language = node.optJSONObject("attrs")?.optString("language").orEmpty()
    val text = buildString {
        val children = node.optJSONArray("content") ?: return@buildString
        for (i in 0 until children.length()) {
            append(children.optJSONObject(i)?.optString("text").orEmpty())
        }
    }
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant,
        shape = RoundedCornerShape(10.dp),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            if (language.isNotBlank()) {
                Text(
                    language,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Text(
                text,
                style = MaterialTheme.typography.bodyMedium,
                fontFamily = FontFamily.Monospace,
                color = MaterialTheme.colorScheme.onSurface,
            )
        }
    }
}

@Composable
private fun TableNode(node: JSONObject) {
    val rows = node.optJSONArray("content") ?: return
    Column(
        Modifier
            .fillMaxWidth()
            .border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(10.dp)),
    ) {
        for (r in 0 until rows.length()) {
            val row = rows.optJSONObject(r) ?: continue
            val cells = row.optJSONArray("content") ?: continue
            Row(Modifier.fillMaxWidth()) {
                for (c in 0 until cells.length()) {
                    val cell = cells.optJSONObject(c) ?: continue
                    val header = cell.optString("type") == "tableHeader"
                    Box(
                        Modifier
                            .weight(1f)
                            .background(
                                if (header) MaterialTheme.colorScheme.surfaceVariant else Color.Transparent,
                            )
                            .padding(horizontal = 10.dp, vertical = 8.dp),
                    ) {
                        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) { NodeChildren(cell) }
                    }
                }
            }
            if (r < rows.length() - 1) {
                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            }
        }
    }
}
