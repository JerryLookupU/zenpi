package zenpi

import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.SemanticsNodeInteractionCollection
import androidx.compose.ui.test.isRoot
import androidx.compose.ui.test.junit4.accessibility.enableAccessibilityChecks
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.tryPerformAccessibilityChecks
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.security.PairedHost
import zenpi.transport.HostProjection
import zenpi.transport.RemoteApproval
import zenpi.transport.RemoteApprovalPreview
import zenpi.transport.RemoteDiff
import zenpi.transport.RemoteModelCatalog
import zenpi.transport.RemoteModelChoice
import zenpi.transport.RemoteModelStatus
import zenpi.transport.RemoteProject
import zenpi.transport.RemotePromptStatus
import zenpi.transport.RemoteResource
import zenpi.transport.RemoteSession
import zenpi.transport.RemoteSubtab
import zenpi.transport.RemoteSubtabs
import zenpi.transport.RemoteTerminal
import zenpi.transport.RemoteToolCall
import zenpi.transport.RemoteTurn
import zenpi.transport.RemoteWorkerStatus
import zenpi.transport.ResourceCatalog
import zenpi.transport.SessionCatalog
import zenpi.transport.TerminalSize
import zenpi.transport.WorkspaceFile
import zenpi.transport.WorkspaceFiles
import zenpi.ui.AppShell
import zenpi.ui.PairingActions
import zenpi.ui.theme.ZenpiTheme

/**
 * ZE-535 validator: the Accessibility Test Framework (Compose `enableAccessibilityChecks`) over every
 * screen of the portrait IA, in Zenpi's light and dark palettes and with the device's dynamic colour.
 * Each click runs the checks on the whole hierarchy; each screen is also checked once it is shown.
 */
class AccessibilityChecksTest {
    @get:Rule val compose = createComposeRule()

    private val main = "a".repeat(64)
    private val tab = RemoteSubtabs("zenpi", "2", main, listOf(RemoteSubtab(main, "main", "main", 1), RemoteSubtab("b".repeat(64), "wt-a", "worktree", 2)),
        true, worktreeControl = true, concurrencyControl = true)
    private val approvals = listOf(
        RemoteApproval("0b7e4c2a-6f1d-4d8e-9a35-2c4f7e1b9d60", "zenpi", "s-zenpi", "turn-2", "call-2", "edit_file", "workspace_write", "agent_tool",
            "c".repeat(64), RemoteApprovalPreview("src/remote/subscription.rs", "@@ -41,3 +41,4 @@\n-    let a = 1;\n+    let a = 2;\n", changed = true,
                truncated = false, redacted = false, withheld = false)),
        RemoteApproval("5d2a9f81-3c6e-4b07-8e14-a7f0c3d29b56", "zenpi", "s-zenpi", "turn-2", "call-3", "bash", "command_execution", "agent_tool", "d".repeat(64)))
    private val project = RemoteProject("zenpi", "zenpi", "s-zenpi", workspaceControl = true, promptControl = true, approvalControl = true,
        pendingApprovals = approvals, sessionName = "Fix the flaky test", modelControl = true, reasoningControl = true, workerControl = true,
        terminalControl = true, diffAvailable = true,
        promptStatuses = listOf(RemotePromptStatus("4e9b7d0c-1f38-4a65-b2d7-9c0e6a3f8b54", "zenpi", "s-zenpi", "running", streamText = "Found it")),
        workerStatuses = listOf(RemoteWorkerStatus("6b3f9e2d-8c04-4a71-9d5e-0f7a2c6b4e93", "zenpi", "s-zenpi", main, "succeeded", outputText = "All tests pass.")),
        terminal = RemoteTerminal("zenpi", "s-zenpi", main, "2", "$ cargo test\nok\n$ ", false, "c".repeat(64), TerminalSize(24, 80)))
    private val connected = PairingState(phase = PairingPhase.CONNECTED, host = PairedHost("192.168.1.20", 7443, "e".repeat(64), "d".repeat(64)),
        registered = true, projection = HostProjection(listOf(project, RemoteProject("docs", "docs", "s-docs", workspaceControl = true)),
            "zenpi", "s-zenpi", "running", listOf(RemoteTurn("turn-1", "user", "fix the flaky test"),
                RemoteTurn("turn-2", "assistant", "The race is in `reconnect`.", toolCalls = listOf(RemoteToolCall("call-1", "bash", "{\"command\":\"cargo test\"}", false)))),
            "4", false, "9", workspaceOwner = "f".repeat(64), workspaceRevision = "3", subtabs = tab,
            modelStatus = RemoteModelStatus("zenpi", "s-zenpi", "gpt-x", "high", true, "7", mapOf("tools" to true), "128000", "8192",
                listOf(null, "low", "high"), RemoteModelCatalog("a".repeat(64), 0, null, listOf(RemoteModelChoice("gpt-x", "active", true))))))

    private fun SemanticsNodeInteractionCollection.check() = repeat(fetchSemanticsNodes().size) { get(it).tryPerformAccessibilityChecks() }

    private fun walk(dark: Boolean?) {
        val state = mutableStateOf(connected)
        compose.setContent {
            if (dark == null) ZenpiTheme { AppShell(state.value, object : PairingActions {}) }
            else ZenpiTheme(dark = dark, dynamic = false) { AppShell(state.value, object : PairingActions {}) }
        }
        compose.enableAccessibilityChecks()
        val screens = listOf<() -> Unit>(
            {},
            { compose.onNodeWithContentDescription("Review approvals").performClick() },
            { state.value = connected.copy(projection = connected.projection!!.copy(projects = connected.projection!!.projects.map { it.copy(pendingApprovals = emptyList()) })) },
            { state.value = state.value.copy(sessionsOpen = true, sessions = SessionCatalog("zenpi", "s-zenpi", "d".repeat(64), 0, 16, listOf(
                RemoteSession("s-zenpi", "1", 4, true, name = "Fix the flaky test"), RemoteSession("f5c2e8a0-6d47-4b19-8e3a-7c1f9d0b2e65", "1", 3, false)))) },
            { state.value = state.value.copy(sessionsOpen = false, sessions = null, diffOpen = true, diff = RemoteDiff("zenpi", "s-zenpi", main, "2", ".", true, false, false,
                "@@ -1 +1 @@\n-a\n+b\n", " M src/tui.rs\n")) },
            { state.value = state.value.copy(diffOpen = false, diff = null) },
            { compose.onNodeWithContentDescription("Desktop terminal").performScrollTo().performClick() },
            { compose.onNodeWithContentDescription("Back to conversation").performClick() },
            { compose.onNodeWithContentDescription("Worktree tasks").performScrollTo().performClick() },
            { compose.onNodeWithContentDescription("Back to conversation").performClick() },
            { state.value = state.value.copy(modelOpen = true) },
            { state.value = state.value.copy(modelOpen = false, resourcesOpen = true, resources = ResourceCatalog("zenpi", "s-zenpi", "2", listOf(
                RemoteResource("review", "template", "/review", "Review the pending change", true)), false, "a".repeat(64))) },
            { state.value = state.value.copy(resourcesOpen = false, resources = null, filesOpen = true, files = WorkspaceFiles("zenpi", "s-zenpi", main, "2", ".",
                listOf(WorkspaceFile("src", "src", true, null, false), WorkspaceFile("Cargo.toml", "Cargo.toml", false, "2048", true)), false)) },
            { state.value = state.value.copy(filesOpen = false, files = null) },
            { compose.onNodeWithContentDescription("Open switcher").performClick() },
        )
        screens.forEach { show ->
            show()
            compose.waitForIdle()
            compose.onAllNodes(isRoot()).check()
        }
    }

    @Test fun lightPalette() = walk(dark = false)
    @Test fun darkPalette() = walk(dark = true)
    @Test fun dynamicColour() = walk(dark = null)
}
