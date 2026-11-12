package com.enclave.app.ui

import org.junit.Assert.assertEquals
import org.junit.Test
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone

/**
 * Timestamps on the note cards. The core writes `chrono::Utc::now()` RFC 3339,
 * which carries nanoseconds and a `+00:00` offset — a stricter parser silently
 * fell through to printing the raw date for every note, which is how this was
 * found on the device.
 */
class TimeAgoTest {

    /** A fixed "now": 2026-11-03T12:00:00Z. */
    private val now = utcMillis("2026-11-03T12:00:00Z")

    private fun utcMillis(iso: String): Long =
        SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'", Locale.US)
            .apply { timeZone = TimeZone.getTimeZone("UTC") }
            .parse(iso)!!
            .time

    private fun ago(seconds: Long): String =
        SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'", Locale.US)
            .apply { timeZone = TimeZone.getTimeZone("UTC") }
            .format(Date(now - seconds * 1000))

    @Test
    fun `the chrono format the core writes is understood`() {
        assertEquals("5m ago", timeAgo("2026-11-03T11:55:00.123456789+00:00", now))
        assertEquals("2h ago", timeAgo("2026-11-03T10:00:00.5+00:00", now))
        assertEquals("just now", timeAgo("2026-11-03T12:00:00+00:00", now))
    }

    @Test
    fun `seconds, minutes, hours and days read as ages`() {
        assertEquals("just now", timeAgo(ago(0), now))
        assertEquals("just now", timeAgo(ago(59), now))
        assertEquals("1m ago", timeAgo(ago(60), now))
        assertEquals("59m ago", timeAgo(ago(59 * 60 + 30), now))
        assertEquals("3h ago", timeAgo(ago(3 * 3600), now))
        assertEquals("6d ago", timeAgo(ago(6 * 86_400), now))
    }

    @Test
    fun `a timestamp in the future reads as just now, not as an age`() {
        // The device clock moving backwards (DST, manual change) must not turn
        // every note into a negative age or a raw date.
        assertEquals("just now", timeAgo("2026-11-03T13:00:00+00:00", now))
    }

    @Test
    fun `older than a week becomes a short human date`() {
        assertEquals("27 Oct 2026", timeAgo(ago(7 * 86_400), now))
        assertEquals("4 Oct 2026", timeAgo(ago(30 * 86_400), now))
    }

    @Test
    fun `a date-only value is read as midnight UTC`() {
        // Midnight UTC on the same day is 12 hours before the fixed "now".
        assertEquals("12h ago", timeAgo("2026-11-03", now))
    }

    @Test
    fun `an unparsable value degrades to its date part`() {
        assertEquals("not-a-date", timeAgo("not-a-date", now))
        assertEquals("", timeAgo("", now))
    }
}
