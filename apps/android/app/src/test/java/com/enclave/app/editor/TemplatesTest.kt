package com.enclave.app.editor

import org.json.JSONObject
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The create stack must only produce documents the core and the editor agree
 * on: every template is parsed through the editor model and must round-trip
 * unchanged.
 */
class TemplatesTest {

    @Test
    fun `every content template parses and round-trips`() {
        for (kind in CreateKind.entries) {
            if (kind == CreateKind.FOLDER) continue

            val raw = Templates.forKind(kind).doc.toString()
            val roundTripped = JSONObject(EditorDocument.parse(raw).toJson())

            assertTrue(
                "template ${kind.name} must round-trip through the editor model",
                JSONObject(raw).similar(roundTripped),
            )
        }
    }

    @Test
    fun `templates carry their expected titles`() {
        assertTrue(Templates.forKind(CreateKind.CHECKLIST).title == "Checklist")
        assertTrue(Templates.forKind(CreateKind.MEETING).title == "Meeting notes")
        assertTrue(Templates.forKind(CreateKind.JOURNAL).title == "Journal")
        assertTrue(Templates.forKind(CreateKind.TEXT).title.isEmpty())
    }
}
