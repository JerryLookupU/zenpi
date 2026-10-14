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
    private val approval = RemoteApproval("approval-1", "zenpi", "s-zenpi", "turn-2", "call-1", "bash", "command_execution",
        "agent_tool", "c".repeat(64))
    val connected = PairingState(phase = PairingPhase.CONNECTED,
        host = PairedHost("192.168.1.20", 7443, "e".repeat(64), "d".repeat(64)), registered = true,
        projection = HostProjection(listOf(
            RemoteProject("zenpi", "zenpi", "s-zenpi", workspaceControl = true, promptControl = true, approvalControl = true,
                pendingApprovals = listOf(approval), sessionName = "Fix the flaky test",
                promptStatuses = listOf(RemotePromptStatus("request-1", "zenpi", "s-zenpi", "running"))),
            RemoteProject("docs", "docs", "s-docs", workspaceControl = true,
                promptStatuses = listOf(RemotePromptStatus("request-2", "docs", "s-docs", "running"))),
            RemoteProject("infra", "infra", "s-infra", workspaceControl = true)),
            "zenpi", "s-zenpi", "running", listOf(
                RemoteTurn("turn-1", "user", "fix the flaky test"),
                RemoteTurn("turn-2", "assistant", "The race is in the reconnect path; reading the tests first.",
                    toolCalls = listOf(RemoteToolCall("call-1", "bash", "{\"command\":\"cargo test reconnect\"}", false)))),
            "4", false, "9", workspaceOwner = "f".repeat(64), workspaceRevision = "3",
            subtabs = RemoteSubtabs("zenpi", "2", main, listOf(RemoteSubtab(main, "main", "main", 1),
                RemoteSubtab(worktree, "wt-a", "worktree", 2)), true, worktreeControl = true, concurrencyControl = true)))
}

@RunWith(ParameterizedRobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class MainScreenScreenshotTest(private val portrait: Portrait) {
    private val compose = createComposeRule()
    @get:Rule val rules: RuleChain = RuleChain.outerRule(PortraitRule(portrait)).around(compose)

    @Test fun mainScreen() {
        compose.setContent { ScreenshotTheme { AppShell(Fixtures.connected, object : PairingActions {}) } }
        compose.assertNoHorizontalClipping()
        compose.captureGolden("main", portrait)
    }

    companion object {
        @JvmStatic
        @ParameterizedRobolectricTestRunner.Parameters(name = "{0}")
        fun portraits(): List<Array<Any>> = Portrait.matrix.map { arrayOf(it) }
    }
}
