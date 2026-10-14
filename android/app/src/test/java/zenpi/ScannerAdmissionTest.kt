package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class ScannerAdmissionTest {
    @Test fun anOutstandingCameraResultAlwaysFencesAnotherLaunch() {
        for (phase in PairingPhase.entries) {
            assertFalse(PairingState(phase = phase, scanInProgress = true).canScan)
            assertEquals(phase != PairingPhase.CONNECTING, PairingState(phase = phase).canScan)
        }
        assertFalse(PairingState(resetting = true).canScan)
        assertFalse(PairingState(resetRequired = true).canScan)
        assertFalse(PairingState(pendingRevoke = "unknown").canScan)
    }
    @Test fun cameraAdmissionNeverAllowsCurrentOrUnknownWritesToOverlap() {
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", promptControl = true)),
            "project", "session", "idle", emptyList(), "1", false, "0", workspaceOwner = "a".repeat(64), workspaceRevision = "1")
        val command = WorkspaceCommand.prompt(projection, "message")
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection)
        assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state.copy(scanInProgress = true), true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state.copy(scanInProgress = true, pendingWorkspace = command), true, true, command))
        assertFalse(state.copy(pendingWorkspace = command).canScan)
    }
    @Test fun scanPauseRetainsDisplayButRequiresFreshAuthenticationForControls() {
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", workspaceControl = true,
            promptControl = true, approvalControl = true)), "project", "session", "idle",
            listOf(RemoteTurn("turn", "assistant", "desktop answer")), "9", false, "7",
            workspaceOwner = "a".repeat(64), workspaceRevision = "1")
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection,
            host = zenpi.security.PairedHost("192.168.1.5", 7777, "b".repeat(64), "c".repeat(64)), registered = true)
        val paused = ConnectionCancelPolicy.stopped(state).copy(scanInProgress = true)
        assertEquals(PairingPhase.OFFLINE, paused.phase)
        assertEquals(state.host, paused.host)
        assertEquals(projection.turns, paused.projection?.turns)
        assertEquals("7", paused.projection?.eventSequence)
        assertNull(paused.projection?.workspaceOwner)
        assertTrue(requireNotNull(paused.projection).projects.all { !it.workspaceControl && !it.promptControl && !it.approvalControl })
        assertFalse(WorkspaceDispatchPolicy.allows(paused, true, true, WorkspaceCommand.prompt(projection, "message")))
        assertFalse(paused.canScan)
        assertTrue(paused.copy(scanInProgress = false).canScan)
    }
}
