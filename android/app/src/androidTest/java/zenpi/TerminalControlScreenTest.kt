package zenpi

import zenpi.ui.PairingScreen
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class TerminalControlScreenTest {
    @get:Rule val compose = createComposeRule()
    @Test fun terminalInputInterruptAndResizeUseTheHostControlAtNarrowWidth() {
        val tab = "b".repeat(64)
        val terminal = RemoteTerminal("project", "session", tab, "1", "HOST_COLUMN_".repeat(30) + "\nSECOND_ROW", false, "c".repeat(64), TerminalSize(33, 91))
        val project = RemoteProject("project", "Workspace", "session", terminal = terminal, terminalControl = true)
        val projection = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false, "0",
            workspaceOwner = "a".repeat(64), workspaceRevision = "3", subtabs = RemoteSubtabs("project", "1", tab,
                listOf(RemoteSubtab(tab, "Main", "main", 1)), true))
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection))
        val inputs = mutableListOf<String>(); val sizes = mutableListOf<Pair<Int, Int>>(); var opened = 0
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onTerminalInput = { inputs += it }, onTerminalResize = { rows, cols -> sizes += rows to cols },
                onOpenDesktopTerminal = { opened++ })
        } } }
        compose.onNodeWithTag("terminal-viewport").performScrollTo()
        val layouts = mutableListOf<androidx.compose.ui.text.TextLayoutResult>()
        compose.onNodeWithTag("desktop-terminal").performSemanticsAction(androidx.compose.ui.semantics.SemanticsActions.GetTextLayoutResult) { it(layouts) }
        assertEquals(2, layouts.single().lineCount)
        fun horizontal() = compose.onNodeWithTag("terminal-viewport").fetchSemanticsNode().config[androidx.compose.ui.semantics.SemanticsProperties.HorizontalScrollAxisRange]
        assertTrue(horizontal().maxValue() > 0)
        compose.onNodeWithTag("terminal-viewport").performTouchInput { swipeLeft() }
        compose.waitForIdle(); assertTrue(horizontal().value() > 0)
        compose.runOnIdle { state.value = state.value.copy(projection = projection.copy(projects = listOf(
            project.copy(terminal = terminal.copy(text = terminal.text + "\nNEW_ROW"))))) }
        compose.waitForIdle(); assertTrue(horizontal().value() > 0)
        compose.onNodeWithTag("terminal-input").performScrollTo().performTextInput("pwd")
        compose.onNodeWithContentDescription("Send terminal input").performScrollTo().performClick()
        compose.onNodeWithContentDescription("Interrupt terminal").performScrollTo().performClick()
        compose.onNodeWithContentDescription("Resize terminal").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(listOf("pwd\r", "\u0003"), inputs); assertEquals(listOf(33 to 91), sizes) }
        for (key in TerminalKey.entries) {
            compose.onNodeWithContentDescription("Terminal keys").performScrollTo().performClick()
            compose.onNodeWithText(key.label).performScrollTo().performClick()
            compose.runOnIdle { assertEquals(key.bytes, inputs.last()) }
        }
        compose.runOnIdle { inputs.subList(2, inputs.size).clear() }
        compose.onNodeWithContentDescription("Terminal keys").performClick()
        compose.runOnIdle { state.value = state.value.copy(pendingRevoke = "unresolved") }
        compose.onNodeWithText("Backspace").assertDoesNotExist()
        compose.onNodeWithContentDescription("Terminal keys").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(pendingRevoke = null) }
        compose.onNodeWithContentDescription("Terminal keys").performClick()
        compose.runOnIdle { state.value = state.value.copy(resetting = true) }
        compose.onNodeWithText("Backspace").assertDoesNotExist()
        compose.runOnIdle { state.value = state.value.copy(resetting = false) }
        compose.onNodeWithText("Rows").performScrollTo().performTextReplacement("41")
        compose.runOnIdle { state.value = state.value.copy(projection = projection.copy(projects = listOf(
            project.copy(terminal = terminal.copy(text = "output only"))))) }
        compose.onNodeWithContentDescription("Resize terminal").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(41 to 91, sizes.last()) }
        compose.runOnIdle { state.value = state.value.copy(projection = projection.copy(projects = listOf(
            project.copy(terminal = terminal.copy(size = TerminalSize(25, 82)))))) }
        compose.onNodeWithContentDescription("Resize terminal").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(25 to 82, sizes.last()) }
        compose.onNodeWithTag("terminal-input").performScrollTo().performTextInput("old draft")
        compose.runOnIdle { state.value = state.value.copy(projection = projection.copy(projects = listOf(
            project.copy(terminal = terminal.copy(text = "new output"))))) }
        compose.onNodeWithTag("terminal-input").assertTextContains("old draft")
        val revised = projection.copy(subtabs = projection.subtabs?.copy(revision = "2"),
            projects = listOf(project.copy(terminal = terminal.copy(subtabRevision = "2"))))
        compose.runOnIdle { state.value = state.value.copy(projection = revised) }
        compose.onNodeWithTag("terminal-viewport").performScrollTo()
        assertEquals(0f, horizontal().value())
        compose.onNodeWithContentDescription("Send terminal input").assertIsNotEnabled()
        compose.onNodeWithTag("terminal-input").performTextInput("owner draft")
        compose.runOnIdle { state.value = state.value.copy(projection = revised.copy(workspaceOwner = "d".repeat(64))) }
        compose.onNodeWithContentDescription("Send terminal input").assertIsNotEnabled()
        compose.onNodeWithTag("terminal-input").performTextInput("new command")
        val connected = state.value
        compose.runOnIdle { state.value = connected.copy(scanInProgress = true) }
        compose.onNodeWithContentDescription("Send terminal input").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Interrupt terminal").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Resize terminal").assertIsNotEnabled()
        compose.runOnIdle { state.value = connected }
        compose.onNodeWithContentDescription("Send terminal input").performScrollTo().performClick()
        compose.runOnIdle {
            assertEquals(listOf("pwd\r", "\u0003", "new command\r"), inputs)
            state.value = state.value.copy(projection = revised.copy(projects = revised.projects.map { it.copy(terminalControl = false) })) }
        compose.onNodeWithTag("terminal-input").assertDoesNotExist()
        compose.onNodeWithTag("desktop-terminal").assertExists()
        compose.runOnIdle { state.value = state.value.copy(projection = revised.copy(projects = listOf(
            project.copy(terminal = terminal.copy(projectId = "foreign", subtabRevision = "2"))))) }
        compose.onNodeWithTag("terminal-viewport").assertDoesNotExist()
        compose.onNodeWithContentDescription("Terminal keys").assertDoesNotExist()
        val tabs = requireNotNull(projection.subtabs)
        val other = RemoteSubtab("d".repeat(64), "Other", "in_place", 1)
        val invalidTabs = tabs.copy(tabs = tabs.tabs + listOf(other, other))
        compose.runOnIdle { state.value = PairingState(phase = PairingPhase.CONNECTED,
            projection = projection.copy(subtabs = invalidTabs)) }
        compose.onNodeWithTag("terminal-viewport").assertDoesNotExist()
        compose.onNodeWithTag("terminal-input").assertDoesNotExist()
        compose.onNodeWithContentDescription("Read terminal history").assertDoesNotExist()
        for (bad in listOf(terminal.copy(text = "bad\u001b"), terminal.copy(size = TerminalSize(0, 80)),
            terminal.copy(terminalId = "invalid"))) {
            compose.runOnIdle { state.value = PairingState(phase = PairingPhase.CONNECTED,
                projection = projection.copy(projects = listOf(project.copy(terminal = bad)))) }
            compose.onNodeWithTag("terminal-viewport").assertDoesNotExist()
            compose.onNodeWithTag("terminal-input").assertDoesNotExist()
            compose.onNodeWithContentDescription("Read terminal history").assertDoesNotExist()
        }
        val closed = projection.copy(projects = listOf(project.copy(terminal = null, terminalControl = false, terminalOpenControl = true)))
        compose.runOnIdle { state.value = PairingState(phase = PairingPhase.CONNECTED, projection = closed) }
        compose.onNodeWithContentDescription("Open desktop terminal").performScrollTo().assertIsEnabled().performClick()
        compose.runOnIdle { assertEquals(1, opened) }
        for (bad in listOf(state.value.copy(resetRequired = true), state.value.copy(resetting = true), state.value.copy(pendingRevoke = "pending"),
            state.value.copy(projection = closed.copy(subtabs = invalidTabs)),
            state.value.copy(scanInProgress = true))) {
            compose.runOnIdle { state.value = bad }
            compose.onNodeWithContentDescription("Open desktop terminal").assertIsNotEnabled()
            compose.runOnIdle { assertEquals(1, opened) }
        }
        compose.runOnIdle { state.value = PairingState(phase = PairingPhase.CONNECTED,
            projection = closed.copy(projects = closed.projects + project.copy(sessionId = "foreign"))) }
        compose.onNodeWithContentDescription("Open desktop terminal").assertDoesNotExist()
        compose.runOnIdle { assertEquals(1, opened) }
        compose.runOnIdle { state.value = PairingState(phase = PairingPhase.OFFLINE, projection = closed) }
        compose.onNodeWithContentDescription("Open desktop terminal").assertDoesNotExist()
        val exited = projection.copy(projects = listOf(project.copy(terminal = terminal.copy(running = false), terminalOpenControl = true)))
        compose.runOnIdle { state.value = PairingState(phase = PairingPhase.CONNECTED, projection = exited) }
        compose.onNodeWithText("Terminal exited").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("desktop-terminal").assertExists()
        compose.onNodeWithTag("terminal-input").assertDoesNotExist()
        compose.onNodeWithContentDescription("Resize terminal").assertDoesNotExist()
        compose.onNodeWithContentDescription("Open desktop terminal").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(2, opened) }
        compose.runOnIdle { state.value = PairingState(phase = PairingPhase.CONNECTED, projection = closed.copy(
            projects = listOf(project.copy(terminal = null, terminalOpenControl = false)))) }
        compose.onNodeWithContentDescription("Open desktop terminal").assertDoesNotExist()
    }
}
