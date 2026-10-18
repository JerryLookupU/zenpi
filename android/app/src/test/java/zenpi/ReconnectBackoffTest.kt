package zenpi

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import zenpi.lifecycle.ReconnectBackoff

class ReconnectBackoffTest {
    @Test fun flooredJitterCeilingsGrowAndRemainCappedWithoutOverflow() {
        val bounds = mutableListOf<Long>()
        val backoff = ReconnectBackoff { bounds.add(it); it - 1 }
        repeat(12) { backoff.failed(); backoff.delayMillis(false) }
        assertEquals(listOf(751L, 1_751L, 3_751L, 7_751L, 15_751L, 31_751L) + List(6) { 59_751L }, bounds)
        repeat(10_000) { backoff.failed() }
        assertEquals(60_000L, backoff.delayMillis(true))
    }
    @Test fun onlySuccessfulPublicationResetsRetryAndRestoresFastEventCatchup() {
        val backoff = ReconnectBackoff { it - 1 }
        assertEquals(2_000L, backoff.delayMillis(false)); assertEquals(200L, backoff.delayMillis(true))
        repeat(5) { backoff.failed() }
        assertEquals(16_000L, backoff.delayMillis(true))
        backoff.succeeded()
        assertEquals(200L, backoff.delayMillis(true)); assertEquals(2_000L, backoff.delayMillis(false))
        backoff.failed(); assertEquals(1_000L, backoff.delayMillis(false))
    }
    @Test fun completeJitterRangeIncludesFloorMidpointAndUpperBound() {
        for (choice in listOf<(Long) -> Long>({ 0 }, { it / 2 }, { it - 1 })) {
            val backoff = ReconnectBackoff(choice)
            for (ceiling in listOf(1_000L, 2_000L, 4_000L, 8_000L, 16_000L, 32_000L, 60_000L)) {
                backoff.failed()
                assertEquals(250L + choice(ceiling - 250 + 1), backoff.delayMillis(false))
            }
        }
    }
    @Test fun distinctDeterministicSamplesDesynchronizeClientsWithoutChangingCaps() {
        val first = ReconnectBackoff { it / 4 }
        val second = ReconnectBackoff { it * 3 / 4 }
        repeat(20) {
            first.failed(); second.failed()
            val a = first.delayMillis(false); val b = second.delayMillis(false)
            assertNotEquals(a, b); assertTrue(a in 250..60_000); assertTrue(b in 250..60_000)
        }
    }
    @Test fun badSamplerCannotInventNegativeOrUnboundedDelay() {
        for (sample in listOf<(Long) -> Long>({ -1 }, { it }, { Long.MAX_VALUE })) {
            val backoff = ReconnectBackoff(sample); backoff.failed()
            assertTrue(runCatching { backoff.delayMillis(false) }.exceptionOrNull() is IllegalArgumentException)
        }
    }
    @Test fun lifecycleCancellationStopsBackoffWithoutStartingAnotherRead() = runBlocking {
        val entered = CompletableDeferred<Unit>()
        val backoff = ReconnectBackoff { it - 1 }
        repeat(7) { backoff.failed() }
        var reads = 1
        val job = launch {
            entered.complete(Unit)
            delay(backoff.delayMillis(false))
            reads++
        }
        entered.await(); job.cancelAndJoin()
        assertEquals(1, reads)
    }
    @Test fun failedCachePublicationDoesNotResetButConfirmedSubscriptionPageDoes() = runBlocking {
        val bounds = mutableListOf<Long>()
        val backoff = ReconnectBackoff { bounds.add(it); it - 1 }
        repeat(4) { backoff.failed() }
        var visible = "durable display"
        val failed = runCatching {
            zenpi.pairing.ownerCallback(Dispatchers.Unconfined, { true }) {
                val save: suspend () -> Unit = { throw java.io.IOException("fixture cache failure") }
                save(); backoff.succeeded(); visible = "uncommitted display"
            }
        }
        assertTrue(failed.isFailure); assertEquals("durable display", visible)
        backoff.failed(); assertEquals(16_000L, backoff.delayMillis(false))
        zenpi.pairing.ownerCallback(Dispatchers.Unconfined, { true }) {
            backoff.succeeded(); visible = "committed page"
        }
        assertEquals("committed page", visible)
        backoff.failed(); assertEquals(1_000L, backoff.delayMillis(false))
        assertEquals(listOf(15_751L, 751L), bounds)
    }
    @Test fun healthyRenewalAndReplayNeverConsumeRandomFailureSamples() {
        val backoff = ReconnectBackoff { error("Healthy renewal must not sample failure jitter") }
        repeat(100) {
            assertEquals(2_000L, backoff.delayMillis(false)); assertEquals(200L, backoff.delayMillis(true))
            backoff.succeeded()
        }
    }
    @Test fun repeatedImmediateFailuresCannotFormAZeroDelayLoopEvenAfterPublicationResets() {
        val backoff = ReconnectBackoff { 0 }
        var waited = 0L
        repeat(10_000) {
            backoff.failed()
            val delay = backoff.delayMillis(it % 2 == 0)
            assertEquals(ReconnectBackoff.MIN_FAILURE_DELAY_MS, delay)
            waited += delay
            if (it % 3 == 0) backoff.succeeded()
        }
        assertEquals(2_500_000L, waited)
        backoff.succeeded()
        assertEquals(200L, backoff.delayMillis(true))
    }
    @Test fun cancellationInterruptsMinimumFailureWaitBeforeAnotherRead() = runBlocking {
        val entered = CompletableDeferred<Unit>()
        val backoff = ReconnectBackoff { 0 }; backoff.failed()
        var reads = 1
        val job = launch {
            entered.complete(Unit)
            delay(backoff.delayMillis(true))
            reads++
        }
        entered.await(); job.cancelAndJoin()
        assertEquals(1, reads)
    }
}
