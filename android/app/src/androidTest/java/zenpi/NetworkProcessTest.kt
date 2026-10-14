package zenpi

import zenpi.ui.PairingScreen
import androidx.test.platform.app.InstrumentationRegistry
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.*
import androidx.compose.runtime.collectAsState
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.security.*
import zenpi.transport.*
import zenpi.lifecycle.LanSocketRouting

class NetworkProcessTest {
    @get:Rule val compose = createComposeRule()
    private fun hostClient() = HostClient(LanSocketRouting(InstrumentationRegistry.getInstrumentation().targetContext)::bind)
    @Test fun cachedOwnerRemainsAliveUntilHarnessKillsItsProcess(): Unit = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val namespace = requireNotNull(InstrumentationRegistry.getArguments().getString("process_namespace"))
        val store = PairedHostStore(instrumentation.targetContext, namespace)
        assertNotNull(store.loadState()?.projection?.eventSequence)
        if (InstrumentationRegistry.getArguments().getString("pending_write") == "true") {
            val saved = requireNotNull(store.loadState())
            val identity = DeviceIdentity.loadOrCreate("zenpi.network-process.$namespace")
            val current = hostClient().inspect(saved.host, identity = identity, previous = saved.projection)
            val command = WorkspaceCommand.create(current, requireNotNull(current.selectedId), "workspace_select")
            store.beginWorkspace(saved.host, command)
            val committed = hostClient().inspect(saved.host, identity = identity, workspaceCommand = command)
            assertTrue(java.math.BigInteger(requireNotNull(committed.workspaceRevision)) > java.math.BigInteger(requireNotNull(current.workspaceRevision)))
            assertEquals(command.envelope, store.loadState()?.pendingWorkspace?.envelope)
            // Keep the pending record to exercise a crash after host commit but before local resolution.
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "pending_write_committed_before_death") })
        }
        instrumentation.sendStatus(0, android.os.Bundle().apply {
            putString("zenpi_phase", "network_process_live_ready")
            putString("network_process_pid", android.os.Process.myPid().toString())
        })
        delay(60_000)
        error("Harness did not kill the live process within the deadline")
    }
    @Test fun newProcessAuthenticatesAndReplaysFromItsEncryptedCursor() = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        val namespace = requireNotNull(args.getString("process_namespace"))
        val expected = requireNotNull(args.getString("expected_text"))
        assertNotEquals(requireNotNull(args.getString("previous_pid")).toInt(), android.os.Process.myPid())
        val context = instrumentation.targetContext
        val store = PairedHostStore(context, namespace)
        val identity = DeviceIdentity.loadOrCreate("zenpi.network-process.$namespace")
        val saved = requireNotNull(store.loadState())
        if (args.getString("pending_write") == "true") assertNotNull("Pending request must survive process death", saved.pendingWorkspace)
        val seed = requireNotNull(saved.projection)
        assertNotNull(seed.subtabs)
        assertFalse(requireNotNull(seed.subtabs).control)
        assertFalse(requireNotNull(seed.subtabs).directoryControl)
        val model = PairingViewModel(context.applicationContext as android.app.Application, store, identity)
        var awaitingRevokeDeath = false
        try {
            assertEquals(saved.host.deviceId, identity.deviceId)
            repeat(30) { if (model.state.value.host == null) delay(100) }
            assertEquals(seed, model.state.value.projection)
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            compose.runOnUiThread { model.setForeground(true) }
            val pending = saved.pendingWorkspace
            if (pending != null) {
                assertEquals(pending.envelope, model.state.value.pendingWorkspace?.envelope)
                delay(300)
                assertEquals(pending.envelope, store.loadState()?.pendingWorkspace?.envelope)
                compose.runOnUiThread { model.networkChanged(null); model.networkChanged(99L) }
                delay(300)
                assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
                assertEquals(pending.envelope, model.state.value.pendingWorkspace?.envelope)
                assertEquals(pending.envelope, store.loadState()?.pendingWorkspace?.envelope)
                compose.runOnUiThread { model.cancel(); model.acceptUri("not-an-invitation") }
                assertEquals(pending.envelope, model.state.value.pendingWorkspace?.envelope)
                assertEquals(pending.envelope, store.loadState()?.pendingWorkspace?.envelope)
                instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "pending_write_explicit_retry_requested") })
                compose.runOnUiThread { model.retryWorkspace() }
            }
            repeat(100) {
                if (model.state.value.projection?.turns?.none { it.content.contains(expected) } != false
                    || model.state.value.phase != PairingPhase.CONNECTED || model.state.value.pendingWorkspace != null) delay(200)
            }
            val resumed = requireNotNull(model.state.value.projection)
            assertEquals(PairingPhase.CONNECTED, model.state.value.phase)
            assertEquals(seed.sessionId, resumed.sessionId)
            assertTrue(resumed.turns.any { it.content.contains(expected) })
            val omitted = seed.turns.filter { it.toolCallsOmitted }
            if (omitted.isEmpty()) {
                assertTrue("A complete cache must resume through its event cursor", resumed.replayedEventCount > 0)
            } else {
                assertFalse(TranscriptRestorePolicy.replay(seed, resumed.projects.single { it.id == seed.selectedId }))
                assertEquals("Omitted tool metadata requires authenticated rehydration before replay", 0, resumed.replayedEventCount)
                for (cached in omitted) {
                    val hydrated = resumed.turns.single { it.id == cached.id }
                    assertEquals(cached.role, hydrated.role)
                    assertEquals(cached.content, hydrated.content)
                    assertFalse(hydrated.toolCallsOmitted)
                    assertTrue("Authenticated snapshot must restore omitted tool metadata", hydrated.toolCalls.isNotEmpty())
                }
            }
            val cachedResumed = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(resumed))
            assertEquals("Recovery must preserve the existing cached message prefix", seed.turns,
                cachedResumed.turns.take(seed.turns.size))
            instrumentation.sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", if (omitted.isEmpty()) "network_process_cursor_replay_verified"
                    else "network_process_tool_metadata_rehydrated_verified")
                putString("network_process_seed_cursor", requireNotNull(seed.eventSequence))
                putString("network_process_recovered_cursor", requireNotNull(resumed.eventSequence))
            })
            assertTrue(java.math.BigInteger(requireNotNull(resumed.eventSequence)) > java.math.BigInteger(requireNotNull(seed.eventSequence)))
            assertEquals(resumed.eventSequence, store.loadState()?.projection?.eventSequence)
            if (pending != null) {
                assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
                val beforeRetry = requireNotNull(resumed.workspaceRevision)
                compose.runOnUiThread { model.setForeground(false) }
                instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "pending_write_duplicate_requested") })
                val duplicate = hostClient().inspect(saved.host, identity = identity, workspaceCommand = pending)
                assertEquals(beforeRetry, duplicate.workspaceRevision)
                instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "pending_write_process_recovery_verified") })
            }
            compose.runOnUiThread { model.setForeground(false) }
            instrumentation.sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", "network_process_resume_verified")
                putString("network_process_pid", android.os.Process.myPid().toString())
            })
            val requestId = java.util.UUID.randomUUID().toString()
            store.beginRevoke(saved.host, requestId)
            hostClient().revokeSelf(saved.host, identity, requestId)
            assertEquals(requestId, store.loadState()?.pendingRevoke)
            if (args.getString("revoke_process_death") == "true") {
                awaitingRevokeDeath = true
                instrumentation.sendStatus(0, android.os.Bundle().apply {
                    putString("zenpi_phase", "network_process_live_ready")
                    putString("revoke_process_pid", android.os.Process.myPid().toString())
                    putString("revoke_request_id", requestId)
                })
                delay(60_000)
                error("Harness did not kill the revoked process within the deadline")
            }
            val recovered = PairingViewModel(context.applicationContext as android.app.Application, PairedHostStore(context, namespace), identity)
            try {
                repeat(30) { if (recovered.state.value.host == null) delay(100) }
                assertEquals(requestId, recovered.state.value.pendingRevoke)
                compose.runOnUiThread { recovered.setForeground(true) }
                val denied = runCatching { hostClient().inspect(saved.host, seed.selectedId, identity) }.exceptionOrNull()
                assertTrue(denied is WireException && denied.message == "Host request refused: unauthorized")
                assertFalse(hostClient().checkRevoke(saved.host, identity, java.util.UUID.randomUUID().toString()))
                compose.setContent { androidx.compose.material3.MaterialTheme {
                    val state = recovered.state.collectAsState().value
                    PairingScreen(state, {}, {}, {}, onCheckRevoke = recovered::checkRevocation)
                } }
                compose.onNodeWithText("Check revocation status").performScrollTo().performClick()
                repeat(50) { if (recovered.state.value.registered) delay(100) }
                assertFalse(recovered.state.value.registered)
                assertEquals("Device revoked on host", recovered.state.value.error)
            } finally { compose.runOnUiThread { recovered.cancel() } }
            assertNull(store.loadState())
            assertNull(DeviceIdentity.loadExisting("zenpi.network-process.$namespace"))
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "android_self_revoke_verified") })
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "revoke_receipt_recovery_verified") })
            val failure = runCatching { hostClient().inspect(saved.host, seed.selectedId, identity) }.exceptionOrNull()
            assertNotNull("Removed device key must not authenticate", failure)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "host_state_verified") })
        } finally {
            compose.runOnUiThread { model.cancel() }
            if (!awaitingRevokeDeath) {
                store.wipe(); identity.wipe(); assertNull(store.loadState())
            }
        }
    }

    @Test fun newProcessResolvesRevocationReceiptAndWipesCredentials() = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        val namespace = requireNotNull(args.getString("process_namespace"))
        assertNotEquals(requireNotNull(args.getString("previous_pid")).toInt(), android.os.Process.myPid())
        val context = instrumentation.targetContext
        val store = PairedHostStore(context, namespace)
        val identity = requireNotNull(DeviceIdentity.loadExisting("zenpi.network-process.$namespace"))
        val saved = requireNotNull(store.loadState())
        val requestId = requireNotNull(args.getString("revoke_request_id"))
        assertEquals(requestId, saved.pendingRevoke)
        assertEquals(saved.host.deviceId, identity.deviceId)
        assertNull(store.loadResumeSnapshot())
        val model = PairingViewModel(context.applicationContext as android.app.Application, store, identity)
        try {
            repeat(30) { if (model.state.value.host == null) delay(100) }
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            assertEquals(requestId, model.state.value.pendingRevoke)
            compose.runOnUiThread { model.setForeground(true); model.networkChanged(null); model.networkChanged(99L) }
            delay(300)
            assertEquals(requestId, store.loadState()?.pendingRevoke)
            val denied = runCatching { hostClient().inspect(saved.host, identity = identity) }.exceptionOrNull()
            assertTrue(denied is WireException && denied.message == "Host request refused: unauthorized")
            assertFalse(hostClient().checkRevoke(saved.host, identity, java.util.UUID.randomUUID().toString()))
            compose.setContent { androidx.compose.material3.MaterialTheme {
                val state = model.state.collectAsState().value
                PairingScreen(state, {}, {}, {}, onCheckRevoke = model::checkRevocation)
            } }
            compose.onNodeWithText("Check revocation status").performScrollTo().performClick()
            repeat(50) { if (model.state.value.registered) delay(100) }
            assertFalse(model.state.value.registered)
            assertEquals("Device revoked on host", model.state.value.error)
            assertNull(store.loadState())
            assertNull(DeviceIdentity.loadExisting("zenpi.network-process.$namespace"))
            val failure = runCatching { hostClient().inspect(saved.host, identity = identity) }.exceptionOrNull()
            assertNotNull(failure)
            instrumentation.sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", "revoke_receipt_process_recovery_verified")
                putString("recovered_revoke_process_pid", android.os.Process.myPid().toString())
            })
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "android_self_revoke_verified") })
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "host_state_verified") })
        } finally {
            compose.runOnUiThread { model.cancel() }
            store.wipe(); identity.wipe()
        }
    }
}
