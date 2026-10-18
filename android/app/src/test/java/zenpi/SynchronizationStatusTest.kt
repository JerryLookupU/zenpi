package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class SynchronizationStatusTest {
    private val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", promptControl = true)),
        "project", "session", "idle", emptyList(), "1", false, "1", workspaceOwner = "a".repeat(64), workspaceRevision = "1",
        snapshotReplaced = true)
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection)

    @Test fun pendingTailTakesPriorityAndOrdinaryConnectedReadHasNoRecoveryClaim() {
        assertEquals("Snapshot replaced", SynchronizationStatus.label(state))
        assertEquals("Catching up", SynchronizationStatus.label(state.copy(projection = projection.copy(hasMoreEvents = true))))
        assertNull(SynchronizationStatus.label(state.copy(projection = projection.copy(snapshotReplaced = false))))
        assertNull(SynchronizationStatus.label(state.copy(projection = null)))
    }

    @Test fun offlineErrorsAndUnresolvedOperationsCannotClaimLiveRecovery() {
        for (phase in PairingPhase.values().filter { it != PairingPhase.CONNECTED })
            assertNull(SynchronizationStatus.label(state.copy(phase = phase)))
        val pending = WorkspaceCommand.prompt(projection, "pending")
        for (next in listOf(state.copy(scanInProgress = true), state.copy(resetting = true), state.copy(resetRequired = true),
            state.copy(pendingWorkspace = pending), state.copy(pendingRevoke = "unknown")))
            assertNull(SynchronizationStatus.label(next))
        val offline = AuthorityLossPolicy.displayOnly(state)
        assertFalse(requireNotNull(offline.projection).snapshotReplaced)
    }
}
