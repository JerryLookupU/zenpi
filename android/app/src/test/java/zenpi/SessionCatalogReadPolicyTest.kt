package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class SessionCatalogReadPolicyTest {
    private val project = RemoteProject("project", "Project", "session")
    private val projection = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false)
    private val catalog = SessionCatalog("project", "session", "d".repeat(64), 0, 16,
        (0 until 16).map { RemoteSession("history-$it", "1", 1, false) })
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, sessionsOpen = true, sessions = catalog)

    @Test fun readOnlyAndRunningProjectsBrowseWithoutResumeAuthorityAndErrorsCanRefresh() {
        assertFalse(project.sessionControl)
        assertTrue(SessionCatalogReadPolicy.browse(state)); assertTrue(SessionCatalogReadPolicy.navigation(state))
        val running = state.copy(projection = projection.copy(projects = listOf(project.copy(
            promptStatuses = listOf(RemotePromptStatus("request", "project", "session", "running"))))))
        assertTrue(SessionCatalogReadPolicy.navigation(running))
        val failed = state.copy(sessions = null, sessionsError = "unavailable")
        assertTrue(SessionCatalogReadPolicy.browse(failed)); assertFalse(SessionCatalogReadPolicy.navigation(failed))
    }

    @Test fun lifecycleScopeAndPendingWritesFenceSearchAndCatalogReads() {
        val pending = WorkspaceCommand.prompt(projection.copy(projects = listOf(project.copy(promptControl = true)),
            workspaceOwner = "a".repeat(64), workspaceRevision = "1"), "pending")
        for (changed in listOf(state.copy(phase = PairingPhase.OFFLINE), state.copy(scanInProgress = true), state.copy(resetting = true),
            state.copy(resetRequired = true), state.copy(sessionsLoading = true), state.copy(pendingRevoke = "pending"),
            state.copy(pendingWorkspace = pending), state.copy(projection = null), state.copy(projection = projection.copy(projects = emptyList())),
            state.copy(projection = projection.copy(sessionId = "foreign")), state.copy(projection = projection.copy(projects = listOf(project, project))))) {
            assertFalse(SessionCatalogReadPolicy.browse(changed)); assertFalse(SessionCatalogReadPolicy.navigation(changed))
        }
        for (foreign in listOf(projection.copy(projects = emptyList()), projection.copy(projects = listOf(project, project)),
            projection.copy(sessionId = "foreign"))) {
            assertTrue(runCatching { SessionCatalogCodec.request(foreign, null, 0) }.isFailure)
        }
    }

    @Test fun foreignClosedOrDifferentQueryPagesCannotNavigateButFreshSearchRemainsAvailable() {
        for (changed in listOf(state.copy(sessionsOpen = false), state.copy(sessions = catalog.copy(projectId = "foreign")),
            state.copy(sessions = catalog.copy(sessionId = "foreign")), state.copy(sessionsQuery = "new"),
            state.copy(sessions = catalog.copy(query = "old")))) {
            assertTrue(SessionCatalogReadPolicy.browse(changed)); assertFalse(SessionCatalogReadPolicy.navigation(changed))
        }
        assertTrue(SessionCatalogReadPolicy.navigation(state.copy(sessionsQuery = "match", sessions = catalog.copy(query = "match"))))
    }
}
