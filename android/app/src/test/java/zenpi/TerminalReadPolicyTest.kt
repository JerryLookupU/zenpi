package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class TerminalReadPolicyTest {
    private val tab = "b".repeat(64)
    private val terminal = RemoteTerminal("project", "session", tab, "1", "screen", false, "c".repeat(64))
    private val project = RemoteProject("project", "Project", "session", terminal = terminal)
    private val tabs = RemoteSubtabs("project", "1", tab, listOf(RemoteSubtab(tab, "Main", "main", 1)), false)
    private val projection = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false, subtabs = tabs)
    private val page = TerminalPage("project", "session", requireNotNull(terminal.terminalId), tab, "1", "d".repeat(64), 0, 3, 6, true, "abc")
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, terminalOpen = true, terminalPage = page)

    @Test fun readOnlyAndExitedTerminalRemainReadableWhileFailureCanRefresh() {
        assertFalse(project.terminalControl)
        assertTrue(TerminalReadPolicy.browse(state)); assertTrue(TerminalReadPolicy.navigation(state))
        val exited = state.copy(projection = projection.copy(projects = listOf(project.copy(terminal = terminal.copy(running = false)))))
        assertTrue(TerminalReadPolicy.navigation(exited))
        val failed = state.copy(terminalPage = null, terminalError = "unavailable")
        assertTrue(TerminalReadPolicy.browse(failed)); assertFalse(TerminalReadPolicy.navigation(failed))
    }

    @Test fun lifecycleAndAmbiguousScopeFenceEveryRead() {
        val pending = WorkspaceCommand.prompt(projection.copy(projects = listOf(project.copy(promptControl = true)),
            workspaceOwner = "a".repeat(64), workspaceRevision = "1"), "pending")
        for (changed in listOf(state.copy(phase = PairingPhase.OFFLINE), state.copy(scanInProgress = true), state.copy(resetting = true),
            state.copy(resetRequired = true), state.copy(terminalLoading = true), state.copy(pendingRevoke = "pending"), state.copy(pendingWorkspace = pending),
            state.copy(projection = null), state.copy(projection = projection.copy(sessionId = "foreign")),
            state.copy(projection = projection.copy(projects = listOf(project, project))),
            state.copy(projection = projection.copy(projects = listOf(project, project.copy(sessionId = "foreign")))),
            state.copy(projection = projection.copy(projects = listOf(project.copy(terminal = null)))),
            state.copy(projection = projection.copy(subtabs = null)), state.copy(projection = projection.copy(subtabs = tabs.copy(projectId = "foreign"))),
            state.copy(projection = projection.copy(subtabs = tabs.copy(tabs = emptyList()))),
            state.copy(projection = projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs))))) {
            assertFalse(TerminalReadPolicy.browse(changed)); assertFalse(TerminalReadPolicy.navigation(changed))
        }
        val duplicate = projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs))
        assertTrue(runCatching { TerminalPageCodec.request(duplicate, null) }.isFailure)
    }

    @Test fun staleMalformedOrClosedHistoryCannotNavigateButFreshReadRemainsAvailable() {
        for (changed in listOf(state.copy(terminalOpen = false), state.copy(terminalPage = page.copy(terminalId = "e".repeat(64))),
            state.copy(terminalPage = page.copy(subtabRevision = "2")), state.copy(terminalPage = page.copy(nextOffset = 4)),
            state.copy(terminalPrevious = listOf(page)))) {
            assertTrue(TerminalReadPolicy.browse(changed)); assertFalse(TerminalReadPolicy.navigation(changed))
        }
    }
}
