package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.security.*
import zenpi.transport.*

class RePairPolicyTest {
    private val host = PairedHost("192.168.1.20", 7777, "a".repeat(64), "b".repeat(64))
    private val tab = "d".repeat(64)
    private val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session",
        workspaceControl = true, promptControl = true, diffAvailable = true,
        promptStatuses = listOf(RemotePromptStatus("request", "project", "session", "running", "TRANSIENT_SECRET")),
        desktopStreamText = "TRANSIENT_SECRET", terminal = RemoteTerminal("project", "session", tab, "9", "TRANSIENT_SECRET", false))),
        "project", "session", "idle", listOf(RemoteTurn("turn", "assistant", "durable display")),
        "42", false, "7", workspaceOwner = "c".repeat(64), workspaceRevision = "4",
        subtabs = RemoteSubtabs("project", "9", tab, listOf(RemoteSubtab(tab, "main", "main", 3)), true))
    @Test fun sameIdentityAtNewLanAddressKeepsDisplayAndCursorWithoutLiveAuthority() {
        val saved = SavedHostState(host, projection)
        val moved = host.copy(host = "192.168.2.25", port = 8888)
        val next = RePairPolicy.state(saved, moved)
        assertEquals(moved, next.host)
        val display = requireNotNull(next.projection)
        assertEquals(projection.turns, display.turns)
        assertEquals("42", display.journalSequence); assertEquals("7", display.eventSequence)
        assertEquals("9", display.subtabs?.revision); assertEquals(tab, display.subtabs?.activeId)
        assertFalse(requireNotNull(display.subtabs).control)
        assertFalse(display.projects.single().workspaceControl); assertFalse(display.projects.single().promptControl)
        assertFalse(display.projects.single().diffAvailable)
        assertNull(display.workspaceOwner); assertNull(display.workspaceRevision)
        assertNull(display.projects.single().terminal); assertNull(display.projects.single().desktopStreamText)
        assertNull(display.projects.single().promptStatuses.single().streamText)
        assertEquals("unknown_outcome", display.projects.single().promptStatuses.single().state)
        assertTrue(runCatching { WorkspaceCommand.prompt(display, "hello") }.isFailure)
        assertEquals(projection, saved.projection)
    }
    @Test fun differentHostPinOrDeviceCannotInheritAnotherIdentityDisplay() {
        for (other in listOf(host.copy(fingerprint = "e".repeat(64)), host.copy(deviceId = "f".repeat(64)))) {
            assertNull(RePairPolicy.state(SavedHostState(host, projection), other).projection)
        }
    }
    @Test fun unresolvedWritesAndRevocationNeverBecomeAddressUpdates() {
        val command = WorkspaceCommand.prompt(projection, "hello")
        for (saved in listOf(SavedHostState(host, projection, command), SavedHostState(host, projection, pendingRevoke = "revoke"))) {
            for (next in listOf(host, host.copy(port = 8888), host.copy(fingerprint = "e".repeat(64)))) {
                assertTrue(runCatching { RePairPolicy.state(saved, next) }.isFailure)
            }
        }
    }
    @Test fun initialPairingAndEmptyAclHaveNoInventedDisplayOrAuthority() {
        assertEquals(SavedHostState(host, null), RePairPolicy.state(null, host))
        assertEquals(SavedHostState(host, null), RePairPolicy.state(SavedHostState(host, null), host))
        val empty = HostProjection(emptyList(), null, null, null, emptyList(), null, false)
        assertNull(RePairPolicy.state(SavedHostState(host, empty), host).projection)
    }
}
