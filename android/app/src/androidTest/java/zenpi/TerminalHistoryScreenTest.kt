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
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class TerminalHistoryScreenTest {
    @get:Rule val compose = createComposeRule()
    @Test fun retainedHistoryHasExplicitNavigationRefreshAndOfflineFences() {
        val tab = "b".repeat(64)
        val terminal = RemoteTerminal("project", "session", tab, "1", "live", false, "c".repeat(64))
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", terminal = terminal)),
            "project", "session", "idle", emptyList(), "1", false, "0", subtabs = RemoteSubtabs("project", "1", tab,
                listOf(RemoteSubtab(tab, "Main", "main", 1)), false))
        val page = TerminalPage("project", "session", "c".repeat(64), tab, "1", "d".repeat(64), 0, 3, 6, true, "abc")
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection))
        val calls = mutableListOf<Boolean>()
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onTerminalPage = { calls += it })
        } } }
        compose.onNodeWithContentDescription("Desktop terminal").performScrollTo().performClick()
        compose.onNodeWithContentDescription("Read terminal history").performClick()
        compose.runOnIdle { assertEquals(listOf(false), calls); state.value = state.value.copy(terminalOpen = true, terminalPage = page) }
        compose.onNodeWithTag("terminal-history-page").assertTextEquals("abc")
        compose.onNodeWithContentDescription("Next terminal history page").performClick()
        compose.runOnIdle { assertEquals(listOf(false, true), calls); state.value = state.value.copy(terminalError = "Changed") }
        compose.onNodeWithContentDescription("Next terminal history page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Refresh terminal history").performClick()
        compose.runOnIdle { assertEquals(listOf(false, true, false), calls); state.value = state.value.copy(terminalError = null) }
        val current = page.copy(offset = 3, nextOffset = 6, totalBytes = 9, text = "def")
        val preceding = page.copy(totalBytes = 9)
        compose.runOnIdle { state.value = state.value.copy(terminalPage = current, terminalPrevious = listOf(preceding)) }
        compose.onNodeWithContentDescription("Previous terminal history page").assertIsEnabled()
        compose.onNodeWithContentDescription("Next terminal history page").assertIsEnabled()
        for (foreign in listOf(current.copy(projectId = "other"), current.copy(sessionId = "other"),
            current.copy(subtabRevision = "2"), current.copy(subtabId = "e".repeat(64)), current.copy(terminalId = "e".repeat(64)),
            current.copy(text = "d"), current.copy(nextOffset = 7), current.copy(digest = "invalid"))) {
            compose.runOnIdle { state.value = state.value.copy(terminalPage = foreign) }
            compose.onNodeWithContentDescription("Previous terminal history page").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Next terminal history page").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Refresh terminal history").assertIsEnabled()
        }
        compose.runOnIdle { state.value = state.value.copy(terminalPage = current, terminalPrevious = listOf(preceding.copy(digest = "e".repeat(64)))) }
        compose.onNodeWithContentDescription("Previous terminal history page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Next terminal history page").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(terminalPrevious = listOf(preceding.copy(text = "a"))) }
        compose.onNodeWithContentDescription("Previous terminal history page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Next terminal history page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Refresh terminal history").assertIsEnabled()
        val healthy = state.value.copy(terminalPage = current, terminalPrevious = listOf(preceding))
        for (blocked in listOf(healthy.copy(scanInProgress = true), healthy.copy(resetting = true), healthy.copy(resetRequired = true),
            healthy.copy(terminalLoading = true), healthy.copy(pendingRevoke = "pending"),
            healthy.copy(projection = projection.copy(projects = projection.projects + projection.projects.single().copy(sessionId = "foreign"))),
            healthy.copy(projection = projection.copy(subtabs = projection.subtabs?.copy(tabs = emptyList()))),
            healthy.copy(projection = projection.copy(subtabs = projection.subtabs?.let { it.copy(tabs = it.tabs + it.tabs) })))) {
            compose.runOnIdle { state.value = blocked }
            compose.onNodeWithContentDescription("Previous terminal history page").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Next terminal history page").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Refresh terminal history").assertIsNotEnabled()
        }
        compose.runOnIdle { state.value = healthy.copy(terminalOpen = false, scanInProgress = true) }
        compose.onNodeWithContentDescription("Read terminal history").assertIsNotEnabled()
        compose.runOnIdle { state.value = healthy }
        compose.runOnIdle { state.value = state.value.copy(projection = projection.copy(projects = projection.projects.map { it.copy(terminal = null) })) }
        compose.onNodeWithContentDescription("Refresh terminal history").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithContentDescription("Refresh terminal history").assertIsNotEnabled()
    }
}
