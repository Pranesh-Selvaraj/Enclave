package com.enclave.app.ui

/**
 * The PIN policy of the vault gate.
 *
 * It mirrors the shipping web app, which accepts `/^\d{4,12}$/` — ASCII digits
 * only, four at the least. Four is the phone-sized minimum, and the extra eight
 * digits are the only control a user has over the offline cost of guessing the
 * vault key blob. The gate therefore never caps the entry at four: the keypad
 * keeps accepting digits until [MAX_LENGTH] and the user confirms explicitly.
 *
 * The core does not validate the length — the password only encrypts the vault
 * key blob (Argon2id over a random salt) — so this policy is entirely the app's.
 *
 * `Char.isDigit()` is deliberately not used: it is true for non-ASCII digits
 * (Arabic-Indic and friends), which the web app rejects and a hardware keyboard
 * can produce, and a PIN typed on the keypad is always ASCII.
 */
internal object PinRules {
    /** Shortest accepted PIN. */
    const val MIN_LENGTH = 4

    /** Longest accepted PIN; the keypad stops accepting digits here. */
    const val MAX_LENGTH = 12

    private fun Char.isPinDigit(): Boolean = this in '0'..'9'

    /** Append a typed character, ignoring anything but an ASCII digit and the cap. */
    fun append(current: String, digit: Char): String = when {
        !digit.isPinDigit() -> current
        current.length >= MAX_LENGTH -> current
        else -> current + digit
    }

    /** Keep only ASCII digits, capped at [MAX_LENGTH] — for text-field input. */
    fun filter(text: String): String = text.filter { it.isPinDigit() }.take(MAX_LENGTH)

    /** True when [pin] is a PIN the gate is allowed to submit. */
    fun isValid(pin: String): Boolean =
        pin.length in MIN_LENGTH..MAX_LENGTH && pin.all { it.isPinDigit() }

    /**
     * Dot slots to draw for [length] entered digits: at least [MIN_LENGTH] so a
     * fresh form still shows a target, growing as the user types past it.
     */
    fun slots(length: Int): Int = length.coerceIn(MIN_LENGTH, MAX_LENGTH)
}
