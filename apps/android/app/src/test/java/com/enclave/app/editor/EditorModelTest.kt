package com.enclave.app.editor

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The editor must read and write exactly the document the core stores: the
 * canonical fixture (`core/core-db/tests/fixtures/block-schema-v1.json`) is the
 * same file the Rust contract test preserves verbatim.
 */
class EditorModelTest {

    private fun fixture(): String =
        javaClass.classLoader!!.getResourceAsStream("block-schema-v1.json")!!
            .bufferedReader()
            .use { it.readText() }

    @Test
    fun `round-trips the golden document without changing it`() {
        val raw = fixture()
        val original = JSONObject(raw)
        val roundTripped = JSONObject(EditorDocument.parse(raw).toJson())

        assertTrue(
            "the editor must reproduce the golden document",
            original.similar(roundTripped),
        )
    }

    @Test
    fun `keeps existing marks when text is appended`() {
        val doc = EditorDocument.parse(fixture())
        val paragraph = doc.nodes.first { it is TextBlock } as TextBlock
        paragraph.content.replace(paragraph.content.text + " edited")

        val nodes = JSONObject(doc.toJson()).getJSONArray("content").getJSONObject(1)
            .getJSONArray("content")
        var boldStillThere = false
        for (i in 0 until nodes.length()) {
            val node = nodes.getJSONObject(i)
            if (node.optString("text") == "bold" && node.optJSONArray("marks")?.toString()?.contains("bold") == true) {
                boldStillThere = true
            }
        }
        assertTrue("the bold run must survive an unrelated edit", boldStillThere)
    }

    @Test
    fun `styles stay aligned with the text while typing`() {
        val doc = EditorDocument.parse(fixture())
        val paragraph = doc.nodes.first { it is TextBlock } as TextBlock

        paragraph.content.replace(paragraph.content.text + "abc")
        assertEquals(paragraph.content.text.length, paragraph.content.styles.size)

        paragraph.content.replace(paragraph.content.text.dropLast(2))
        assertEquals(paragraph.content.text.length, paragraph.content.styles.size)

        paragraph.content.replace("")
        assertEquals(0, paragraph.content.text.length)
        assertEquals(0, paragraph.content.styles.size)
    }

    @Test
    fun `serialises an empty paragraph and a new document`() {
        val empty = EditorDocument.empty()
        val json = JSONObject(empty.toJson())
        assertEquals("doc", json.getString("type"))
        assertEquals(1, json.getJSONArray("content").length())
        assertEquals("paragraph", json.getJSONArray("content").getJSONObject(0).getString("type"))
    }
}
