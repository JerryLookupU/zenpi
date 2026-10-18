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

class SessionNewScreenTest {
    @get:Rule val compose = createComposeRule()
    @Test fun newSessionRequiresConfirmationAndOldSessionScope() {
        val tab = "b".repeat(64)
        val project = RemoteProject("a".repeat(64), "Workspace", "session", sessionControl = true)
        val projection = HostProjection(listOf(project), project.id, "session", "idle", emptyList(), "1", false, "0",
            subtabs = RemoteSubtabs(project.id, "1", tab, listOf(RemoteSubtab(tab, "Main", "main", 1)), false))
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection))
        val tabs = requireNotNull(projection.subtabs)
        var created = 0
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onNewSession = { created++ })
        } } }
        compose.onNodeWithContentDescription("New session").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(0, created) }
        compose.onNodeWithContentDescription("Confirm new session").performClick()
        compose.runOnIdle { assertEquals(1, created) }
        compose.onNodeWithContentDescription("New session").performScrollTo().performClick()
        compose.runOnIdle { state.value = state.value.copy(projection = projection.copy(sessionId = "other")) }
        compose.onNodeWithContentDescription("Confirm new session").assertDoesNotExist()
        compose.runOnIdle { state.value = state.value.copy(projection = projection) }
        compose.onNodeWithContentDescription("Confirm new session").assertDoesNotExist()
        compose.onNodeWithContentDescription("New session").performScrollTo().performClick()
        for (blocked in listOf(state.value.copy(scanInProgress = true), state.value.copy(resetting = true), state.value.copy(resetRequired = true),
            state.value.copy(projection = projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs))),
            state.value.copy(projection = projection.copy(subtabs = tabs.copy(projectId = "foreign"))),
            state.value.copy(projection = projection.copy(projects = listOf(project, project.copy(sessionId = "foreign")))))) {
            compose.runOnIdle { state.value = blocked }
            compose.onNodeWithContentDescription("Confirm new session").assertIsNotEnabled()
        }
        compose.onNodeWithContentDescription("Cancel new session").performClick()
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.OFFLINE, projection = projection) }
        compose.onNodeWithContentDescription("New session").performScrollTo().assertIsNotEnabled()
    }
}
