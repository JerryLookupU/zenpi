package zenpi.screenshot

import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.ParameterizedRobolectricTestRunner
import org.robolectric.annotation.GraphicsMode
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.security.PairedHost
import zenpi.transport.HostProjection
import zenpi.transport.RemoteApproval
import zenpi.transport.RemoteApprovalPreview
import zenpi.transport.RemoteProject
import zenpi.transport.RemotePromptStatus
import zenpi.transport.RemoteSubtab
import zenpi.transport.RemoteSubtabs
import zenpi.transport.RemoteToolCall
import zenpi.transport.RemoteTurn
import zenpi.ui.AppShell
import zenpi.ui.PairingActions

/** A connected host with one project waiting on the user, one running and one idle. */
object Fixtures {
    private val main = "a".repeat(64)
    private val worktree = "b".repeat(64)
    private val approval = RemoteApproval("0b7e4c2a-6f1d-4d8e-9a35-2c4f7e1b9d60", "zenpi", "s-zenpi", "e3f8a0b4-7d29-4c61-a5e3-8b0f2d6c4a17", "1a6c3e9f-4b80-47d2-9f5a-6e2b8d0c3f41", "bash", "command_execution",
        "agent_tool", "c".repeat(64))
    val connected = PairingState(phase = PairingPhase.CONNECTED,
        host = PairedHost("192.168.1.20", 7443, "e".repeat(64), "d".repeat(64)), registered = true,
        projection = HostProjection(listOf(
            RemoteProject("zenpi", "zenpi", "s-zenpi", workspaceControl = true, promptControl = true, approvalControl = true,
                pendingApprovals = listOf(approval), sessionName = "Fix the flaky test",
                promptStatuses = listOf(RemotePromptStatus("4e9b7d0c-1f38-4a65-b2d7-9c0e6a3f8b54", "zenpi", "s-zenpi", "running",
                    streamText = "Found it: `reconnect()` drops the cursor when"))),
            RemoteProject("docs", "docs", "s-docs", workspaceControl = true,
                promptStatuses = listOf(RemotePromptStatus("a2d6f4c8-5b1e-4f93-8c70-e1b4d9a6c3f7", "docs", "s-docs", "running"))),
            RemoteProject("infra", "infra", "s-infra", workspaceControl = true)),
            "zenpi", "s-zenpi", "running", listOf(
                RemoteTurn("9c41e6d3-0a5b-4f72-b8e9-14d6a3c7f025", "user", "fix the flaky test"),
                RemoteTurn("e3f8a0b4-7d29-4c61-a5e3-8b0f2d6c4a17", "assistant", "The race is in the `reconnect` path. Plan:\n- read `src/tui.rs`\n- run the test\n" +
                    "```rust\nlet cursor = subscription.resume(cursor)?; // keep the cursor across reconnects\n```",
                    toolCalls = listOf(RemoteToolCall("1a6c3e9f-4b80-47d2-9f5a-6e2b8d0c3f41", "bash", "{\"command\":\"cargo test reconnect\"}", false)))),
            "4", false, "9", workspaceOwner = "f".repeat(64), workspaceRevision = "3",
            subtabs = RemoteSubtabs("zenpi", "2", main, listOf(RemoteSubtab(main, "main", "main", 1),
                RemoteSubtab(worktree, "wt-a", "worktree", 2)), true, worktreeControl = true, concurrencyControl = true)))

    /** Two approvals queued on the selected project; the head carries a diff preview. */
    val queued: List<RemoteApproval> = listOf(
        RemoteApproval("0b7e4c2a-6f1d-4d8e-9a35-2c4f7e1b9d60", "zenpi", "s-zenpi", "e3f8a0b4-7d29-4c61-a5e3-8b0f2d6c4a17", "7f0d2b5e-9a14-4e38-b6c7-3d9e1f4a8c02", "edit_file", "workspace_write", "agent_tool", "c".repeat(64),
            RemoteApprovalPreview("src/remote/subscription.rs", "@@ -41,3 +41,4 @@ fn reconnect\n     let mut cursor = state.cursor;\n" +
                "-    let events = subscription.resume(None)?;\n+    let events = subscription.resume(Some(cursor))?;\n+    cursor = events.last_cursor();\n" +
                "     Ok(events)\n", changed = true, truncated = false, redacted = false, withheld = false)),
        RemoteApproval("5d2a9f81-3c6e-4b07-8e14-a7f0c3d29b56", "zenpi", "s-zenpi", "e3f8a0b4-7d29-4c61-a5e3-8b0f2d6c4a17", "c8b5f1a7-2e63-4d90-8a4b-5f7c0e3d9b18", "bash", "command_execution", "agent_tool", "d".repeat(64)))
    val queuedState: PairingState = connected.copy(projection = connected.projection!!.copy(projects = connected.projection!!.projects.map {
        if (it.id == "zenpi") it.copy(pendingApprovals = queued) else it }))
}

@RunWith(ParameterizedRobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class MainScreenScreenshotTest(private val portrait: Portrait) {
    private val compose = createComposeRule()
    @get:Rule val rules: RuleChain = RuleChain.outerRule(PortraitRule(portrait)).around(compose)

    @Test fun mainScreen() {
        compose.setContent { ScreenshotTheme { AppShell(Fixtures.connected, object : PairingActions {}) } }
        compose.assertNoHorizontalClipping()
        compose.assertAccessibleText()
        compose.captureGolden("main", portrait)
    }

    companion object {
        @JvmStatic
        @ParameterizedRobolectricTestRunner.Parameters(name = "{0}")
        fun portraits(): List<Array<Any>> = Portrait.matrix.map { arrayOf(it) }
    }
}
