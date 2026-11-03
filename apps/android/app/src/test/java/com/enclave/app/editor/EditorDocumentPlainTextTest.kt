package com.enclave.app.editor

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * `plainText()` feeds two user-visible surfaces — the "share" intent body and
 * the note card previews — so it is tested against the same golden fixture the
 * core preserves verbatim, plus the formatting rules for each node kind.
 *
 * It deliberately drops images (no alt text in the contract) and keeps table
 * cells as their own lines, which is what a share body and a four-line preview
 * can actually use.
 */
class EditorDocumentPlainTextTest {

    private fun fixture(): String =
        javaClass.classLoader!!.getResourceAsStream("block-schema-v1.json")!!
            .bufferedReader()
            .use { it.readText() }

    private fun doc(vararg nodes: EditorBlock) = EditorDocument(nodes.toMutableList())

    private fun paragraph(text: String) =
        TextBlock("paragraph", mutableMapOf(), EditableText().also { it.replace(text) })

    private fun item(type: String, text: String, checked: Boolean? = null): ContainerBlock {
        val attrs = mutableMapOf<String, Any?>()
        if (checked != null) attrs["checked"] = checked
        return ContainerBlock(
            type,
            attrs,
            mutableListOf<EditorBlock>(paragraph(text)),
        )
    }

    @Test
    fun `the golden fixture flattens with its list and task markers`() {
        val text = EditorDocument.parse(fixture()).plainText()

        // Headings, paragraphs and code keep their text.
        assertTrue(text, text.contains("Phase 0 editor slice"))
        assertTrue(text, text.contains("Plain text with bold, italic, code and a link."))
        assertTrue(text, text.contains("    println!(\"hello\");"))

        // Both list kinds use a bullet; task items use their checked state.
        assertTrue(text, text.contains("• First bullet"))
        assertTrue(text, text.contains("• Second bullet"))
        assertTrue(text, text.contains("• First step"))
        assertTrue(text, text.contains("☐ Todo item"))
        assertTrue(text, text.contains("☑ Done item"))

        // Rules survive; the image has no alt text in the contract, so it does not.
        assertTrue(text, text.contains("---"))
        assertFalse(text, text.contains("image"))
    }

    @Test
    fun `no leading, trailing or repeated blank lines`() {
        val text = doc(
            paragraph("   "),
            paragraph("First"),
            paragraph(""),
            paragraph("Second"),
            paragraph("\n  \n"),
        ).plainText()
        assertEquals("First\nSecond", text)
    }

    @Test
    fun `a nested list inside an item keeps both lines`() {
        val nested = ContainerBlock(
            "bulletList",
            mutableMapOf(),
            mutableListOf<EditorBlock>(item("listItem", "Inner")),
        )
        val outer = ContainerBlock("listItem", mutableMapOf(), mutableListOf<EditorBlock>(paragraph("Outer"), nested))
        val text = doc(ContainerBlock("bulletList", mutableMapOf(), mutableListOf<EditorBlock>(outer))).plainText()
        assertEquals("• Outer\n• Inner", text)
    }

    @Test
    fun `a task without a checked attribute reads as unchecked`() {
        val text = doc(ContainerBlock("taskList", mutableMapOf(), mutableListOf<EditorBlock>(item("taskItem", "Maybe")))).plainText()
        assertEquals("☐ Maybe", text)
    }

    @Test
    fun `an empty document flattens to an empty string`() {
        assertEquals("", EditorDocument.empty().plainText())
    }
}
