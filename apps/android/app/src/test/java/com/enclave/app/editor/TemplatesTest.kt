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
        assertTrue(Templates.forKind(CreateKind.TEXT).title.isEmpty())
    }

    /**
     * Journal and Meeting notes were removed from the create stack when users
     * could make their own templates (#220): a fixed "meeting" layout cannot
     * know this team's agenda, and the set is now Text / Checklist / Folder
     * plus whatever the user saved.
     */
    @Test
    fun `only the creation primitives are built in`() {
        assertTrue(CreateKind.entries.map { it.name }.containsAll(listOf("TEXT", "CHECKLIST", "FOLDER")))
        assertTrue(CreateKind.entries.none { it.name == "MEETING" || it.name == "JOURNAL" })
    }
}
