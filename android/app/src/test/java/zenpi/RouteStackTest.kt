package zenpi

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.requiredSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.performSemanticsAction
import androidx.activity.ComponentActivity
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performScrollToIndex
import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode
import zenpi.pairing.PairingState
import zenpi.screenshot.Fixtures
import zenpi.transport.RemoteDiff
import zenpi.transport.RemoteTerminal
import zenpi.transport.RemoteTurn
import zenpi.transport.TerminalSize
import zenpi.ui.AppShell
import zenpi.ui.DiffFiles
import zenpi.ui.PairingActions
import zenpi.ui.TerminalViewport
import zenpi.ui.nav.Route
import zenpi.ui.nav.RouteStack

@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(qualifiers = "w360dp-h800dp-port")
class RouteStackTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @Test fun pushPopAndReaderSyncKeepOneEntryPerPage() {
        val stack = RouteStack.saver.restore(emptyList())!!
        stack.push(Route.TASKS); stack.push(Route.TERMINAL); stack.push(Route.TASKS)
        assertEquals(listOf(Route.TERMINAL, Route.TASKS), stack.entries)
        // A reader opened by the view model is pushed; a reader it closed leaves the stack wherever it sits.
        stack.sync(sessionsOpen = true, diffOpen = false)
        assertEquals(Route.SESSIONS, stack.top)
        stack.sync(sessionsOpen = false, diffOpen = true)
        assertEquals(listOf(Route.TERMINAL, Route.TASKS, Route.DIFF), stack.entries)
        assertEquals(listOf("TERMINAL", "TASKS", "DIFF"), with(RouteStack.saver) { androidx.compose.runtime.saveable.SaverScope { true }.save(stack) })
        assertEquals(Route.DIFF, stack.pop()); assertEquals(Route.TASKS, stack.pop()); assertEquals(Route.TERMINAL, stack.pop())
        assertNull(stack.pop())
    }

    @Test fun backClosesTheReaderAndRestoresTheConversationScroll() {
        val base = Fixtures.connected.copy(projection = Fixtures.connected.projection!!.let { projection ->
            projection.copy(turns = projection.turns + (1..40).map { RemoteTurn("t$it", "assistant", "turn $it") },
                projects = projection.projects.map { if (it.id == "zenpi") it.copy(diffAvailable = true) else it })
        })
        var state by mutableStateOf(base)
        val closed = mutableListOf<String>()
        compose.setContent { MaterialTheme { AppShell(state, object : PairingActions {
            override fun diff(path: String) { state = state.copy(diffOpen = true, diff = diffOf(state, path)) }
            override fun closeDiff() { closed += "diff"; state = state.copy(diffOpen = false, diff = null) }
        }) } }
        compose.onNodeWithTag("conversation").performScrollToIndex(20)
        compose.onNodeWithText("turn 21").assertIsDisplayed()
        compose.onNodeWithContentDescription("Review diff").performScrollTo().performClick()
        compose.onNodeWithTag("diff-patch").assertIsDisplayed()
        compose.onNodeWithTag("conversation").assertDoesNotExist()
        compose.onNodeWithContentDescription("Close diff").performClick()
        compose.onNodeWithText("turn 21").assertIsDisplayed()
        assertEquals(listOf("diff"), closed)
        // A navigation-only page: back returns to the same place in the conversation.
        compose.onNodeWithContentDescription("Worktree tasks").performScrollTo().performClick()
        compose.onNodeWithText("No worktree tasks").assertIsDisplayed()
        compose.onNodeWithContentDescription("Back to conversation").performClick()
        compose.onNodeWithText("turn 21").assertIsDisplayed()
        // The system back gesture pops the same stack: Diff over Tasks, then Tasks, then the conversation.
        compose.onNodeWithContentDescription("Worktree tasks").performScrollTo().performClick()
        compose.runOnIdle { state = state.copy(diffOpen = true, diff = diffOf(state, ".")) }
        compose.onNodeWithTag("diff-patch").assertIsDisplayed()
        compose.runOnIdle { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithText("No worktree tasks").assertIsDisplayed()
        compose.runOnIdle { compose.activity.onBackPressedDispatcher.onBackPressed() }
        compose.onNodeWithText("turn 21").assertIsDisplayed()
        assertEquals(listOf("diff", "diff"), closed)
    }

    @Test fun terminalGridFollowsTheViewportAndIsRecomputedOnRotation() {
        assertEquals(TerminalSize(40, 50), TerminalViewport.fit(400f, 600f, 8f, 15f))
        assertEquals(TerminalSize(512, 1), TerminalViewport.fit(1f, 100000f, 8f, 15f))
        assertNull(TerminalViewport.fit(0f, 600f, 8f, 15f))
        val tab = Fixtures.connected.projection!!.subtabs!!.activeId
        val terminal = RemoteTerminal("zenpi", "s-zenpi", tab, "2", "$ cargo test\nok", false, "c".repeat(64), TerminalSize(24, 80))
        val state = Fixtures.connected.copy(projection = Fixtures.connected.projection!!.let { projection ->
            projection.copy(projects = projection.projects.map { if (it.id == "zenpi") it.copy(terminal = terminal, terminalControl = true) else it })
        })
        var portrait by mutableStateOf(true)
        val sizes = mutableListOf<Pair<Int, Int>>()
        compose.setContent { MaterialTheme { Box(Modifier.requiredSize(if (portrait) 360.dp else 800.dp, if (portrait) 800.dp else 360.dp)) {
            AppShell(state, object : PairingActions { override fun terminalResize(rows: Int, cols: Int) { sizes += rows to cols } })
        } } }
        compose.onNodeWithContentDescription("Desktop terminal").performScrollTo().performClick()
        compose.onNodeWithTag("desktop-terminal").assertIsDisplayed()
        compose.onNodeWithContentDescription("Resize terminal").performScrollTo().performClick()
        compose.runOnIdle { portrait = false }
        // The landscape box overhangs the portrait test window, so the click goes through semantics.
        compose.onNodeWithContentDescription("Resize terminal").performSemanticsAction(SemanticsActions.OnClick)
        compose.runOnIdle {
            assertEquals(2, sizes.size)
            val (tall, wide) = sizes
            assertNotEquals(tall, wide)
            assert(wide.second > tall.second && wide.first < tall.first) { "rotation should trade rows for columns: $sizes" }
        }
    }

    @Test fun changedFilesComeFromThePorcelainStatus() {
        assertEquals(listOf("src/a.rs", "new name.txt", "b.rs"),
            DiffFiles.of(" M src/a.rs\nR  old.txt -> \"new name.txt\"\n?? b.rs\n?? dir/\n"))
    }

    private fun diffOf(state: PairingState, path: String): RemoteDiff {
        val tabs = state.projection!!.subtabs!!
        return RemoteDiff("zenpi", "s-zenpi", tabs.activeId, tabs.revision, path, true, false, false,
            "@@ -1 +1 @@\n-old\n+new\n", " M src/remote/subscription.rs")
    }
}
