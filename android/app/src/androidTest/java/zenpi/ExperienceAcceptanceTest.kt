package zenpi

import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.test.SemanticsMatcher
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasContentDescription
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createEmptyComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollToIndex
import androidx.compose.ui.test.performScrollToKey
import androidx.compose.ui.test.performTouchInput
import androidx.compose.ui.test.swipeDown
import androidx.compose.ui.test.swipeUp
import androidx.test.core.app.ActivityScenario
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
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
import zenpi.ui.theme.ZenpiTheme

/**
 * ZE-591 acceptance on the device, without a host: the approval bar stays on screen at every scroll
 * position of a 10,000-turn conversation, and the primary actions sit in the bottom 40% of the
 * screen. The host-backed requirements are in [HostExperienceTest]; the 10,000-turn scroll frame
 * rate and the offline cached render (release-like build) are in android/macrobenchmark.
 */
class ExperienceAcceptanceTest {
    @get:Rule val compose = createEmptyComposeRule()
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val approval = RemoteApproval("r-591", "p", "s", "t", "c", "write_file", "workspace_write", "agent_tool", "a".repeat(64))

    private fun turn(index: Int) = if (index % 2 == 0) RemoteTurn("t$index", "user", "Step $index: check `module_$index`.")
    else RemoteTurn("t$index", "assistant", "Turn $index found:\n- `src/file_$index.rs` changed\n```rust\nfn check_$index() {}\n```",
        toolCalls = listOf(RemoteToolCall("c$index", "bash", "{\"command\":\"cargo test case_$index\"}", false)))

    /** A connected session of 10,000 turns with one pending approval this device may decide; idle or with a running message. */
    private fun state(running: Boolean) = PairingState(phase = PairingPhase.CONNECTED,
        host = PairedHost("192.168.1.20", 7443, "e".repeat(64), "d".repeat(64)), registered = true,
        projection = HostProjection(listOf(RemoteProject("p", "zenpi", "s", workspaceControl = true, promptControl = true,
            approvalControl = true, pendingApprovals = listOf(approval),
            promptStatuses = if (running) listOf(RemotePromptStatus("r", "p", "s", "running", streamText = "Streaming answer")) else emptyList())),
            "p", "s", if (running) "running" else "idle", (0 until 10_000).map(::turn), "10000", false, "10000",
            workspaceOwner = "f".repeat(64), workspaceRevision = "1",
            subtabs = RemoteSubtabs("p", "1", "a".repeat(64), listOf(RemoteSubtab("a".repeat(64), "main", "main", 1)), false)))

    private fun show(state: PairingState): ActivityScenario<ComponentActivity> = ActivityScenario.launch(ComponentActivity::class.java).also { scenario ->
        scenario.onActivity { it.setContent { ZenpiTheme { AppShell(state, object : PairingActions {}) } } }
        compose.waitForIdle()
    }

    /** Bounds in screen pixels: the approval sheet is a window of its own. */
    private fun onScreen(matcher: SemanticsMatcher): Rect = compose.onNode(matcher).assertIsDisplayed().fetchSemanticsNode().let {
        Rect(it.positionOnScreen, Size(it.size.width.toFloat(), it.size.height.toFloat()))
    }
    private fun screenHeight(): Int = instrumentation.targetContext.getSystemService(android.view.WindowManager::class.java)
        .maximumWindowMetrics.bounds.height()

    @Test fun approvalBarStaysOnScreenAtEveryScrollPositionOfTenThousandTurns() {
        show(state(running = true)).use {
            val list = compose.onNodeWithTag("conversation")
            val height = screenHeight()
            fun barOnScreen(where: String) {
                val bar = onScreen(hasTestTag("approval-bar"))
                Log.i("ZE591", "approval bar at $where: top=${bar.top} bottom=${bar.bottom} of $height")
                assertTrue("$where: bar $bar, screen height $height", bar.top >= 0f && bar.bottom <= height)
                // Still one tap from a decision.
                onScreen(hasTestTag("Allow approval r-591")); onScreen(hasTestTag("Deny approval r-591"))
            }
            barOnScreen("newest")
            list.performScrollToKey("turn-5000-t5000"); barOnScreen("middle (turn 5000)")
            list.performScrollToKey("workspace"); barOnScreen("oldest (above turn 0)")
            repeat(5) { list.performTouchInput { swipeUp() } }; barOnScreen("after 5 flings toward newer")
            list.performScrollToKey("turn-9000-t9000")
            repeat(5) { list.performTouchInput { swipeDown() } }; barOnScreen("after 5 flings toward older")
            list.performScrollToIndex(0); barOnScreen("newest again")
        }
    }

    @Test fun primaryActionsSitInTheBottomFortyPercentOfTheScreen() {
        val height = screenHeight()
        fun inBottom40(label: String, matcher: SemanticsMatcher) {
            val bounds = onScreen(matcher)
            Log.i("ZE591", "primary action '$label' top=${bounds.top} (${"%.1f".format(100 * bounds.top / height)}% of $height)")
            assertTrue("$label top ${bounds.top} is above 60% of the screen height $height", bounds.top >= 0.6f * height)
        }
        show(state(running = false)).use {
            // Idle: composer, Send, the palette triggers, and the approval bar's decisions.
            inBottom40("composer", hasTestTag("prompt-composer"))
            inBottom40("send", hasTestTag("composer-action"))
            inBottom40("host resources (/)", hasContentDescription("Select host resource"))
            inBottom40("attach file (@)", hasContentDescription("Attach host file"))
            inBottom40("approval bar: review", hasContentDescription("Review approvals"))
            inBottom40("approval bar: deny", hasTestTag("Deny approval r-591"))
            inBottom40("approval bar: allow", hasTestTag("Allow approval r-591"))
            // The approval sheet's one-tap decisions.
            compose.onNodeWithContentDescription("Review approvals").performClick()
            inBottom40("sheet: deny", hasText("Deny") and hasAnyAncestor(hasTestTag("approval-sheet")))
            inBottom40("sheet: allow once", hasText("Allow once") and hasAnyAncestor(hasTestTag("approval-sheet")))
        }
        show(state(running = true)).use {
            // Running: the same button stops the message (or steers with text).
            inBottom40("stop", hasTestTag("composer-action") and hasContentDescription("Stop message"))
        }
    }
}
