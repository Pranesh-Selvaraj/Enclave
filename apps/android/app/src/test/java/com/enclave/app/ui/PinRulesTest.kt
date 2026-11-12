package com.enclave.app.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The unlock secret has no minimum and no maximum: any single character is
 * accepted, and the keypad simply keeps accepting input. These cases pin that
 * policy down, because it deliberately gives up the old 4–12-digit rule.
 */
class PinRulesTest {

    @Test
    fun `one character is a valid secret`() {
        assertTrue(PinRules.isValid("7"))
        assertTrue(PinRules.isValid("a"))
        assertTrue(PinRules.isValid("🔐"))
    }

    @Test
    fun `only an empty secret is rejected`() {
        assertFalse(PinRules.isValid(""))
        assertTrue(PinRules.isValid("1"))
        assertTrue(PinRules.isValid("x"))
    }

    @Test
    fun `there is no upper bound on the secret`() {
        val long = "correct horse battery staple and then some more words"
        assertTrue(PinRules.isValid(long))
        var typed = ""
        repeat(64) { typed = PinRules.append(typed, '9') }
        assertEquals(64, typed.length)
        assertTrue(PinRules.isValid(typed))
    }

    @Test
    fun `any character is allowed, not just digits`() {
        assertTrue(PinRules.isValid("pässwörd!"))
        assertTrue(PinRules.isValid("٢٩١٤"))
        assertEquals("12 34", PinRules.filter("12 34"))
    }

    @Test
    fun `append keeps every character it is given`() {
        assertEquals("1", PinRules.append("", '1'))
        assertEquals("1a", PinRules.append("1", 'a'))
        assertEquals("1a!", PinRules.append("1a", '!'))
    }

    @Test
    fun `four dots are drawn for anything up to four characters`() {
        // The four-slot placeholder is a decoy: an onlooker cannot tell a
        // one-character secret from a four-character one.
        assertEquals(4, PinRules.slots(0))
        assertEquals(4, PinRules.slots(1))
        assertEquals(4, PinRules.slots(3))
        assertEquals(4, PinRules.slots(4))
    }

    @Test
    fun `a longer secret grows the row, up to the display cap`() {
        assertEquals(5, PinRules.slots(5))
        assertEquals(9, PinRules.slots(9))
        assertEquals(PinRules.MAX_DOTS, PinRules.slots(PinRules.MAX_DOTS))
        assertEquals(PinRules.MAX_DOTS, PinRules.slots(PinRules.MAX_DOTS + 9))
    }
}
