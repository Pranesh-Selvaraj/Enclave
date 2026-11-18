package com.enclave.app.editor

import org.junit.Assert.assertEquals
import org.junit.Test

class NoteFactoryTest {
    @Test
    fun sharedTextUsesItsFirstNonEmptyLineAsTitle() {
        assertEquals(
            "Hello world",
            NoteFactory.sharedTitle("\n  \nHello world\nsecond line", ""),
        )
    }

    @Test
    fun blankSharedTextFallsBackToTheTemplateTitle() {
        assertEquals("Text", NoteFactory.sharedTitle(" \n\t\n", "Text"))
        assertEquals("Text", NoteFactory.sharedTitle(null, "Text"))
    }

    @Test
    fun longFirstLinesAreCapped() {
        assertEquals(80, NoteFactory.sharedTitle("x".repeat(120), "").length)
    }
}
