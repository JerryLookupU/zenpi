package zenpi

import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Test
import zenpi.lifecycle.LanSocketRouting
import java.net.Socket
import kotlinx.coroutines.runBlocking
import zenpi.security.DeviceIdentity
import zenpi.security.PairedHost
import zenpi.pairing.PairingInvite
import zenpi.transport.HostClient
import zenpi.transport.PairingClient

class LanSocketRoutingTest {
    @Test fun pairingKeyDeletionDuringBindingClosesSocketBeforeTcpConnect() = runBlocking {
        val alias = "zenpi.routing-key-loss.${System.nanoTime()}"
        val identity = DeviceIdentity.loadOrCreate(alias)
        val token = java.util.Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32))
        val invite = PairingInvite.parse("zenpi://pair?v=1&address=192.0.2.1%3A7777&fingerprint=${"a".repeat(64)}&token=$token&expires=${System.currentTimeMillis() / 1000 + 60}")
        var captured: Socket? = null; var binds = 0
        try {
            val error = runCatching {
                PairingClient { socket, host ->
                    assertEquals(invite.host, host)
                    assertFalse(socket.isConnected); assertFalse(socket.isClosed)
                    captured = socket; binds++
                    identity.wipe()
                }.pair(invite, invite.safetyCode, "routing key loss", identity)
            }.exceptionOrNull()
            assertTrue(error is zenpi.transport.HostAuthenticationException)
            assertEquals(1, binds)
            assertFalse(requireNotNull(captured).isConnected)
            assertTrue(requireNotNull(captured).isClosed)
            assertNull(DeviceIdentity.loadExisting(alias))
        } finally { identity.wipe() }
    }
    @Test fun authenticatedKeyDeletionDuringBindingClosesSocketBeforeTcpConnect() = runBlocking {
        val alias = "zenpi.routing-auth-key-loss.${System.nanoTime()}"
        val identity = DeviceIdentity.loadOrCreate(alias)
        val host = PairedHost("192.0.2.1", 7777, "a".repeat(64), identity.deviceId)
        var captured: Socket? = null; var binds = 0
        try {
            val error = runCatching {
                HostClient { socket, address ->
                    assertEquals(host.host, address)
                    assertFalse(socket.isConnected); assertFalse(socket.isClosed)
                    captured = socket; binds++
                    identity.wipe()
                }.inspect(host, identity = identity)
            }.exceptionOrNull()
            assertTrue(error is zenpi.transport.HostAuthenticationException)
            assertEquals(1, binds)
            assertFalse(requireNotNull(captured).isConnected)
            assertTrue(requireNotNull(captured).isClosed)
            assertNull(DeviceIdentity.loadExisting(alias))
        } finally { identity.wipe() }
    }
    @Test fun loopbackFixtureBypassesNetworkBindingWithoutChangingProcessRoute() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val connectivity = context.getSystemService(android.net.ConnectivityManager::class.java)
        val before = connectivity.boundNetworkForProcess
        val routing = LanSocketRouting(context)
        Socket().use { socket ->
            routing.bind(socket, "127.0.0.1")
            assertFalse(socket.isConnected); assertFalse(socket.isClosed)
        }
        assertEquals(before, connectivity.boundNetworkForProcess)
    }
    @Test fun closedSocketAndHostnameCannotReachBindingOrDns() {
        val routing = LanSocketRouting(InstrumentationRegistry.getInstrumentation().targetContext)
        val closed = Socket().apply { close() }
        assertTrue(runCatching { routing.bind(closed, "192.168.1.2") }.exceptionOrNull() is java.io.IOException)
        Socket().use { socket ->
            assertTrue(runCatching { routing.bind(socket, "example.invalid") }.exceptionOrNull() is java.io.IOException)
            assertFalse(socket.isConnected)
        }
    }
    @Test fun mappedUnsafeDestinationsCannotBypassLiteralRoutePolicy() {
        val routing = LanSocketRouting(InstrumentationRegistry.getInstrumentation().targetContext)
        for (host in listOf("::ffff:169.254.1.2", "::ffff:255.255.255.255")) {
            Socket().use { socket ->
                assertTrue(host, runCatching { routing.bind(socket, host) }.isFailure)
                assertFalse(socket.isConnected)
            }
        }
    }
    @Test fun unmatchedTargetDoesNotFallbackToDefaultOrChangeProcessBinding() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val connectivity = context.getSystemService(android.net.ConnectivityManager::class.java)
        assertNotNull("An active emulator network is required", connectivity.activeNetwork)
        val before = connectivity.boundNetworkForProcess
        Socket().use { socket ->
            assertTrue(runCatching { LanSocketRouting(context).bind(socket, "192.0.2.1") }.isFailure)
            assertFalse(socket.isConnected); assertFalse(socket.isClosed)
        }
        assertEquals(before, connectivity.boundNetworkForProcess)
    }
    @Test fun pairingAndAuthenticatedReadBindBeforeConnectAndCloseAfterBindingFailure() = runBlocking {
        val alias = "zenpi.routing-test.${System.nanoTime()}"
        val identity = DeviceIdentity.loadOrCreate(alias)
        val host = PairedHost("192.0.2.1", 7777, "a".repeat(64), identity.deviceId)
        val token = java.util.Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32))
        val invite = PairingInvite.parse("zenpi://pair?v=1&address=192.0.2.1%3A7777&fingerprint=${host.fingerprint}&token=$token&expires=${System.currentTimeMillis() / 1000 + 60}")
        var captured: Socket? = null
        var calls = 0
        val bind: (Socket, String) -> Unit = { socket, address ->
            assertEquals(host.host, address)
            assertFalse(socket.isConnected); assertFalse(socket.isClosed)
            captured = socket; calls++
            throw java.io.IOException("fixture binding failed")
        }
        try {
            val wrongCode = if (invite.safetyCode == "000000") "111111" else "000000"
            assertTrue(runCatching { PairingClient(bind).pair(invite, wrongCode, "routing test", identity) }.exceptionOrNull() is zenpi.transport.WireException)
            assertEquals("Invalid confirmation must not bind a socket", 0, calls)
            val foreignHost = host.copy(deviceId = if (identity.deviceId == "b".repeat(64)) "c".repeat(64) else "b".repeat(64))
            assertTrue(runCatching { HostClient(bind).inspect(foreignHost, identity = identity) }.exceptionOrNull() is zenpi.transport.WireException)
            assertEquals("Saved identity mismatch must not bind a socket", 0, calls)
            assertEquals(host.deviceId, DeviceIdentity.loadExisting(alias)?.deviceId)
            assertTrue(runCatching { PairingClient(bind).pair(invite, invite.safetyCode, "routing test", identity) }.exceptionOrNull() is java.io.IOException)
            assertEquals(1, calls); assertTrue(requireNotNull(captured).isClosed)
            captured = null
            assertTrue(runCatching { HostClient(bind).inspect(host, identity = identity) }.exceptionOrNull() is java.io.IOException)
            assertEquals(2, calls); assertTrue(requireNotNull(captured).isClosed)
            InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", "explicit_identity_preconnect_fences_verified")
            })
        } finally { identity.wipe() }
    }
}
