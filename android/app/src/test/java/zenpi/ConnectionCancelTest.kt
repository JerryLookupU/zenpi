package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.security.PairedHost
import zenpi.state.ProjectionCacheCodec
import zenpi.transport.*

class ConnectionCancelTest {
    private val host = PairedHost("192.168.1.20", 7777, "a".repeat(64), "b".repeat(64))
    private val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", workspaceControl = true,
        promptControl = true, promptStatuses = listOf(RemotePromptStatus("running", "project", "session", "running", "live")),
        pendingApprovals = listOf(RemoteApproval("approval", "project", "session", "turn", "call", "write", "workspace_write", "agent_tool", "d".repeat(64))),
        approvalControl = true, desktopStreamText = "live")), "project", "session", "running",
        listOf(RemoteTurn("turn", "assistant", "durable")), "4", false, "2", workspaceOwner = "c".repeat(64), workspaceRevision = "3")
    private val state = PairingState(phase = PairingPhase.CONNECTED, registered = true, host = host, projection = projection,
        terminalOpen = true, sessionsOpen = true, diffOpen = true, filesOpen = true, resourcesOpen = true)
    @Test fun cancelRetainsPairingAndDurableCursorButRemovesLiveAuthorityAndReaders() {
        val stopped = ConnectionCancelPolicy.stopped(state)
        assertEquals(host, stopped.host); assertTrue(stopped.registered); assertEquals(PairingPhase.OFFLINE, stopped.phase)
        assertEquals(ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(projection)), stopped.projection)
        assertFalse(stopped.terminalOpen); assertFalse(stopped.sessionsOpen); assertFalse(stopped.diffOpen)
        assertFalse(stopped.filesOpen); assertFalse(stopped.resourcesOpen); assertNull(stopped.error)
        assertNull(stopped.invite); assertNull(stopped.pendingWorkspace)
        assertEquals(stopped, ConnectionCancelPolicy.stopped(stopped))
        assertFalse(WorkspaceDispatchPolicy.allows(stopped, true, true, WorkspaceCommand.prompt(projection, "new")))
    }
    @Test fun cancelNeverResolvesUnknownWriteOrRevocation() {
        val command = WorkspaceCommand.prompt(projection, "pending")
        val stopped = ConnectionCancelPolicy.stopped(state.copy(pendingWorkspace = command))
        assertEquals(command.envelope, stopped.pendingWorkspace?.envelope)
        assertTrue(requireNotNull(stopped.error).contains("unknown"))
        assertTrue(WorkspaceDispatchPolicy.allows(stopped, true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(stopped, false, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(stopped, true, false, command))
        val revoking = ConnectionCancelPolicy.stopped(state.copy(pendingRevoke = "pending-revoke"))
        assertEquals("pending-revoke", revoking.pendingRevoke); assertEquals(host, revoking.host)
        assertFalse(WorkspaceDispatchPolicy.allows(revoking, true, true, command))
    }
    @Test fun unpairedConfirmationCancelReturnsToEmptyStateWithoutInventingAHost() {
        assertEquals(PairingState(), ConnectionCancelPolicy.stopped(PairingState(phase = PairingPhase.CONFIRMING)))
        assertEquals(PairingState(), ConnectionCancelPolicy.stopped(PairingState(phase = PairingPhase.ERROR, error = "pairing incomplete")))
    }
    @Test fun offlineBoundaryCannotRegainOldControlsWhenNextAuthenticationFails() {
        val stopped = AuthorityLossPolicy.displayOnly(state).copy(phase = PairingPhase.OFFLINE)
        val reconnectFailed = stopped.copy(phase = PairingPhase.ERROR)
        assertEquals(projection.turns, reconnectFailed.projection?.turns)
        assertEquals(projection.eventSequence, reconnectFailed.projection?.eventSequence)
        val oldCancel = WorkspaceCommand.cancelPrompt(projection, "running")
        val oldSteer = WorkspaceCommand.steer(projection, "running", "change")
        val oldApprove = WorkspaceCommand.approve(projection, projection.projects.single().pendingApprovals.single(), "allow")
        for (command in listOf(oldCancel, oldSteer, oldApprove)) {
            assertFalse(WorkspaceDispatchPolicy.allows(stopped, true, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(reconnectFailed, true, true, command))
        }
        val pending = WorkspaceCommand.prompt(projection, "pending")
        val unresolved = AuthorityLossPolicy.displayOnly(state.copy(pendingWorkspace = pending)).copy(phase = PairingPhase.ERROR)
        assertEquals(pending.envelope, unresolved.pendingWorkspace?.envelope)
        assertTrue(WorkspaceDispatchPolicy.allows(unresolved, true, true, pending))
        assertFalse(WorkspaceDispatchPolicy.allows(unresolved, true, true, oldCancel))
    }
}
