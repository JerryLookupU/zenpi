package zenpi

import zenpi.ui.PairingScreen
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class WorkspaceScreenTest {
    @get:Rule val compose = createComposeRule()
    private val projection = HostProjection(listOf(RemoteProject("one", "One", "s1", true), RemoteProject("two", "Two", "s2", true)),
        "one", "s1", "idle", emptyList(), "1", false, "0", workspaceOwner = "a".repeat(64), workspaceRevision = "1")
    @Test fun workspaceActionsNeedExplicitConfirmationAndUseStableIds() {
        val actions = mutableListOf<Triple<String, String, String?>>()
        compose.setContent { MaterialTheme { PairingScreen(PairingState(phase = PairingPhase.CONNECTED, projection = projection),
            {}, {}, {}, onWorkspace = { operation, project, cwd -> actions.add(Triple(operation, project, cwd)) }) } }
        compose.onNodeWithText("Two").performClick()
        assertEquals(Triple("workspace_select", "two", null), actions.single()); actions.clear()
        compose.onNodeWithContentDescription("Open workspace").performScrollTo().performClick()
        compose.onNodeWithText("Open").assertIsNotEnabled()
        compose.onNodeWithText("Host directory").performTextInput("child")
        compose.onNodeWithText("Host directory").performImeAction()
        compose.waitForIdle()
        compose.onNodeWithText("Open").performClick()
        compose.waitUntil(3000) { actions.isNotEmpty() }
        assertEquals(Triple("workspace_open", "one", "child"), actions.single()); actions.clear()
        compose.onNodeWithContentDescription("Close workspace").performScrollTo().performClick()
        assertTrue(actions.isEmpty())
        compose.onNodeWithText("Close").performClick()
        compose.waitUntil(3000) { actions.isNotEmpty() }
        assertEquals(Triple("workspace_close", "one", null), actions.single())
    }
    @Test fun ambiguousWriteDisablesNewActionsAndRetriesOnlyTheOriginalRequest() {
        val command = WorkspaceCommand.create(projection, "one", "workspace_select")
        var retry = false
        compose.setContent { MaterialTheme { PairingScreen(PairingState(phase = PairingPhase.ERROR, projection = projection,
            pendingWorkspace = command, error = "Workspace result unknown"), {}, {}, {}, onRetryWorkspace = { retry = true }) } }
        compose.onNodeWithText("Two").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Open workspace").performScrollTo().assertIsNotEnabled()
        compose.onNodeWithContentDescription("Close workspace").assertIsNotEnabled()
        compose.onNodeWithText("Retry same request").performClick()
        compose.waitUntil(3000) { retry }; assertTrue(retry)
    }
    @Test fun scanningResetAndDuplicateWorkspaceIdentityDisableTargets() {
        val healthy = PairingState(phase = PairingPhase.CONNECTED, projection = projection)
        val state = mutableStateOf(healthy)
        val actions = mutableListOf<String>()
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {},
            onWorkspace = { _, id, _ -> actions.add(id) }) } }
        for (blocked in listOf(healthy.copy(scanInProgress = true), healthy.copy(resetting = true), healthy.copy(resetRequired = true))) {
            compose.runOnIdle { state.value = blocked }
            compose.onNodeWithTag("workspace-tab-two").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Open workspace").performScrollTo().assertIsNotEnabled()
            compose.onNodeWithContentDescription("Close workspace").assertIsNotEnabled()
        }
        compose.runOnIdle { state.value = healthy.copy(projection = projection.copy(projects = projection.projects +
            projection.projects[1].copy(sessionId = "foreign", workspaceControl = false))) }
        compose.onAllNodesWithTag("workspace-tab-two").assertCountEquals(2)
        compose.onAllNodesWithTag("workspace-tab-two")[0].assertIsNotEnabled()
        compose.onAllNodesWithTag("workspace-tab-two")[1].assertIsNotEnabled()
        compose.runOnIdle { state.value = healthy }
        compose.onNodeWithTag("workspace-tab-two").performScrollTo().performClick()
        assertEquals(listOf("two"), actions)
    }
}
