package zenpi.screenshot

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.ParameterizedRobolectricTestRunner
import org.robolectric.annotation.GraphicsMode
import zenpi.pairing.PairingState
import zenpi.transport.RemoteDiff
import zenpi.transport.RemoteModelCatalog
import zenpi.transport.RemoteModelChoice
import zenpi.transport.RemoteModelStatus
import zenpi.transport.RemoteProject
import zenpi.transport.RemoteSession
import zenpi.transport.RemoteTerminal
import zenpi.transport.RemoteWorkerStatus
import zenpi.transport.SessionCatalog
import zenpi.transport.TerminalSize
import zenpi.ui.AppShell
import zenpi.ui.PairingActions

/** The secondary pages (Sessions, Diff, Terminal, Tasks) and the model sheet, in every portrait cell. */
@RunWith(ParameterizedRobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class SecondaryPagesScreenshotTest(private val portrait: Portrait) {
    private val compose = createComposeRule()
    @get:Rule val rules: RuleChain = RuleChain.outerRule(PortraitRule(portrait)).around(compose)

    private val base = Fixtures.connected
    private val tabs = base.projection!!.subtabs!!

    private fun project(change: (RemoteProject) -> RemoteProject): PairingState = base.copy(projection = base.projection!!.copy(
        projects = base.projection!!.projects.map { if (it.id == "zenpi") change(it) else it }))

    private fun capture(state: PairingState, name: String, open: String? = null) {
        compose.setContent { ScreenshotTheme { AppShell(state, object : PairingActions {}) } }
        open?.let { compose.onNodeWithContentDescription(it).performScrollTo().performClick() }
        compose.waitForIdle()
        compose.assertNoHorizontalClipping()
        captureScreenGolden(name, portrait)
    }

    @Test fun sessions() = capture(base.copy(sessionsOpen = true, sessionsQuery = "flaky", sessions = SessionCatalog("zenpi", "s-zenpi", "d".repeat(64), 0, 16, listOf(
        RemoteSession("s-zenpi", "1", 4, true, name = "Fix the flaky test", matches = 2, snippets = listOf("fix the flaky test")),
        RemoteSession("s-older", "1", 12, false, name = "Reconnect cursor", matches = 1, snippets = listOf("the flaky reconnect test drops the cursor")),
        RemoteSession("s-oldest", "1", 3, false)), query = "flaky")), "sessions")

    @Test fun diff() = capture(project { it.copy(diffAvailable = true) }.copy(diffOpen = true, diff = RemoteDiff("zenpi", "s-zenpi", tabs.activeId, tabs.revision,
        ".", true, false, false, "diff --git a/src/remote/subscription.rs b/src/remote/subscription.rs\n@@ -41,3 +41,4 @@ fn reconnect\n" +
            "     let mut cursor = state.cursor;\n-    let events = subscription.resume(None)?;\n+    let events = subscription.resume(Some(cursor))?;\n" +
            "+    cursor = events.last_cursor();\n     Ok(events)\n", " M src/remote/subscription.rs\n M src/tui.rs\n")), "diff")

    @Test fun terminal() = capture(project { it.copy(terminalControl = true, terminal = RemoteTerminal("zenpi", "s-zenpi", tabs.activeId, tabs.revision,
        "$ cargo test reconnect\n   Compiling zenpi v0.1.0\n    Finished test profile\nrunning 1 test\ntest reconnect_keeps_cursor ... ok\n$ ",
        false, "c".repeat(64), TerminalSize(24, 80))) }, "terminal", open = "Desktop terminal")

    @Test fun tasks() = capture(project { it.copy(workerControl = true, workerStatuses = listOf(
        RemoteWorkerStatus("task-1", "zenpi", "s-zenpi", tabs.activeId, "running", streamText = "Reading src/remote/subscription.rs"),
        RemoteWorkerStatus("task-2", "zenpi", "s-zenpi", tabs.activeId, "succeeded", outputText = "All 412 tests pass."))) }, "tasks", open = "Worktree tasks")

    @Test fun modelSheet() = capture(project { it.copy(modelControl = true, reasoningControl = true) }.copy(modelOpen = true,
        modelStatus = RemoteModelStatus("zenpi", "s-zenpi", "gpt-x", "high", true, "7", mapOf("tools" to true, "images" to false), "128000", "8192",
            listOf(null, "low", "high"), RemoteModelCatalog("a".repeat(64), 0, null, listOf(RemoteModelChoice("gpt-x", "active", true))))), "model_sheet")

    companion object {
        @JvmStatic
        @ParameterizedRobolectricTestRunner.Parameters(name = "{0}")
        fun portraits(): List<Array<Any>> = Portrait.matrix.map { arrayOf(it) }
    }
}
