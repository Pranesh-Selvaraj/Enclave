package com.enclave.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * JVM tests for the note JSON helpers that feed the widget cache (issue #124).
 * These run without a device: the functions are pure Kotlin over org.json.
 */
class NoteJsonTest {

    @Test
    fun taskListDocJsonRoundTripsThroughDocChecklist() {
        val tasks = listOf(TaskItem("buy milk", false), TaskItem("call mum", true))
        assertEquals(tasks, docChecklist(taskListDocJson(tasks)))
    }

    @Test
    fun docChecklistReadsCheckedAttrAndNestedText() {
        val doc = """
            {"type":"doc","content":[{"type":"taskList","content":[
              {"type":"taskItem","attrs":{"checked":true},"content":[
                {"type":"paragraph","content":[{"type":"text","text":"done thing"}]}]},
              {"type":"taskItem","content":[
                {"type":"paragraph","content":[{"type":"text","text":"open thing"}]}]}
            ]}]}
        """.trimIndent()
        assertEquals(
            listOf(TaskItem("done thing", true), TaskItem("open thing", false)),
            docChecklist(doc),
        )
    }

    @Test
    fun docChecklistIsEmptyWithoutATaskList() {
        assertEquals(emptyList<TaskItem>(), docChecklist("""{"type":"doc","content":[]}"""))
        assertEquals(emptyList<TaskItem>(), docChecklist(null))
        assertEquals(emptyList<TaskItem>(), docChecklist(""))
        assertEquals(emptyList<TaskItem>(), docChecklist("not json at all"))
    }

    @Test
    fun docBodyTextJoinsParagraphsAndSkipsTaskList() {
        val doc = """
            {"type":"doc","content":[
              {"type":"paragraph","content":[{"type":"text","text":"First para"}]},
              {"type":"paragraph","content":[{"type":"text","text":"line one"},{"type":"hardBreak"},{"type":"text","text":"line two"}]},
              {"type":"taskList","content":[{"type":"taskItem","content":[
                {"type":"paragraph","content":[{"type":"text","text":"do not show"}]}]}]},
              {"type":"paragraph","content":[{"type":"text","text":"Last para"}]}
            ]}
        """.trimIndent()
        assertEquals("First para\n\nline one\nline two\n\nLast para", docBodyText(doc))
    }

    @Test
    fun docBodyTextToleratesNullAndMalformedInput() {
        assertEquals("", docBodyText(null))
        assertEquals("", docBodyText(""))
        assertEquals("", docBodyText("{"))
    }

    @Test
    fun docSnippetFallsBackToChecklistItems() {
        val tasks = listOf(TaskItem("one", false), TaskItem("two", true))
        assertEquals("one two", docSnippet(taskListDocJson(tasks)))
    }

    @Test
    fun docJsonRoundTripsThroughDocBodyText() {
        assertEquals("Hello\nworld\n\nSecond", docBodyText(docJson("Hello\nworld\n\nSecond")))
    }

    @Test
    fun tagsOfParsesAndDropsBlanks() {
        assertEquals(listOf("work", "home"), tagsOf("""{"tags":["work","","home"]}"""))
        assertEquals(emptyList<String>(), tagsOf(null))
        assertEquals(emptyList<String>(), tagsOf("""{"tags":[]}"""))
        assertEquals(emptyList<String>(), tagsOf("garbage"))
    }

    @Test
    fun widgetBlockEnabledRequiresExplicitTrue() {
        assertTrue(widgetBlockEnabled("""{"enabled":true}"""))
        assertFalse(widgetBlockEnabled("""{"enabled":false}"""))
        assertFalse(widgetBlockEnabled("""{"other":true}"""))
        assertFalse(widgetBlockEnabled(null))
        assertFalse(widgetBlockEnabled(""))
        assertFalse(widgetBlockEnabled("garbage"))
    }
}
