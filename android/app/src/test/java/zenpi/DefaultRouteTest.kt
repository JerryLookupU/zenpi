package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.lifecycle.DefaultRouteState
import zenpi.lifecycle.NetworkInventoryState
import zenpi.lifecycle.NetworkObservation

class DefaultRouteTest {
    @Test fun repeatedInventoryReconciliationDoesNotReconnectAndMissedLossIsOffline() {
        val routes = NetworkInventoryState<String>()
        routes.update(7L, mapOf(9L to "LAN")); val initial = routes.current
        repeat(100) { assertFalse(routes.update(7L, mapOf(9L to "LAN"))); assertEquals(initial, routes.current) }
        // A fresh inventory is authoritative even when no loss callback arrived.
        assertTrue(routes.update(null, emptyMap())); assertNull(routes.current)
        repeat(100) { assertFalse(routes.update(null, emptyMap())); assertNull(routes.current) }
        assertTrue(routes.update(7L, mapOf(9L to "LAN")))
        assertNotNull(routes.current); assertNotEquals(initial, routes.current)
    }

    @Test fun failedInventoryRetiresCycleBeforeOfflineAndNeverPublishesPartialRoutes() {
        for (error in listOf(SecurityException("network permission changed"), IllegalStateException("service unavailable"))) {
            val events = mutableListOf<String>()
            var generation = 1
            val admitted = generation
            val routes = NetworkInventoryState<String>()
            routes.update(7L, mapOf(9L to "LAN"))
            val observed = NetworkObservation.read(onFailure = {
                generation++; events += "retired"; events += "offline"
            }) {
                events += "partial-local-inventory"
                throw error
            }
            assertNull(observed)
            assertEquals(listOf("partial-local-inventory", "retired", "offline"), events)
            // Queued callbacks use the captured cycle and cannot apply the partial result.
            assertNotEquals(admitted, generation)
        }
    }
    @Test fun completeInventoryIsPublishedOnlyAfterAllReadsAndFailureConsumerErrorsAreNotSwallowed() {
        var failed = false
        val complete = 7L to mapOf(9L to "LAN")
        assertEquals(complete, NetworkObservation.read(onFailure = { failed = true }) { complete })
        assertFalse(failed)
        val consumer = IllegalArgumentException("consumer failure")
        assertSame(consumer, runCatching {
            NetworkObservation.read(onFailure = { throw consumer }) { throw SecurityException() }
        }.exceptionOrNull())
        val fatal = AssertionError("not a recoverable network service failure")
        assertSame(fatal, runCatching { NetworkObservation.read(onFailure = { failed = true }) { throw fatal } }.exceptionOrNull())
        assertFalse(failed)
    }
    @Test fun newMonitorCycleCannotReuseAViewModelObservedRouteToken() {
        val first = NetworkInventoryState<String>()
        first.update(7L, mapOf(9L to "LAN"))
        val observed = first.current
        val restarted = NetworkInventoryState<String>()
        restarted.update(7L, mapOf(9L to "LAN"), force = true)
        assertNotNull(restarted.current); assertNotEquals(observed, restarted.current)
        val token = restarted.current
        assertFalse(restarted.update(7L, mapOf(9L to "LAN")))
        assertEquals(token, restarted.current)
        assertTrue(restarted.update(7L, mapOf(9L to "LAN"), force = true))
        assertNotEquals(token, restarted.current)
    }
    @Test fun stoppedNetworkChangeAndOfflineRestartCannotBeHiddenByTokenCollision() {
        val first = NetworkInventoryState<String>(); first.update(7L, mapOf(9L to "LAN"))
        val next = NetworkInventoryState<String>(); next.update(8L, mapOf(9L to "LAN"))
        assertNotEquals(first.current, next.current)
        assertTrue(next.update(null, emptyMap(), force = true)); assertNull(next.current)
        assertTrue(next.update(null, emptyMap(), force = true)); assertNull(next.current)
        next.update(8L, emptyMap()); assertNotEquals(first.current, next.current)
    }
    @Test fun parallelInventoryInstancesAllocateDistinctProcessTokens() {
        val tokens = java.util.Collections.synchronizedSet(mutableSetOf<Long>())
        val failures = java.util.concurrent.ConcurrentLinkedQueue<Throwable>()
        val threads = (0 until 8).map { index -> Thread {
            try { repeat(100) {
                    val routes = NetworkInventoryState<Int>()
                    routes.update(index.toLong() + 1, mapOf(9L to index))
                    assertTrue(tokens.add(requireNotNull(routes.current)))
                }
            } catch (error: Throwable) { failures.add(error) }
        }.apply { start() } }
        threads.forEach { it.join(5_000); assertFalse(it.isAlive) }
        assertTrue(failures.toString(), failures.isEmpty())
        assertEquals(800, tokens.size)
    }
    @Test fun offlineLanIsAvailableWithoutDefaultAndLinkChangesFenceExistingSockets() {
        val routes = NetworkInventoryState<String>()
        assertTrue(routes.update(null, emptyMap())); assertNull(routes.current)
        assertTrue(routes.update(null, mapOf(7L to "192.168.1.0/24"))); val first = routes.current
        assertNotNull(first); assertFalse(routes.update(null, mapOf(7L to "192.168.1.0/24")))
        assertTrue(routes.update(null, mapOf(7L to "192.168.2.0/24"))); assertNotEquals(first, routes.current)
        assertTrue(routes.update(null, emptyMap())); assertNull(routes.current)
    }
    @Test fun localNetworkLossIsOfflineEvenWhileCellularDefaultRemains() {
        val routes = NetworkInventoryState<String>()
        routes.update(1L, mapOf(7L to "LAN")); val first = routes.current
        assertTrue(routes.update(1L, emptyMap())); assertNull(routes.current); assertNotEquals(first, routes.current)
        val second = routes.current
        assertTrue(routes.update(2L, emptyMap())); assertNull(second); assertNull(routes.current)
        assertFalse(routes.update(2L, emptyMap()))
    }
    @Test fun oldRouteLossCannotOverrideNewDefaultAndDuplicateEventsDoNotReconnect() {
        val routes = DefaultRouteState()
        assertTrue(routes.available(1)); assertFalse(routes.available(1))
        assertTrue(routes.available(2)); assertFalse(routes.lost(1)); assertEquals(2L, routes.current)
        assertTrue(routes.lost(2)); assertNull(routes.current); assertFalse(routes.lost(2))
        assertTrue(routes.available(3)); assertEquals(3L, routes.current)
    }
    @Test fun initiallyAbsentRouteAndRecoveryRemainDistinct() {
        val routes = DefaultRouteState()
        assertFalse(routes.available(null)); assertFalse(routes.lost(1))
        assertTrue(routes.available(7)); assertTrue(routes.lost(7)); assertNull(routes.current)
    }
}
