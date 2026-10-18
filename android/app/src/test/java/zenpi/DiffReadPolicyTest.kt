package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class DiffReadPolicyTest {
    private val project = RemoteProject("project", "Project", "session", diffAvailable = true)
    private val tab = "b".repeat(64)
    private val tabs = RemoteSubtabs("project", "7", tab, listOf(RemoteSubtab(tab, "Main", "main", 1)), false)
    private val projection = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false, subtabs = tabs)
    private val diff = RemoteDiff("project", "session", tab, "7", ".", false, false, false, "", "")
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, diff = diff, diffOpen = true)

    @Test fun readOnlyAndRunningTasksCanReviewWhileFailedReadsCanRefresh() {
        assertTrue(DiffReadPolicy.browse(state)); assertTrue(DiffReadPolicy.navigation(state))
        val running = state.copy(projection = projection.copy(projects = listOf(project.copy(promptControl = true,
            promptStatuses = listOf(RemotePromptStatus("request", "project", "session", "running"))))))
        assertTrue(DiffReadPolicy.browse(running)); assertTrue(DiffReadPolicy.navigation(running))
        val failed = state.copy(diff = null, diffError = "unavailable")
        assertTrue(DiffReadPolicy.browse(failed)); assertFalse(DiffReadPolicy.navigation(failed))
    }

    @Test fun lifecycleScopeCapabilityAndUnknownWritesFenceEveryRead() {
        val pending = WorkspaceCommand.prompt(projection.copy(projects = listOf(project.copy(promptControl = true)),
            workspaceOwner = "a".repeat(64), workspaceRevision = "1"), "pending")
        for (changed in listOf(state.copy(phase = PairingPhase.OFFLINE), state.copy(scanInProgress = true), state.copy(resetting = true),
            state.copy(resetRequired = true), state.copy(diffLoading = true), state.copy(pendingRevoke = "pending"),
            state.copy(pendingWorkspace = pending), state.copy(projection = null),
            state.copy(projection = projection.copy(projects = emptyList())), state.copy(projection = projection.copy(sessionId = "foreign")),
            state.copy(projection = projection.copy(projects = listOf(project, project))),
            state.copy(projection = projection.copy(projects = listOf(project, project.copy(sessionId = "foreign")))),
            state.copy(projection = projection.copy(projects = listOf(project, project.copy(diffAvailable = false)))),
            state.copy(projection = projection.copy(projects = listOf(project.copy(diffAvailable = false)))),
            state.copy(projection = projection.copy(subtabs = null)), state.copy(projection = projection.copy(subtabs = tabs.copy(projectId = "foreign"))),
            state.copy(projection = projection.copy(subtabs = tabs.copy(activeId = "missing"))),
            state.copy(projection = projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs))))) {
            assertFalse(DiffReadPolicy.browse(changed)); assertFalse(DiffReadPolicy.navigation(changed))
        }
    }

    @Test fun stalePagesCannotNavigateButMayStartFreshReviewAndAmbiguousTabsNeverEncode() {
        for (changed in listOf(diff.copy(projectId = "foreign"), diff.copy(sessionId = "foreign"),
            diff.copy(subtabId = "foreign"), diff.copy(subtabRevision = "8"))) {
            assertTrue(DiffReadPolicy.browse(state.copy(diff = changed)))
            assertFalse(DiffReadPolicy.navigation(state.copy(diff = changed)))
        }
        val ambiguous = projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs))
        assertFalse(DiffProjectionCodec.sameSelection(diff, ambiguous))
        assertTrue(runCatching { DiffProjectionCodec.request(ambiguous, ".") }.isFailure)
        val mixed = projection.copy(projects = listOf(project, project.copy(diffAvailable = false)))
        assertFalse(DiffProjectionCodec.sameSelection(diff, mixed))
        assertTrue(runCatching { DiffProjectionCodec.request(mixed, ".") }.isFailure)
    }
}
