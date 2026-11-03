package com.enclave.app.editor

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.heightIn
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
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.Checkbox
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.font.FontStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.OffsetMapping
import androidx.compose.ui.text.input.TransformedText
import androidx.compose.ui.text.input.VisualTransformation
import com.enclave.app.ui.EnclaveMono
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp

/**
 * The editable document surface. Containers render their children, leaves are
 * `BasicTextField`s whose marks are applied by [MarkTransformation], so the
 * on-disk JSON and the visible formatting cannot drift apart.
 */
@Composable
fun EditableDocument(
    doc: EditorDocument,
    onChanged: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Column(modifier, verticalArrangement = Arrangement.spacedBy(8.dp)) {
        doc.nodes.forEach { block -> EditableBlock(block, onChanged) }
    }
}

@Composable
private fun EditableBlock(block: EditorBlock, onChanged: () -> Unit) {
    when (block) {
        is TextBlock -> TextBlockField(block, onChanged)

        is AtomBlock -> when (block.type) {
            "horizontalRule" -> HorizontalDivider(
                modifier = Modifier.padding(vertical = 6.dp),
                color = MaterialTheme.colorScheme.outlineVariant,
            )
            "image" -> Text("[image]", color = MaterialTheme.colorScheme.onSurfaceVariant)
            else -> Unit
        }

        is ContainerBlock -> when (block.type) {
            "bulletList" -> ListBlock(block, ordered = false, onChanged)
            "orderedList" -> ListBlock(block, ordered = true, onChanged)
            "taskList" -> TaskListBlock(block, onChanged)
            "blockquote" -> QuoteBlock(block, onChanged)
            "table" -> TableBlock(block, onChanged)
            else -> Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                block.children.forEach { child -> EditableBlock(child, onChanged) }
            }
        }
    }
}

@Composable
private fun TextBlockField(block: TextBlock, onChanged: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    val typography = MaterialTheme.typography
    val level = (block.attrs["level"] as? Number)?.toInt() ?: 1

    val style = when (block.type) {
        "heading" -> when (level) {
            1 -> typography.headlineSmall
            2 -> typography.titleLarge
            else -> typography.titleMedium
        }.copy(fontWeight = FontWeight.SemiBold)

        "codeBlock" -> typography.bodyMedium.copy(fontFamily = EnclaveMono)
        else -> typography.bodyLarge
    }

    // Reading the styles here subscribes the field to mark changes as well as
    // text changes; the transformation itself is cheap to rebuild.
    val styles = block.content.styles.toList()
    val transformation = MarkTransformation(
        styles = styles,
        codeForeground = scheme.primary,
        codeBackground = scheme.surfaceVariant,
        link = scheme.secondary,
    )

    // An empty block has no glyphs, so on a phone it collapsed to nothing: the
    // caret could not be put into the body and the only tappable text was the
    // title. A placeholder makes the block visible and a minimum height makes
    // it a finger-sized target, while real text keeps its natural line height.
    val placeholder = when (block.type) {
        "heading" -> null
        "codeBlock" -> "Code"
        else -> "Start writing…"
    }
    val field: @Composable () -> Unit = {
        BasicTextField(
            value = block.content.text,
            onValueChange = { block.content.replace(it); onChanged() },
            textStyle = style.copy(color = scheme.onBackground),
            visualTransformation = transformation,
            cursorBrush = SolidColor(scheme.primary),
            modifier = Modifier
                .fillMaxWidth()
                .then(
                    if (block.content.text.isEmpty()) Modifier.heightIn(min = 44.dp) else Modifier,
                ),
            decorationBox = { inner ->
                Box(Modifier.fillMaxWidth()) {
                    if (block.content.text.isEmpty() && placeholder != null) {
                        Text(
                            placeholder,
                            style = style.copy(
                                color = scheme.onSurfaceVariant.copy(alpha = 0.45f),
                            ),
                        )
                    }
                    inner()
                }
            },
        )
    }

    if (block.type == "codeBlock") {
        Surface(
            color = scheme.surfaceVariant,
            shape = RoundedCornerShape(10.dp),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                val language = block.attrs["language"] as? String
                if (!language.isNullOrBlank()) {
                    Text(
                        language,
                        style = typography.labelSmall,
                        color = scheme.onSurfaceVariant,
                    )
                }
                field()
            }
        }
    } else {
        field()
    }
}

private class MarkTransformation(
    private val styles: List<CharStyle>,
    private val codeForeground: Color,
    private val codeBackground: Color,
    private val link: Color,
) : VisualTransformation {
    override fun filter(text: AnnotatedString): TransformedText {
        val source = text.text
        val builder = AnnotatedString.Builder(source)

        var i = 0
        while (i < source.length) {
            val style = styles.getOrElse(i) { CharStyle.NONE }
            var j = i + 1
            while (j < source.length && styles.getOrElse(j) { CharStyle.NONE } == style) j++

            if (style.bold) builder.addStyle(SpanStyle(fontWeight = FontWeight.Bold), i, j)
            if (style.italic) builder.addStyle(SpanStyle(fontStyle = FontStyle.Italic), i, j)
            if (style.strike) builder.addStyle(SpanStyle(textDecoration = TextDecoration.LineThrough), i, j)
            if (style.code) {
                builder.addStyle(
                    SpanStyle(fontFamily = EnclaveMono, color = codeForeground, background = codeBackground),
                    i,
                    j,
                )
            }
            if (style.link != null) {
                builder.addStyle(SpanStyle(color = link, textDecoration = TextDecoration.Underline), i, j)
            }
            i = j
        }

        return TransformedText(builder.toAnnotatedString(), OffsetMapping.Identity)
    }
}

/** Width of the marker column, shared by bullets, numbers and checkboxes. */
private val MarkerWidth = 26.dp

/** Height of one body line, so a marker sits on the first line of its item. */
private val MarkerLine = 24.dp

/**
 * The bullet, number or checkbox that starts a list item.
 *
 * It is drawn in a fixed-size box pinned to the top of the row: an empty item
 * is taller than one line (44dp, so it can be tapped), and centring the marker
 * on the row would float it into the middle of that empty space. Because all
 * three list kinds share [MarkerWidth], their text always starts at the same x.
 */
@Composable
private fun ListMarker(content: @Composable () -> Unit) {
    Box(
        modifier = Modifier.width(MarkerWidth).height(MarkerLine),
        contentAlignment = Alignment.CenterStart,
    ) { content() }
}

@Composable
private fun ListBlock(block: ContainerBlock, ordered: Boolean, onChanged: () -> Unit) {
    var index = (block.attrs["start"] as? Number)?.toInt() ?: 1
    Column(
        modifier = Modifier.padding(start = 4.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        block.children.forEach { child ->
            Row(verticalAlignment = Alignment.Top) {
                ListMarker {
                    Text(
                        if (ordered) "$index." else "•",
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Column(
                    modifier = Modifier.weight(1f),
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) { EditableBlock(child, onChanged) }
            }
            index++
        }
    }
}

@Composable
private fun TaskListBlock(block: ContainerBlock, onChanged: () -> Unit) {
    Column(
        modifier = Modifier.padding(start = 4.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        block.children.forEach { child ->
            var checked by remember(child) { mutableStateOf(child.attrs["checked"] == true) }
            Row(verticalAlignment = Alignment.Top) {
                ListMarker {
                    Checkbox(
                        checked = checked,
                        onCheckedChange = {
                            checked = it
                            child.attrs["checked"] = it
                            onChanged()
                        },
                        modifier = Modifier.size(20.dp),
                    )
                }
                Column(
                    modifier = Modifier.weight(1f),
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) { EditableBlock(child, onChanged) }
            }
        }
    }
}

@Composable
private fun QuoteBlock(block: ContainerBlock, onChanged: () -> Unit) {
    Row(Modifier.height(IntrinsicSize.Min)) {
        Box(
            Modifier
                .width(3.dp)
                .fillMaxHeight()
                .background(MaterialTheme.colorScheme.primary.copy(alpha = 0.55f), RoundedCornerShape(2.dp)),
        )
        Spacer(Modifier.width(12.dp))
        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) { block.children.forEach { child -> EditableBlock(child, onChanged) } }
    }
}

@Composable
private fun TableBlock(block: ContainerBlock, onChanged: () -> Unit) {
    Column(
        Modifier
            .fillMaxWidth()
            .border(1.dp, MaterialTheme.colorScheme.outlineVariant, RoundedCornerShape(10.dp)),
    ) {
        block.children.forEachIndexed { rowIndex, row ->
            if (row !is ContainerBlock) return@forEachIndexed
            Row(Modifier.fillMaxWidth()) {
                row.children.forEach { cell ->
                    if (cell !is ContainerBlock) return@forEach
                    val header = cell.type == "tableHeader"
                    Box(
                        Modifier
                            .weight(1f)
                            .background(
                                if (header) MaterialTheme.colorScheme.surfaceVariant else Color.Transparent,
                            )
                            .padding(horizontal = 10.dp, vertical = 8.dp),
                    ) {
                        Column(
                            verticalArrangement = Arrangement.spacedBy(2.dp),
                        ) { cell.children.forEach { EditableBlock(it, onChanged) } }
                    }
                }
            }
            if (rowIndex < block.children.lastIndex) {
                HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            }
        }
    }
}
