package com.enclave.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import uniffi.core_api.ChecklistRow

/**
 * JVM tests for the widget cache codec and the locked/hidden decision
 * (issue #124). The ProseMirror projection itself moved to Rust in #122 and
 * is covered by the core-api unit tests; here we only pin the Android-side
 * Keystore cache format and the hide rule.
 */
class WidgetStoreTest {

    private fun note(
        id: String = "doc-1",
        title: String = "Shopping",
        tags: List<String> = listOf("home", "errands"),
        updatedAt: String = "2026-10-01T10:00:00Z",
        checklist: List<ChecklistRow> = listOf(ChecklistRow("milk", false), ChecklistRow("bread", true)),
        text: String = "body text",
    ) = WidgetStore.WidgetNote(id, title, tags, updatedAt, checklist, text)

    @Test
    fun cacheSerializationRoundTripsEverythingWidgetsRender() {
        val notes = listOf(note(), note(id = "doc-2", title = "Untitled", tags = emptyList(), checklist = emptyList(), text = ""))
        assertEquals(notes, WidgetStore.parse(WidgetStore.serialize(notes)))
    }

    @Test
    fun emptyCacheRoundTrips() {
        assertEquals(emptyList<WidgetStore.WidgetNote>(), WidgetStore.parse(WidgetStore.serialize(emptyList())))
    }

    @Test
    fun parseIgnoresNotesWithoutTheExpectedShape() {
        // A cache from an older/newer build must degrade to nothing, never crash.
        assertEquals(emptyList<WidgetStore.WidgetNote>(), WidgetStore.parse("""{"notes":[]}"""))
    }

    @Test
    fun widgetsHideOnlyWhenTheDialIsOnAndTheVaultIsLocked() {
        assertTrue(WidgetStore.widgetsHidden(hideWhenLocked = true, vaultUnlocked = false))
        assertFalse(WidgetStore.widgetsHidden(hideWhenLocked = true, vaultUnlocked = true))
        assertFalse(WidgetStore.widgetsHidden(hideWhenLocked = false, vaultUnlocked = false))
        assertFalse(WidgetStore.widgetsHidden(hideWhenLocked = false, vaultUnlocked = true))
    }
}
