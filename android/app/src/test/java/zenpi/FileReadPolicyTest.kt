package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class FileReadPolicyTest {
    private val tab = "b".repeat(64)
    private val project = RemoteProject("project", "Project", "session")
    private val tabs = RemoteSubtabs("project", "1", tab, listOf(RemoteSubtab(tab, "Main", "main", 1)), false)
    private val projection = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false, "0", subtabs = tabs)
    private val files = WorkspaceFiles("project", "session", tab, "1", ".", emptyList(), false)
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, files = files)

    @Test fun readOnlyAndRunningProjectCanNavigateWithoutAttachmentAuthority() {
        assertTrue(FileReadPolicy.browse(state)); assertTrue(FileReadPolicy.navigation(state)); assertFalse(FileReadPolicy.attachment(state))
        val control = state.copy(projection = projection.copy(projects = listOf(project.copy(promptControl = true))))
        assertTrue(FileReadPolicy.attachment(control))
        val running = control.copy(projection = projection.copy(projects = listOf(project.copy(promptControl = true,
            promptStatuses = listOf(RemotePromptStatus("request", "project", "session", "running"))))))
        assertTrue(FileReadPolicy.browse(running)); assertTrue(FileReadPolicy.navigation(running)); assertFalse(FileReadPolicy.attachment(running))
    }
    @Test fun scopeAndLifecycleChangesFenceEveryFileActionWhileFailedReadsCanRefresh() {
        for (changed in listOf(state.copy(phase = PairingPhase.OFFLINE), state.copy(scanInProgress = true),
            state.copy(resetting = true), state.copy(resetRequired = true), state.copy(filesLoading = true), state.copy(pendingRevoke = "revoke"),
            state.copy(projection = null), state.copy(projection = projection.copy(projects = emptyList())),
            state.copy(projection = projection.copy(projects = listOf(project, project))), state.copy(projection = projection.copy(sessionId = "other")),
            state.copy(projection = projection.copy(projects = listOf(project, project.copy(sessionId = "foreign")))),
            state.copy(projection = projection.copy(subtabs = null)), state.copy(projection = projection.copy(subtabs = tabs.copy(projectId = "other"))),
            state.copy(projection = projection.copy(subtabs = tabs.copy(activeId = "c".repeat(64)))),
            state.copy(projection = projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs))))) {
            assertFalse(FileReadPolicy.browse(changed)); assertFalse(FileReadPolicy.navigation(changed)); assertFalse(FileReadPolicy.attachment(changed))
        }
        val failed = state.copy(files = null, filesError = "unavailable")
        assertTrue(FileReadPolicy.browse(failed)); assertFalse(FileReadPolicy.navigation(failed))
    }
    @Test fun fileWireRequestsRefuseUnknownProjectsAndNonexistentOrDuplicateActiveTabs() {
        for (changed in listOf(projection.copy(projects = emptyList()), projection.copy(sessionId = "foreign"),
            projection.copy(projects = listOf(project, project.copy(sessionId = "foreign"))),
            projection.copy(subtabs = tabs.copy(activeId = "c".repeat(64))),
            projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs)))) {
            assertFalse(WorkspaceFilesCodec.readable(changed))
            assertFalse(WorkspaceFilesCodec.matchesSelection(files, changed))
            assertTrue(runCatching { WorkspaceFilesCodec.request(changed, ".") }.isFailure)
        }
    }
    @Test fun staleMetadataAndUnknownWritesNeverAuthorizeNavigationOrAttachment() {
        for (changed in listOf(files.copy(projectId = "other"), files.copy(sessionId = "other"),
            files.copy(subtabId = "c".repeat(64)), files.copy(subtabRevision = "2"))) {
            assertTrue(FileReadPolicy.browse(state.copy(files = changed))); assertFalse(FileReadPolicy.navigation(state.copy(files = changed)))
        }
        val command = WorkspaceCommand.prompt(projection.copy(projects = listOf(project.copy(promptControl = true)),
            workspaceOwner = "d".repeat(64), workspaceRevision = "1"), "pending")
        assertFalse(FileReadPolicy.browse(state.copy(pendingWorkspace = command)))
        assertFalse(FileReadPolicy.attachment(state.copy(pendingWorkspace = command)))
    }
}
