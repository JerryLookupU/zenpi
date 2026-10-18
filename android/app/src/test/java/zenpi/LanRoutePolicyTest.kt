package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.lifecycle.*

class LanRoutePolicyTest {
    private fun address(value: String, prefix: Int) = LanAddressState(value, prefix, 0, 0)
    @Test fun unchangedAddressCannotHideFlagOrScopeTransitionBeforeConnectOrFromNetworkMonitor() {
        val source = address("fd00::10", 64)
        val link = LanLinkState("wlan0", 1500, setOf(source))
        val route = LanRoute(7, 64, link = link)
        for (changed in listOf(source.copy(flags = 1), source.copy(flags = 2), source.copy(flags = 32),
            source.copy(flags = 64), source.copy(scope = 253), source.copy(flags = 64, scope = 253))) {
            var observations = 0
            var connected = false
            val binds = mutableListOf<Long>()
            assertTrue(runCatching {
                LanRoutePolicy.bindStable(observe = {
                    listOf(if (observations++ == 0) route else route.copy(link = link.copy(addresses = setOf(changed))))
                }, bind = { binds += it })
                connected = true
            }.exceptionOrNull() is java.io.IOException)
            assertFalse(connected); assertEquals(listOf(7L), binds); assertEquals(2, observations)
            val monitor = NetworkInventoryState<LanLinkState>()
            assertTrue(monitor.update(7L, mapOf(7L to link)))
            val before = monitor.current
            assertTrue(monitor.update(7L, mapOf(7L to link.copy(addresses = setOf(changed)))))
            assertNotEquals(before, monitor.current)
            assertFalse(monitor.update(7L, mapOf(7L to link.copy(addresses = setOf(changed)))))
        }
    }

    @Test fun unchangedRouteCannotHideSourceAddressInterfaceOrMtuReplacementDuringBind() {
        val link = LanLinkState("wlan0", 1500, setOf(address("192.168.1.10", 24), address("fd00::10", 64)))
        val route = LanRoute(7, 24, path = LanRoutePath("192.168.1.0", null, "wlan0"), link = link)
        for (changed in listOf(null, link.copy(networkInterface = "wlan1"), link.copy(mtu = 1280),
            link.copy(addresses = emptySet()), link.copy(addresses = setOf(address("192.168.1.11", 24), address("fd00::10", 64))),
            link.copy(addresses = setOf(address("192.168.1.10", 16), address("fd00::10", 64))),
            link.copy(addresses = link.addresses + address("fd00::11", 64)))) {
            var reads = 0
            val bound = mutableListOf<Long>()
            var connected = false
            assertTrue(runCatching {
                LanRoutePolicy.bindStable(observe = {
                    listOf(if (reads++ == 0) route else route.copy(link = changed))
                }, bind = { bound += it })
                connected = true
            }.exceptionOrNull() is java.io.IOException)
            assertFalse(connected); assertEquals(listOf(7L), bound); assertEquals(2, reads)
        }
    }
    @Test fun linkAddressEnumerationOrderDoesNotInvalidateAStableLanBinding() {
        val addresses = listOf(address("192.168.1.10", 24), address("fd00::10", 64))
        var reads = 0
        val bindings = mutableListOf<Long>()
        LanRoutePolicy.bindStable(observe = {
            listOf(LanRoute(7, 24, link = LanLinkState("wlan0", 1500,
                (if (reads++ == 0) addresses else addresses.reversed() + addresses.first()).toSet())))
        }, bind = { bindings += it })
        assertEquals(listOf(7L), bindings); assertEquals(2, reads)
    }

    @Test fun stableBindingReobservesBeforeAllowingConnectWithoutDependingOnRouteOrder() {
        val events = mutableListOf<String>()
        val routes = listOf(LanRoute(7, 24), LanRoute(8, 16))
        var reads = 0
        LanRoutePolicy.bindStable(observe = {
            events += "observe"
            if (reads++ == 0) routes else routes.reversed() + routes.first()
        }, bind = { events += "bind:$it" })
        events += "connect"
        assertEquals(listOf("observe", "bind:7", "observe", "connect"), events)
        assertEquals(2, reads)
    }
    @Test fun bindingRaceCannotConnectOrRetryWithDefaultChangedAmbiguousOrRejectedRoutes() {
        val before = listOf(LanRoute(7, 24))
        for (after in listOf(emptyList(), listOf(LanRoute(8, 24)),
            listOf(LanRoute(7, 24), LanRoute(8, 24)), listOf(LanRoute(7, 24, false)),
            listOf(LanRoute(7, 16)), listOf(LanRoute(7, 24), LanRoute(7, 32, false)))) {
            var reads = 0
            val bound = mutableListOf<Long>()
            var connected = false
            val error = runCatching {
                LanRoutePolicy.bindStable(observe = { if (reads++ == 0) before else after }, bind = { bound += it })
                connected = true
            }.exceptionOrNull()
            assertTrue("$after", error is java.io.IOException)
            assertFalse(connected)
            assertEquals(listOf(7L), bound)
            assertEquals(2, reads)
        }
    }
    @Test fun bindingFailuresAndIncompleteSecondObservationsNeverPermitConnect() {
        val failure = SecurityException("network observation failed")
        var reads = 0
        var bindings = 0
        var connected = false
        assertSame(failure, runCatching {
            LanRoutePolicy.bindStable(observe = {
                if (reads++ == 0) listOf(LanRoute(7, 24)) else throw failure
            }, bind = { bindings++ })
            connected = true
        }.exceptionOrNull())
        assertEquals(1, bindings); assertFalse(connected)
        reads = 0
        assertSame(failure, runCatching {
            LanRoutePolicy.bindStable(observe = { reads++; listOf(LanRoute(7, 24)) }, bind = { throw failure })
        }.exceptionOrNull())
        assertEquals(1, reads)
        for (routes in listOf(emptyList(), listOf(LanRoute(7, 24, false)), listOf(LanRoute(7, 24), LanRoute(8, 24)))) {
            assertTrue(runCatching {
                LanRoutePolicy.bindStable(observe = { routes }, bind = { fail("Invalid route must not bind") })
            }.exceptionOrNull() is java.io.IOException)
        }
    }
    @Test fun sameNetworkPrefixWithChangedGatewayOrInterfaceIsStillAChangedRoute() {
        val before = LanRoute(7, 24, path = LanRoutePath("192.168.1.0", null, "wlan0"))
        for (path in listOf(LanRoutePath("192.168.1.0", "192.168.1.1", "wlan0"),
            LanRoutePath("192.168.1.0", null, "wlan1"), LanRoutePath("192.168.2.0", null, "wlan0"))) {
            var reads = 0
            var connected = false
            assertTrue(runCatching {
                LanRoutePolicy.bindStable(observe = {
                    listOf(if (reads++ == 0) before else before.copy(path = path))
                }, bind = { assertEquals(7L, it) })
                connected = true
            }.exceptionOrNull() is java.io.IOException)
            assertFalse(connected)
        }
    }
    @Test fun literalGuardNeverLetsHostnameOrLegacyIpv4ReachResolution() {
        for (host in listOf("deadbeef", "abc", "123", "127.1", "01.2.3.4", "256.1.2.3", "example.invalid", "0.0.0.0", "224.0.0.1", "255.255.255.255", "fe80::1%wlan0"))
            assertFalse(host, LanRoutePolicy.literal(host))
        for (host in listOf("192.168.1.2", "fd00::1", "::ffff:192.168.1.2", "::1")) assertTrue(host, LanRoutePolicy.literal(host))
        for (host in listOf("::::", "1::2::3", "dead::beef::1")) assertFalse(host, LanRoutePolicy.literal(host))
    }
    @Test fun directOfflineLanDoesNotNeedDefaultOrInternetValidation() {
        assertEquals(7L, LanRoutePolicy.select(listOf(LanRoute(7, 24))))
        assertNull(LanRoutePolicy.select(emptyList()))
    }
    @Test fun longestMatchingPrefixWinsRegardlessOfEnumerationOrder() {
        val routes = listOf(LanRoute(1, 16), LanRoute(2, 24), LanRoute(3, 8))
        assertEquals(2L, LanRoutePolicy.select(routes))
        assertEquals(2L, LanRoutePolicy.select(routes.reversed()))
        assertEquals(4L, LanRoutePolicy.select(listOf(LanRoute(3, 48), LanRoute(4, 64))))
    }
    @Test fun duplicateRoutesOnOneNetworkAreNotAmbiguous() {
        assertEquals(9L, LanRoutePolicy.select(listOf(LanRoute(9, 24), LanRoute(9, 24), LanRoute(2, 16))))
    }
    @Test fun equallySpecificDifferentNetworksFailInsteadOfGuessing() {
        for (routes in listOf(listOf(LanRoute(1, 24), LanRoute(2, 24)),
            listOf(LanRoute(1, 64), LanRoute(2, 64)))) {
            assertTrue(runCatching { LanRoutePolicy.select(routes) }.exceptionOrNull() is java.io.IOException)
            assertTrue(runCatching { LanRoutePolicy.select(routes.reversed()) }.isFailure)
        }
    }
    @Test fun moreSpecificUnreachableRouteMustNotFallBackToBroaderLan() {
        for (routes in listOf(listOf(LanRoute(1, 16), LanRoute(1, 24, false)),
            listOf(LanRoute(1, 16), LanRoute(2, 24, false)), listOf(LanRoute(1, 64, false)))) {
            assertTrue(runCatching { LanRoutePolicy.select(routes) }.exceptionOrNull() is java.io.IOException)
            assertTrue(runCatching { LanRoutePolicy.select(routes.reversed()) }.isFailure)
        }
    }
    @Test fun equalSpecificityRejectOrThrowWinsEvenOnTheSameNetwork() {
        for (other in listOf(1L, 2L)) {
            assertTrue(runCatching { LanRoutePolicy.select(listOf(LanRoute(1, 24), LanRoute(other, 24, false))) }.isFailure)
        }
    }
    @Test fun moreSpecificForwardingRouteCanOverrideABroaderReject() {
        assertEquals(2L, LanRoutePolicy.select(listOf(LanRoute(1, 16, false), LanRoute(2, 24))))
        assertEquals(2L, LanRoutePolicy.select(listOf(LanRoute(1, 48, false), LanRoute(2, 64))))
    }
    @Test fun invalidOrDefaultPrefixCannotEnterExplicitLanSelection() {
        for (prefix in listOf(-1, 0, 129, Int.MAX_VALUE))
            assertTrue(runCatching { LanRoutePolicy.select(listOf(LanRoute(1, prefix))) }.isFailure)
        assertEquals(1L, LanRoutePolicy.select(listOf(LanRoute(1, 128))))
    }
    @Test fun exhaustiveTwoNetworkPrefixAndRefusalMatrixUsesOnlyTheMostSpecificRoutes() {
        for (firstPrefix in 1..32) for (secondPrefix in 1..32)
            for (firstForward in listOf(false, true)) for (secondForward in listOf(false, true)) {
                val routes = listOf(LanRoute(1, firstPrefix, firstForward), LanRoute(2, secondPrefix, secondForward))
                val expected = when {
                    firstPrefix > secondPrefix && firstForward -> 1L
                    secondPrefix > firstPrefix && secondForward -> 2L
                    else -> null
                }
                for (ordered in listOf(routes, routes.reversed())) {
                    val result = runCatching { LanRoutePolicy.select(ordered) }
                    if (expected == null) assertTrue("$ordered", result.exceptionOrNull() is java.io.IOException)
                    else assertEquals("$ordered", expected, result.getOrThrow())
                }
            }
    }
}
