package zenpi

import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.ConnectionState
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.security.PairedHost
import zenpi.transport.HostProjection
import zenpi.transport.RemoteProject
import zenpi.transport.RemotePromptStatus
import zenpi.transport.RemoteSubtab
import zenpi.transport.RemoteSubtabs
import zenpi.ui.DraftKey
import zenpi.ui.DraftPolicy
import zenpi.ui.InviteTicker

class ScreenKeysTest {
    private val main = "a".repeat(64)
    private val other = "c".repeat(64)
    private val project = RemoteProject("p".repeat(64), "Project", "s".repeat(64), workspaceControl = true, promptControl = true)
    private val host = PairedHost("192.0.2.1", 7000, "f".repeat(64), "d".repeat(64))
    private fun tabs(revision: String, active: String = main, name: String = "other", concurrency: Int = 1) =
        RemoteSubtabs(project.id, revision, active, listOf(RemoteSubtab(main, "main", "main", 1), RemoteSubtab(other, name, "in_place", concurrency)), true)
    private fun state(subtabs: RemoteSubtabs, statuses: List<RemotePromptStatus> = emptyList()) = PairingState(
        phase = PairingPhase.CONNECTED, host = host,
        projection = HostProjection(listOf(project.copy(promptStatuses = statuses)), project.id, project.sessionId, "idle", emptyList(), "1", false,
            subtabs = subtabs))

    @Test fun draftKeyIgnoresSubtabRevisionRenameConcurrencyAndRunState() {
        val base = DraftKey.of(state(tabs("2")))
        assertEquals(base, DraftKey.of(state(tabs("3", name = "renamed"))))
        assertEquals(base, DraftKey.of(state(tabs("4", concurrency = 5))))
        val running = state(tabs("5"), listOf(RemotePromptStatus("r".repeat(32), project.id, project.sessionId, "running")))
        assertEquals(base, DraftKey.of(running))
        assertEquals(base, DraftKey.of(state(tabs("6"), listOf(RemotePromptStatus("r".repeat(32), project.id, project.sessionId, "succeeded")))))
        assertEquals(DraftKey(host.fingerprint to host.deviceId, project.id, project.sessionId, main), base)
    }

    @Test fun draftKeyStillSeparatesHostsSessionsAndTabs() {
        val base = DraftKey.of(state(tabs("2")))
        assertNotEquals(base, DraftKey.of(state(tabs("2", active = other))))
        assertNotEquals(base, DraftKey.of(state(tabs("2")).copy(host = host.copy(deviceId = "e".repeat(64)))))
        val otherSession = state(tabs("2")).let { it.copy(projection = it.projection!!.copy(sessionId = "t".repeat(64))) }
        assertNotEquals(base, DraftKey.of(otherSession))
    }

    @Test fun steeringDraftFollowsTheSteeredRunButNotRevisions() {
        fun run(id: String, revision: String) = state(tabs(revision), listOf(RemotePromptStatus(id, project.id, project.sessionId, "running")))
        assertEquals(DraftPolicy.steerKey(run("first", "2")), DraftPolicy.steerKey(run("first", "3")))
        assertNotEquals(DraftPolicy.steerKey(run("first", "2")), DraftPolicy.steerKey(run("second", "2")))
        assertNotEquals(DraftPolicy.steerKey(run("first", "2")), DraftPolicy.steerKey(state(tabs("2"))))
    }

    @Test fun unsubmittedDraftSurvivesMarkerChangesAndSubmittedDraftClearsOnAcknowledgement() {
        val idle = DraftPolicy.promptMarker(state(tabs("2")))
        val renamed = DraftPolicy.promptMarker(state(tabs("3", name = "renamed")))
        val running = DraftPolicy.promptMarker(state(tabs("3"), listOf(RemotePromptStatus("r".repeat(32), project.id, project.sessionId, "accepted"))))
        assertNotEquals(idle, renamed); assertNotEquals(renamed, running)
        // Typing while a remote rename or a run start/finish lands keeps the text.
        assertEquals("half typed", DraftPolicy.settle("half typed", null, idle, renamed))
        assertEquals("half typed", DraftPolicy.settle("half typed", null, running, renamed))
        // Text edited after sending is kept too.
        assertEquals("edited", DraftPolicy.settle("edited", "sent", idle, running))
        // The submitted message clears once the host acknowledges it, never before.
        assertEquals("", DraftPolicy.settle("sent", "sent", idle, running))
        assertEquals("sent", DraftPolicy.settle("sent", "sent", idle, idle))
        assertEquals(DraftPolicy.workerMarker(state(tabs("2"))), "2")
    }

    @Test fun inviteTickerPublishesOnceAndStopsWithoutInvite() = runBlocking {
        assertFalse(InviteTicker.active(null, 0))
        assertFalse(InviteTicker.active(PairingState().invite, System.currentTimeMillis() / 1000))
        val published = mutableListOf<Long>(); var sleeps = 0
        InviteTicker.run(active = { InviteTicker.active(null, it) }, clock = { 100L }, sleep = { sleeps++ }, publish = { published += it })
        assertEquals(listOf(100L), published)
        assertEquals(0, sleeps)
    }

    @Test fun inviteTickerTicksEverySecondOnlyUntilExpiry() = runBlocking {
        var now = 100L
        val published = mutableListOf<Long>(); val sleeps = mutableListOf<Long>()
        InviteTicker.run(active = { it < 103 }, clock = { now }, sleep = { sleeps += it; now += it / 1000 }, publish = { published += it })
        assertEquals(listOf(100L, 101L, 102L, 103L), published)
        assertEquals(listOf(1000L, 1000L, 1000L), sleeps)
    }

    @Test fun pairingStateCopyKeepsUntouchedSubStatesAndStructuralEquality() {
        val base = state(tabs("2"))
        val next = base.copy(diffOpen = true)
        assertSame(base.connection, next.connection)
        assertSame(base.workspace, next.workspace)
        assertNotSame(base.diffView, next.diffView)
        assertTrue(next.diffView.open && next.diffOpen)
        assertEquals(base, next.copy(diffOpen = false))
        assertEquals(base.hashCode(), next.copy(diffOpen = false).hashCode())
        assertEquals(ConnectionState(phase = PairingPhase.CONNECTED, host = host), base.connection)
        assertEquals(PairingState(), PairingState(phase = PairingPhase.OFFLINE))
        assertNotEquals(base, base.copy(sessionsQuery = "x"))
    }
}
