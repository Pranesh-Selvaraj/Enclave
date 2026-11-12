package com.enclave.app.ui

/**
 * The rule for the app's unlock secret — which is not really a "PIN": any
 * non-empty string is accepted, with no minimum and no maximum. One digit is a
 * valid secret, and so is a long passphrase; the keypad is merely the numeric
 * input method, not the policy. Letters and symbols come from the Password tab
 * or from the text fields in the restore/recovery dialogs.
 *
 * The core does not validate either — the secret only encrypts the vault key
 * blob with Argon2id over a random salt — so this is entirely a product choice.
 *
 * A one-character secret is weak (ten guesses if it is a digit), and the UI says
 * so next to the field ("longer is safer") instead of refusing it: the user owns
 * the trade-off, and a wallet-style vault may legitimately be quick to open.
 */
internal object PinRules {
    /** Shortest accepted secret. One character. */
    const val MIN_LENGTH = 1

    /** Dots drawn at most; longer secrets show a count instead of more dots. */
    const val MAX_DOTS = 12

    /** Append a typed character. No length cap: the secret is the user's. */
    fun append(current: String, digit: Char): String = current + digit

    /** Normalise text-field input; nothing is stripped, so any character is allowed. */
    fun filter(text: String): String = text

    /** Any non-empty secret is submittable. */
    fun isValid(pin: String): Boolean = pin.isNotEmpty()

    /** Dot slots to draw: at least one target, at most [MAX_DOTS]. */
    fun slots(length: Int): Int = length.coerceIn(MIN_LENGTH, MAX_DOTS)
}
