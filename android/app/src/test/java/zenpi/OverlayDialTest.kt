package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.lifecycle.LanRoutePolicy
import zenpi.pairing.AddressHint
import zenpi.transport.HintDialer
import zenpi.transport.HostIdentityException
import java.io.IOException
import java.net.InetAddress

/** ZE-516: T1 overlay ranges, VPN admission and LAN-first multi-hint dialing. */
class OverlayDialTest {
    private fun ip(value: String) = InetAddress.getByName(value)

    @Test fun cgnatAndTailscaleUlaAreOverlayAndNothingElseIs() {
        for (overlay in listOf("100.64.0.0", "100.64.0.1", "100.101.102.103", "100.127.255.255",
            "fd7a:115c:a1e0::", "fd7a:115c:a1e0::1", "fd7a:115c:a1e0:ffff:ffff:ffff:ffff:ffff", "::ffff:100.100.1.1"))
            assertTrue(overlay, LanRoutePolicy.overlay(ip(overlay)))
        for (other in listOf("100.63.255.255", "100.128.0.0", "10.0.0.1", "192.168.1.5", "172.16.0.1", "8.8.8.8",
            "fd7a:115c:a1e1::1", "fd7a:115d:a1e0::1", "fd00::1", "fe80::1", "2001:db8::1"))
            assertFalse(other, LanRoutePolicy.overlay(ip(other)))
    }

    @Test fun vpnIsAdmittedOnlyForOverlayTargetsAndLanStaysOnWifiOrEthernet() {
        // VPN transport: overlay targets only.
        assertTrue(LanRoutePolicy.eligible(ip("100.101.1.2"), vpn = true, localLink = false, notRestricted = true))
        assertTrue(LanRoutePolicy.eligible(ip("fd7a:115c:a1e0::5"), vpn = true, localLink = false, notRestricted = true))
        for (target in listOf("192.168.1.5", "10.8.0.1", "fd00::1", "8.8.8.8", "100.128.0.1"))
            assertFalse(target, LanRoutePolicy.eligible(ip(target), vpn = true, localLink = true, notRestricted = true))
        // Restricted networks are never used.
        assertFalse(LanRoutePolicy.eligible(ip("100.101.1.2"), vpn = true, localLink = false, notRestricted = false))
        assertFalse(LanRoutePolicy.eligible(ip("192.168.1.5"), vpn = false, localLink = true, notRestricted = false))
        // Non-VPN: Wi-Fi/Ethernet only (no cellular default route), for any literal target.
        assertTrue(LanRoutePolicy.eligible(ip("192.168.1.5"), vpn = false, localLink = true, notRestricted = true))
        assertFalse(LanRoutePolicy.eligible(ip("192.168.1.5"), vpn = false, localLink = false, notRestricted = true))
    }

    private val lan = AddressHint("lan", "192.168.50.55", 7777)
    private val tailnet = AddressHint("tailnet", "100.101.102.103", 7777)
    private val tailnet6 = AddressHint("tailnet", "fd7a:115c:a1e0::1", 7777)

    @Test fun lanIsDialedFirstThenTheNextHintInOrder() {
        val tried = mutableListOf<AddressHint>()
        val (chosen, value) = HintDialer.dial(listOf(tailnet, lan, tailnet6)) { hint, timeout ->
            tried += hint; assertEquals(3_000, timeout)
            if (hint == lan) throw java.net.ConnectException("LAN unreachable") else "connected:${hint.host}"
        }
        assertEquals(listOf(lan, tailnet), tried)
        assertEquals(tailnet, chosen); assertEquals("connected:100.101.102.103", value)
        assertEquals(lan to 5_000, HintDialer.dial(listOf(lan)) { hint, timeout -> timeout }.let { it.first to it.second })
    }

    @Test fun pinMismatchOnAnyHintFailsWithoutTryingAnother() {
        for (failing in listOf(lan, tailnet)) {
            val tried = mutableListOf<AddressHint>()
            val error = runCatching {
                HintDialer.dial(listOf(lan, tailnet, tailnet6)) { hint, _ ->
                    tried += hint
                    when (hint) {
                        failing -> throw javax.net.ssl.SSLHandshakeException("pin").apply { initCause(HostIdentityException()) }
                        lan -> throw java.net.NoRouteToHostException("LAN down")
                        else -> "connected"
                    }
                }
            }.exceptionOrNull()
            assertTrue(error is javax.net.ssl.SSLHandshakeException)
            assertEquals(failing, tried.last())
            assertFalse(tried.contains(tailnet6))
        }
    }

    @Test fun everyHintUnreachableSurfacesTheLastFailureAndCancellationIsNotSwallowed() {
        val error = runCatching {
            HintDialer.dial(listOf(lan, tailnet)) { hint, _ -> throw IOException("down ${hint.kind}") }
        }.exceptionOrNull()
        assertEquals("down tailnet", error?.message)
        assertEquals("down lan", error?.suppressed?.single()?.message)
        assertTrue(runCatching {
            HintDialer.dial(listOf(lan, tailnet)) { _, _ -> throw kotlinx.coroutines.CancellationException("stop") }
        }.exceptionOrNull() is kotlinx.coroutines.CancellationException)
    }
}
