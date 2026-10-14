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

class WorkerStatusPageScreenTest {
    @get:Rule val compose = createComposeRule()
    @Test fun secondPageCancelUsesListedIdentityAndDisablesStaleOrUnknownRuntimeControls() {
        val task = RemoteWorkerStatus("second-page", "project", "session", "a".repeat(64), "running")
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", workerControl = true)),
            "project", "session", "idle", emptyList(), "1", false, "0")
        val page = WorkerStatusPage("project", "session", "b".repeat(64), 16, 17, 17, false, listOf(task))
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection,
            workerStatusesOpen = true, workerStatusesPage = page))
        val cancelled = mutableListOf<String>()
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onCancelWorker = { cancelled.add(it) }) } }
        val button = compose.onNodeWithContentDescription("Cancel listed task second-page")
        button.assertIsEnabled().performClick()
        assertEquals(listOf("second-page"), cancelled)
        for (mixed in listOf(task.copy(state = "succeeded"), task.copy(sessionId = "foreign", state = "succeeded"))) {
            compose.runOnUiThread { state.value = state.value.copy(workerStatusesPage = page.copy(tasks = listOf(task, mixed))) }
            button.assertIsNotEnabled()
        }
        compose.runOnUiThread { state.value = state.value.copy(workerStatusesPage = page) }
        button.assertIsEnabled()
        for (changed in listOf(state.value.copy(workerStatusesLoading = true), state.value.copy(workerStatusesError = "changed"),
            state.value.copy(phase = PairingPhase.OFFLINE), state.value.copy(pendingRevoke = "revoke"),
            state.value.copy(projection = projection.copy(projects = projection.projects.map { it.copy(workerStatuses = listOf(task.copy(state = "succeeded"))) })))) {
            compose.runOnUiThread { state.value = changed }; button.assertIsNotEnabled()
        }
        compose.runOnUiThread { state.value = PairingState(phase = PairingPhase.CONNECTED, projection = projection,
            workerStatusesOpen = true, workerStatusesPage = page.copy(tasks = listOf(task.copy(state = "unknown_outcome")))) }
        button.assertDoesNotExist()
        compose.runOnUiThread { state.value = state.value.copy(workerStatusesPage = page.copy(tasks = listOf(task.copy(state = "unknown_outcome", runtimeActive = true)))) }
        button.assertIsEnabled()
    }
    @Test fun completedSecondPageHasOutputAndHistoryCommandsWithoutControlCapability() {
        val task = RemoteWorkerStatus("completed-page", "project", "session", "a".repeat(64), "succeeded")
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session")), "project", "session", "idle", emptyList(), "1", false, "0")
        val page = WorkerStatusPage("project", "session", "b".repeat(64), 16, 17, 17, false, listOf(task))
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection, workerStatusesOpen = true, workerStatusesPage = page))
        val output = mutableListOf<String>(); val history = mutableListOf<String>()
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {},
            onWorkerTranscript = { id, next -> assertFalse(next); output.add(id) },
            onWorkerHistory = { id, next -> assertFalse(next); history.add(id) }) } }
        val outputButton = compose.onNodeWithContentDescription("Read listed task output completed-page")
        val historyButton = compose.onNodeWithContentDescription("Read listed task history completed-page")
        outputButton.assertIsEnabled().performClick(); historyButton.assertIsEnabled().performClick()
        assertEquals(listOf(task.requestId), output); assertEquals(listOf(task.requestId), history)
        val ready = state.value
        compose.runOnUiThread { state.value = ready.copy(workerStatusesPage = page.copy(nextOffset = 18)) }
        outputButton.assertIsNotEnabled(); historyButton.assertIsNotEnabled()
        compose.onNodeWithContentDescription("Refresh task list").assertIsEnabled()
        for (changed in listOf(ready.copy(scanInProgress = true), ready.copy(resetting = true), ready.copy(resetRequired = true),
            ready.copy(projection = projection.copy(projects = listOf(projection.projects.single(), projection.projects.single().copy(sessionId = "foreign")))))) {
            compose.runOnUiThread { state.value = changed }
            outputButton.assertIsNotEnabled(); historyButton.assertIsNotEnabled()
            compose.onNodeWithContentDescription("Refresh task list").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Next task list page").assertIsNotEnabled()
        }
        compose.runOnUiThread { state.value = ready.copy(workerStatusesError = "changed") }
        compose.onNodeWithContentDescription("Refresh task list").assertIsEnabled()
        outputButton.assertIsNotEnabled(); historyButton.assertIsNotEnabled()
        compose.runOnUiThread { state.value = ready }
        compose.runOnUiThread { state.value = state.value.copy(workerStatusesLoading = true) }
        outputButton.assertIsNotEnabled(); historyButton.assertIsNotEnabled()
        compose.runOnUiThread { state.value = state.value.copy(workerStatusesLoading = false, phase = PairingPhase.OFFLINE) }
        outputButton.assertIsNotEnabled(); historyButton.assertIsNotEnabled()
    }
}
