package zenpi

import zenpi.ui.PairingScreen
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.security.PairedHost
import zenpi.transport.*

class SessionScopeScreenTest {
    @get:Rule val compose = createComposeRule()
    private val host = PairedHost("192.168.1.2", 7777, "a".repeat(64), "b".repeat(64))
    private val project = RemoteProject("project", "Workspace", "old", promptControl = true)
    private val projection = HostProjection(listOf(project), "project", "old", "idle", emptyList(), "1", false, "0")

    @Test fun messageDraftSurvivesSameSessionUpdatesButNotSessionOrHostReplacement() {
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, host = host, projection = projection))
        val sent = mutableListOf<String>()
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onPrompt = { sent += it }) } }
        compose.onNodeWithText("Message").performScrollTo().performTextInput("old draft")
        compose.runOnIdle { state.value = state.value.copy(projection = projection.copy(eventSequence = "1")) }
        compose.onNode(hasSetTextAction()).assertTextContains("old draft")
        compose.runOnIdle { state.value = state.value.copy(projection = projection.copy(sessionId = "new", projects = listOf(project.copy(sessionId = "new")))) }
        compose.onNode(hasSetTextAction() and hasText("old draft", substring = true)).assertDoesNotExist()
        compose.onNodeWithContentDescription("Send message").performScrollTo().assertIsNotEnabled()
        compose.onNodeWithText("Message").performTextInput("new draft")
        compose.runOnIdle { state.value = state.value.copy(host = host.copy(fingerprint = "c".repeat(64))) }
        compose.onNodeWithContentDescription("Send message").performScrollTo().assertIsNotEnabled()
        val tab = "d".repeat(64)
        val scoped = projection.copy(subtabs = RemoteSubtabs("project", "1", tab,
            listOf(RemoteSubtab(tab, "Main", "main", 1)), false))
        compose.runOnIdle { state.value = state.value.copy(projection = scoped) }
        compose.onNodeWithText("Message").performTextInput("use @\"old-root.txt\"")
        compose.runOnIdle { state.value = state.value.copy(projection = scoped.copy(eventSequence = "2")) }
        compose.onNode(hasSetTextAction()).assertTextContains("old-root.txt", substring = true)
        compose.runOnIdle { state.value = state.value.copy(projection = scoped.copy(subtabs = scoped.subtabs?.copy(revision = "2"))) }
        // ZE-518: drafts are keyed by host, session and tab ids only; a remote revision keeps them.
        compose.onNode(hasSetTextAction()).assertTextContains("old-root.txt", substring = true)
        compose.onNodeWithText("Message").performTextReplacement("new message draft")
        val running = scoped.copy(projects = listOf(project.copy(promptStatuses = listOf(
            RemotePromptStatus("first", "project", "old", "running")))))
        compose.runOnIdle { state.value = state.value.copy(projection = running) }
        // ZE-530: the empty steering draft turns the composer button into Stop.
        compose.onNodeWithContentDescription("Send steering input").assertDoesNotExist()
        compose.onNodeWithContentDescription("Stop message").assertExists()
        compose.onNodeWithText("Steer").performTextInput("first task steering")
        compose.runOnIdle { state.value = state.value.copy(projection = running.copy(eventSequence = "3")) }
        compose.onNode(hasSetTextAction()).assertTextContains("first task steering")
        compose.runOnIdle { state.value = state.value.copy(projection = running.copy(projects = listOf(project.copy(promptStatuses = listOf(
            RemotePromptStatus("second", "project", "old", "running")))))) }
        // ZE-530: the empty steering draft turns the composer button into Stop.
        compose.onNodeWithContentDescription("Send steering input").assertDoesNotExist()
        compose.onNodeWithContentDescription("Stop message").assertExists()
        compose.onNodeWithText("Steer").performTextInput("second task steering")
        compose.runOnIdle { state.value = state.value.copy(projection = scoped) }
        // The run finished: its steering text is dropped and the unsent message draft is back.
        compose.onNode(hasSetTextAction()).assertTextContains("new message draft")
        compose.onNode(hasSetTextAction() and hasText("second task steering", substring = true)).assertDoesNotExist()
        compose.runOnIdle { assertEquals(emptyList<String>(), sent) }
    }

    @Test fun approvalSheetCannotResurrectWhenReturningToAnOldSession() {
        val approval = RemoteApproval("approval", "project", "old", "turn", "call", "write_file", "workspace_write", "agent_tool", "d".repeat(64))
        val original = projection.copy(projects = listOf(project.copy(pendingApprovals = listOf(approval), approvalControl = true)))
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, host = host, projection = original))
        var decisions = 0
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onApprove = { _, _ -> decisions++ }) } }
        compose.onNodeWithContentDescription("Review approvals").performClick()
        compose.onNodeWithText("Allow once").assertIsEnabled()
        compose.runOnIdle { state.value = state.value.copy(projection = projection.copy(sessionId = "new", projects = listOf(project.copy(sessionId = "new")))) }
        compose.onNodeWithText("Allow once").assertDoesNotExist()
        compose.runOnIdle { state.value = state.value.copy(projection = original) }
        compose.onNodeWithText("Allow once").assertDoesNotExist()
        compose.runOnIdle { assertEquals(0, decisions) }
    }

    @Test fun reusedTerminalIdentityCannotCarryInputAcrossSessionReplacement() {
        val tab = "e".repeat(64)
        val terminal = RemoteTerminal("project", "old", tab, "1", "screen", false, "f".repeat(64))
        val original = projection.copy(projects = listOf(project.copy(terminal = terminal, terminalControl = true)),
            subtabs = RemoteSubtabs("project", "1", tab, listOf(RemoteSubtab(tab, "Main", "main", 1)), true))
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, host = host, projection = original))
        var writes = 0
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onTerminalInput = { writes++ }) } }
        compose.onNodeWithTag("terminal-input").performScrollTo().performTextInput("old command")
        compose.runOnIdle { state.value = state.value.copy(projection = original.copy(eventSequence = "1")) }
        compose.onNodeWithTag("terminal-input").assertTextContains("old command")
        compose.runOnIdle { state.value = state.value.copy(projection = original.copy(sessionId = "new",
            projects = listOf(project.copy(sessionId = "new", terminal = terminal.copy(sessionId = "new"), terminalControl = true)))) }
        compose.onNodeWithContentDescription("Send terminal input").performScrollTo().assertIsNotEnabled()
        compose.runOnIdle { assertEquals(0, writes) }
    }

    @Test fun worktreeRemovalIsFencedByRevokeAndDismissedBySessionReplacement() {
        val main = "e".repeat(64); val tab = "f".repeat(64)
        val original = projection.copy(subtabs = RemoteSubtabs("project", "1", tab,
            listOf(RemoteSubtab(main, "Main", "main", 1), RemoteSubtab(tab, "Worktree", "worktree", 1)), true, worktreeControl = true))
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, host = host, projection = original))
        var writes = 0
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onSubtab = { _, _, _ -> writes++ }) } }
        compose.onNodeWithContentDescription("Remove Git worktree").performScrollTo().performClick()
        compose.onNodeWithText("Remove").assertIsEnabled()
        compose.runOnIdle { state.value = state.value.copy(pendingRevoke = "unconfirmed") }
        compose.onNodeWithText("Remove").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(pendingRevoke = null, projection = original.copy(sessionId = "new")) }
        compose.onNodeWithText("Remove Git worktree?").assertDoesNotExist()
        compose.runOnIdle { state.value = state.value.copy(projection = original) }
        compose.onNodeWithText("Remove Git worktree?").assertDoesNotExist()
        compose.runOnIdle { assertEquals(0, writes) }
    }
}
