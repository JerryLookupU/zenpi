package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class SessionHistoryReadPolicyTest {
    private val project = RemoteProject("project", "Project", "session")
    private val projection = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "80", false)
    private val page = SessionHistoryPage("project", "session", "80", "d".repeat(64), 0, 1, 2, true,
        listOf(RemoteTurn("old", "user", "text")))
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, sessionHistoryOpen = true, sessionHistory = page)

    @Test fun readOnlyAndRunningProjectsBrowseAndFailedPagesCanRefresh() {
        assertFalse(project.promptControl)
        assertTrue(SessionHistoryReadPolicy.browse(state)); assertTrue(SessionHistoryReadPolicy.navigation(state))
        val running = state.copy(projection = projection.copy(projects = listOf(project.copy(
            promptStatuses = listOf(RemotePromptStatus("request", "project", "session", "running"))))))
        assertTrue(SessionHistoryReadPolicy.browse(running)); assertTrue(SessionHistoryReadPolicy.navigation(running))
        val failed = state.copy(sessionHistory = null, sessionHistoryError = "changed")
        assertTrue(SessionHistoryReadPolicy.browse(failed)); assertFalse(SessionHistoryReadPolicy.navigation(failed))
    }

    @Test fun lifecycleScopeAndJournalBoundsFenceInitialReadBeforeEncoding() {
        val pending = WorkspaceCommand.prompt(projection.copy(projects = listOf(project.copy(promptControl = true)),
            workspaceOwner = "a".repeat(64), workspaceRevision = "1"), "pending")
        for (changed in listOf(state.copy(phase = PairingPhase.OFFLINE), state.copy(scanInProgress = true), state.copy(resetting = true),
            state.copy(resetRequired = true), state.copy(pendingWorkspace = pending), state.copy(pendingRevoke = "pending"),
            state.copy(sessionHistoryLoading = true), state.copy(transcriptTurn = "old"), state.copy(projection = null))) {
            assertFalse(SessionHistoryReadPolicy.browse(changed)); assertFalse(SessionHistoryReadPolicy.navigation(changed))
        }
        for (foreign in listOf(projection.copy(projects = emptyList()), projection.copy(projects = listOf(project, project)),
            projection.copy(sessionId = "foreign"), projection.copy(journalSequence = null), projection.copy(journalSequence = "-1"),
            projection.copy(journalSequence = "01"), projection.copy(journalSequence = "18446744073709551616"))) {
            assertFalse(SessionHistoryReadPolicy.browse(state.copy(projection = foreign)))
            assertTrue(runCatching { SessionHistoryCodec.request(foreign, null) }.isFailure)
        }
        for (revision in listOf("0", "18446744073709551615")) {
            assertTrue(SessionHistoryCodec.readable(projection.copy(journalSequence = revision)))
            SessionHistoryCodec.request(projection.copy(journalSequence = revision), null)
        }
    }

    @Test fun closedStaleOrBrokenPagesCannotNavigateButFreshScopeRemainsReadable() {
        for (changed in listOf(state.copy(sessionHistoryOpen = false), state.copy(sessionHistory = page.copy(sessionId = "foreign")),
            state.copy(sessionHistory = page.copy(journalRevision = "79")), state.copy(sessionHistoryPrevious = listOf(page)),
            state.copy(sessionHistory = page.copy(nextOffset = 2)))) {
            assertTrue(SessionHistoryReadPolicy.browse(changed)); assertFalse(SessionHistoryReadPolicy.navigation(changed))
        }
    }
}
