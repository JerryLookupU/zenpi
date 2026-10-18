package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.lifecycle.ReconnectBackoff
import zenpi.transport.SubscriptionSession

class SubscriptionSessionTest {
    /**
     * Idle foreground, as HostClient.inspect and PairingViewModel.refresh drive it: one
     * handshake, pages of the full host duration while the session continues, then the
     * caller's idle delay before it reconnects. Returns the handshake times (ms).
     */
    private fun idleHandshakes(minutes: Int): Pair<List<Long>, Int> {
        var now = 0L
        var pages = 0
        val handshakes = mutableListOf<Long>()
        val backoff = ReconnectBackoff { it - 1 }
        while (now < minutes * 60_000L) {
            handshakes += now
            val session = SubscriptionSession { now * 1_000_000 }
            do { now += SubscriptionSession.PAGE_MS; pages++ } while (session.continues())
            now += backoff.delayMillis(false)
        }
        return handshakes to pages
    }

    @Test fun idleSubscriptionHandshakesAtMostOncePerMinuteAndReusesTheSession() {
        val (handshakes, pages) = idleHandshakes(10)
        assertTrue(handshakes.zipWithNext().all { (before, after) -> after - before >= 60_000 })
        assertTrue(handshakes.size <= 10)
        // Before ZE-542 every 3 s page needed its own handshake (12 per minute).
        assertTrue(pages >= handshakes.size * 20)
    }

    @Test fun operationDeadlineCoversTheWholeReuseWindowAndAFinalPage() {
        assertTrue(SubscriptionSession.OPERATION_MS >= 15_000 + SubscriptionSession.REUSE_MS + SubscriptionSession.PAGE_MS)
        var now = 0L
        val session = SubscriptionSession { now }
        assertTrue(session.continues())
        now = SubscriptionSession.REUSE_MS * 1_000_000 - 1
        assertTrue(session.continues())
        now += 1
        assertFalse(session.continues())
    }
}
