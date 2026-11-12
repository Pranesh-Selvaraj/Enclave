package com.enclave.app.ui

import android.content.Context
import android.content.SharedPreferences
import kotlin.math.min
import kotlin.math.pow

/**
 * Throttles failed unlock attempts.
 *
 * The app has no length policy (ADR 0011): a one-character secret is valid, so
 * the *only* remaining barrier to guessing a short secret is the cost of an
 * attempt plus the time between attempts. Argon2id alone is a speed bump — ten
 * taps walk the whole digit space — so the number of attempts has to be bounded
 * in time and persisted, not held in a field that a restart clears.
 *
 * The schedule is deliberately generous before it bites (people mistype) and
 * then grows quickly: the first two failures cost nothing, then 1 s, 3 s, 10 s,
 * 30 s, and a minute each after that, capped at five minutes.
 */
object UnlockThrottle {
    private const val PREFS = "enclave_unlock_throttle"
    private const val KEY_FAILURES = "failures"
    private const val KEY_NEXT_AT = "next_attempt_at"

    /** Delays in milliseconds, indexed by the number of consecutive failures. */
    private val Delays = longArrayOf(0, 0, 1_000, 3_000, 10_000, 30_000, 60_000)
    private const val MAX_DELAY = 300_000L

    private fun prefs(context: Context): SharedPreferences =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    /** Consecutive failures so far; reset by a successful unlock. */
    fun failures(context: Context): Int = prefs(context).getInt(KEY_FAILURES, 0)

    /** When the next attempt is allowed, in elapsed-realtime milliseconds. */
    fun blockedUntil(context: Context): Long = prefs(context).getLong(KEY_NEXT_AT, 0L)

    /** Remaining wait in milliseconds, or 0 when an attempt is allowed now. */
    fun remaining(context: Context, now: Long = android.os.SystemClock.elapsedRealtime()): Long =
        (blockedUntil(context) - now).coerceAtLeast(0L)

    /** Record a failed attempt and extend the window. */
    fun recordFailure(context: Context, now: Long = android.os.SystemClock.elapsedRealtime()) {
        val next = failures(context) + 1
        val delay = if (next < Delays.size) {
            Delays[next]
        } else {
            min(MAX_DELAY, Delays.last() * 2.0.pow(next - Delays.size + 1).toLong())
        }
        prefs(context).edit()
            .putInt(KEY_FAILURES, next)
            .putLong(KEY_NEXT_AT, now + delay)
            .apply()
    }

    /** Clear the counter; called after a successful unlock. */
    fun reset(context: Context) {
        prefs(context).edit().clear().apply()
    }

    /** "45s" / "2m 10s" — the label beside a disabled unlock button. */
    fun describe(remainingMs: Long): String {
        val seconds = (remainingMs / 1000).coerceAtLeast(1)
        return if (seconds < 60) "${seconds}s" else "${seconds / 60}m ${seconds % 60}s"
    }
}
