package com.enclave.app.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The PIN policy has to match the shipping app (`/^\d{4,12}$/`): the gate used
 * to accept exactly four digits, which silently rejected every longer PIN a
 * user could have created in the web app. These cases lock the boundaries.
 */
class PinRulesTest {

    @Test
    fun `four digits is the shortest valid pin`() {
        assertFalse(PinRules.isValid(""))
        assertFalse(PinRules.isValid("123"))
        assertTrue(PinRules.isValid("1234"))
    }

    @Test
    fun `twelve digits is the longest valid pin`() {
        assertTrue(PinRules.isValid("123456789012"))
        assertFalse(PinRules.isValid("1234567890123"))
    }

    @Test
    fun `only ascii digits are valid`() {
        assertFalse(PinRules.isValid("123a"))
        assertFalse(PinRules.isValid("12 34"))
        // Arabic-Indic digits pass Char.isDigit() but not the web app's \d.
        assertFalse(PinRules.isValid("٢٩١٤"))
        assertFalse(PinRules.isValid("-123"))
    }

    @Test
    fun `append ignores non-digits and never exceeds the cap`() {
        assertEquals("1", PinRules.append("", '1'))
        assertEquals("1", PinRules.append("1", 'a'))
        assertEquals("1", PinRules.append("1", ' '))
        assertEquals("", PinRules.append("", '-'))

        val full = (1..PinRules.MAX_LENGTH).fold("") { pin, digit ->
            PinRules.append(pin, digit.toString()[0])
        }
        assertEquals(PinRules.MAX_LENGTH, full.length)
        assertTrue(PinRules.isValid(full))
        // One past the cap is dropped rather than truncated to something else.
        assertEquals(full, PinRules.append(full, '9'))
    }

    @Test
    fun `filter keeps digits from a pasted or hardware-keyboard entry`() {
        assertEquals("1234", PinRules.filter("1a2b3c4"))
        assertEquals("1234", PinRules.filter("12 34"))
        assertEquals("123456789012", PinRules.filter("123456789012345"))
        assertEquals("", PinRules.filter("٢٩١٤"))
    }

    @Test
    fun `the dot row grows with the entry and never shrinks below the minimum`() {
        assertEquals(PinRules.MIN_LENGTH, PinRules.slots(0))
        assertEquals(PinRules.MIN_LENGTH, PinRules.slots(3))
        assertEquals(PinRules.MIN_LENGTH, PinRules.slots(4))
        assertEquals(7, PinRules.slots(7))
        assertEquals(PinRules.MAX_LENGTH, PinRules.slots(PinRules.MAX_LENGTH))
    }

    @Test
    fun `a longer pin can be typed digit by digit and stays valid`() {
        val pin = "4815162342"
        var typed = ""
        pin.forEach { typed = PinRules.append(typed, it) }
        assertEquals(pin, typed)
        assertTrue(PinRules.isValid(typed))
    }
}
