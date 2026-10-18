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
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class WorkerScreenTest {
    @get:Rule val compose = createComposeRule()
    @Test fun unknownTaskCanBeCancelledOnlyWithLiveRuntimeEvidenceAndForegroundControls() {
        val project = "a".repeat(64); val tab = "b".repeat(64)
        val status = RemoteWorkerStatus("uncertain", project, "session", tab, "unknown_outcome", runtimeActive = true)
        val projection = HostProjection(listOf(RemoteProject(project, "Workspace", "session", workerControl = true, workerStatuses = listOf(status))),
            project, "session", "idle", emptyList(), "1", false, "0")
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection))
        val cancelled = mutableListOf<String>()
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onCancelWorker = { cancelled.add(it) })
        } } }
        compose.onNodeWithContentDescription("Worktree tasks").performScrollTo().performClick()
        compose.onNodeWithText("Task runtime active").performScrollTo().assertIsDisplayed()
        val button = compose.onNodeWithTag("Cancel worktree task uncertain")
        button.performScrollTo().performClick()
        assertEquals(listOf("uncertain"), cancelled)
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        button.assertIsNotEnabled()
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.CONNECTED, projection = projection.copy(
            projects = projection.projects.map { it.copy(workerStatuses = listOf(status.copy(runtimeActive = false))) })) }
        compose.onNodeWithTag("Cancel worktree task uncertain").assertDoesNotExist()
        compose.onNodeWithText("Task runtime active").assertDoesNotExist()
        compose.onNodeWithText("Task result unknown").performScrollTo().assertIsDisplayed()
    }
    @Test fun fullToolArgumentsOpenInOneDialogAndReturnToHistory() {
        val project = "a".repeat(64); val tab = "b".repeat(64)
        val call = RemoteToolCall("call-a", "read_file", "short preview", true)
        val turn = RemoteTurn("invocation", "assistant", "", toolCalls = listOf(call))
        val history = WorkerHistoryPage(project, "session", "done", tab, "c".repeat(64), 0, 1, 1, false, listOf(turn))
        val projection = HostProjection(listOf(RemoteProject(project, "Workspace", "session", workerStatuses = listOf(
            RemoteWorkerStatus("done", project, "session", tab, "succeeded")))), project, "session", "idle", emptyList(), "1", false, "0")
        val original = PairingState(phase = PairingPhase.CONNECTED, projection = projection, workerHistoryRequest = "done", workerHistoryPage = history)
        val state = mutableStateOf(original)
        val pages = mutableListOf<Pair<String, Boolean>>()
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onWorkerToolArguments = { id, callId ->
                assertEquals(turn.id, id); assertEquals(call.id, callId)
                state.value = state.value.copy(transcriptTurn = "done", transcriptWorker = true, transcriptHistoryTurn = turn, transcriptToolCall = call,
                    transcriptPage = TranscriptPage(project, "session", turn.id, "assistant", "d".repeat(64), 0, 3, 6, true, "abc", "done", tab, call.id, call.name))
            }, onWorkerTranscript = { id, next -> pages.add(id to next) }, onCloseTranscript = { state.value = WorkerHistoryNavigation.dismissMessage(state.value) })
        } } }
        for (bad in listOf(original.copy(resetRequired = true), original.copy(resetting = true),
            original.copy(workerHistoryPage = history.copy(turns = listOf(turn.copy(toolCallsTruncated = true)))),
            original.copy(workerHistoryPage = history.copy(turns = listOf(turn.copy(toolCalls = listOf(call.copy(argumentsPreview = "x".repeat(513))))))),
            original.copy(projection = projection.copy(sessionId = "foreign")), original.copy(workerHistoryPrevious = listOf(history)))) {
            compose.runOnUiThread { state.value = bad }
            compose.onNodeWithTag("Read tool arguments call-a").performScrollTo().assertIsNotEnabled()
        }
        compose.runOnUiThread { state.value = original }
        compose.onNodeWithTag("Read tool arguments call-a").performScrollTo().performClick()
        compose.onNodeWithText("Task history").assertDoesNotExist()
        compose.onNodeWithText("read_file").assertIsDisplayed()
        compose.onNodeWithTag("transcript-page").assertTextEquals("abc")
        compose.onNodeWithContentDescription("Next message page").performClick()
        assertEquals(listOf("done" to true), pages)
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithContentDescription("Next message page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Close message").performClick()
        compose.onNodeWithText("Task history").assertIsDisplayed()
        compose.onNodeWithTag("Read tool arguments call-a").assertIsNotEnabled()
        assertNull(state.value.transcriptToolCall)
    }
    @Test fun emptyAssistantMessageStillShowsToolInvocationInNarrowHistory() {
        val project = "a".repeat(64); val tab = "b".repeat(64)
        val turn = RemoteTurn("call-message", "assistant", "", toolCalls = listOf(
            RemoteToolCall("read-task-file", "read_file", "{\"path\":\"evidence.txt\"}", false)))
        val history = WorkerHistoryPage(project, "session", "done", tab, "c".repeat(64), 0, 1, 1, false, listOf(turn))
        val projection = HostProjection(listOf(RemoteProject(project, "Workspace", "session", workerStatuses = listOf(
            RemoteWorkerStatus("done", project, "session", tab, "succeeded")))), project, "session", "idle", emptyList(), "1", false, "0")
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(PairingState(phase = PairingPhase.CONNECTED, projection = projection, workerHistoryRequest = "done", workerHistoryPage = history), {}, {}, {})
        } } }
        compose.onNodeWithText("read_file").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("{\"path\":\"evidence.txt\"}").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("Read task message call-message").performScrollTo().assertIsEnabled()
    }
    @Test fun fullTaskMessageDismissalRestoresHistoryAndDoesNotStackDialogs() {
        val project = "a".repeat(64); val tab = "b".repeat(64)
        val message = RemoteTurn("tool-message", "tool", "Short tool preview", true)
        val history = WorkerHistoryPage(project, "session", "done", tab, "c".repeat(64), 4, 5, 8, true, listOf(message))
        val projection = HostProjection(listOf(RemoteProject(project, "Workspace", "session", workerStatuses = listOf(
            RemoteWorkerStatus("done", project, "session", tab, "succeeded")))), project, "session", "idle", emptyList(), "1", false, "0")
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection,
            workerHistoryRequest = "done", workerHistoryPage = history))
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onWorkerHistoryTurn = { id ->
                assertEquals(message.id, id)
                state.value = state.value.copy(transcriptTurn = "done", transcriptWorker = true, transcriptHistoryTurn = message,
                    transcriptPage = TranscriptPage(project, "session", message.id, "tool", "d".repeat(64), 0, 14, 14, false, "Full tool text", "done", tab))
            }, onCloseTranscript = { state.value = WorkerHistoryNavigation.dismissMessage(state.value) })
        } } }
        compose.onNodeWithTag("Read task message tool-message").performScrollTo().performClick()
        compose.onNodeWithText("Task history").assertDoesNotExist()
        compose.onNodeWithTag("transcript-page").assertTextEquals("Full tool text")
        compose.onNodeWithContentDescription("Close message").performClick()
        compose.onNodeWithText("Task history").assertIsDisplayed()
        compose.onNodeWithText("4 - 5 / 8").assertIsDisplayed()
        compose.onNodeWithTag("transcript-page").assertDoesNotExist()
        assertEquals(history, state.value.workerHistoryPage)
    }
    @Test fun taskHistoryShowsRolesRoutesMessagesAndDisablesPagingOffline() {
        val project = "a".repeat(64); val tab = "b".repeat(64)
        val page = WorkerHistoryPage(project, "session", "done", tab, "c".repeat(64), 0, 2, 3, true,
            listOf(RemoteTurn("input", "user", "Original task"), RemoteTurn("tool", "tool", "Tool result", true)))
        val projection = HostProjection(listOf(RemoteProject(project, "Workspace", "session", workerStatuses = listOf(
            RemoteWorkerStatus("done", project, "session", tab, "succeeded")))), project, "session", "idle", emptyList(), "1", false, "0")
        val original = PairingState(phase = PairingPhase.CONNECTED, projection = projection, workerHistoryRequest = "done", workerHistoryPage = page)
        val state = mutableStateOf(original)
        val messages = mutableListOf<String>(); val advances = mutableListOf<Pair<String, Boolean>>()
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onWorkerHistoryTurn = { messages.add(it) }, onWorkerHistory = { id, next -> advances.add(id to next) })
        } } }
        compose.onNodeWithText("Tool result").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("Read task message tool").performScrollTo().performClick()
        assertEquals(listOf("tool"), messages)
        compose.onNodeWithContentDescription("Next task history page").performClick()
        assertEquals(listOf("done" to true), advances)
        for (bad in listOf(original.copy(phase = PairingPhase.OFFLINE), original.copy(resetting = true), original.copy(resetRequired = true),
            original.copy(projection = projection.copy(sessionId = "foreign")), original.copy(workerHistoryRequest = "foreign"),
            original.copy(workerHistoryPage = page.copy(digest = "bad")), original.copy(workerHistoryPrevious = listOf(page)))) {
            compose.runOnUiThread { state.value = bad }
            compose.onNodeWithContentDescription("Next task history page").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Previous task history page").assertIsNotEnabled()
            compose.onNodeWithTag("Read task message tool").assertIsNotEnabled()
        }
        assertEquals(listOf("tool"), messages); assertEquals(listOf("done" to true), advances)
    }
    @Test fun completedWorkerOutputOpensAndPagesThroughWorkerRouteNotMasterRoute() {
        val project = "a".repeat(64); val tab = "b".repeat(64)
        val projection = HostProjection(listOf(RemoteProject(project, "Workspace", "session", workerStatuses = listOf(
            RemoteWorkerStatus("done", project, "session", tab, "succeeded", outputText = "preview", outputTruncated = true)))),
            project, "session", "idle", emptyList(), "1", false, "0")
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection))
        val calls = mutableListOf<Pair<String, Boolean>>()
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onWorkerTranscript = { id, next -> calls.add(id to next) },
                onTranscript = { _, _ -> error("worker page used master route") })
        } } }
        compose.onNodeWithContentDescription("Worktree tasks").performScrollTo().performClick()
        val open = compose.onNodeWithTag("Read task output done")
        open.performScrollTo().performClick()
        assertEquals(listOf("done" to false), calls)
        compose.runOnUiThread { state.value = state.value.copy(transcriptTurn = "done", transcriptWorker = true,
            transcriptPage = TranscriptPage(project, "session", "answer", "assistant", "c".repeat(64), 0, 3, 6, true, "abc", "done", tab)) }
        compose.onNodeWithTag("transcript-page").assertTextEquals("abc")
        compose.onNodeWithContentDescription("Next message page").performClick()
        assertEquals(listOf("done" to false, "done" to true), calls)
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithContentDescription("Next message page").assertIsNotEnabled()
    }
    @Test fun quotaStepperUsesConfirmedValuesAndFencesPendingOfflineAndCachedStates() {
        val project = "a".repeat(64)
        val tab = "b".repeat(64)
        val live = HostProjection(listOf(RemoteProject(project, "Workspace", "session", workspaceControl = true)),
            project, "session", "idle", emptyList(), "1", false, "0", workspaceOwner = "c".repeat(64), workspaceRevision = "1",
            subtabs = RemoteSubtabs(project, "0", tab, listOf(RemoteSubtab(tab, "main", "main", 1)), true, concurrencyControl = true))
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = live))
        val changes = mutableListOf<Pair<String, Int>>()
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onConcurrency = { id, count -> changes.add(id to count) })
        } } }
        val decrease = compose.onNodeWithContentDescription("Decrease worktree concurrency")
        val increase = compose.onNodeWithContentDescription("Increase worktree concurrency")
        decrease.performScrollTo().assertIsNotEnabled()
        increase.performScrollTo().assertIsEnabled().performClick().performClick()
        assertEquals(listOf(tab to 2, tab to 2), changes)
        compose.runOnUiThread { state.value = state.value.copy(pendingWorkspace = WorkspaceCommand.concurrency(live, tab, 2)) }
        increase.assertIsNotEnabled(); decrease.assertIsNotEnabled()
        compose.runOnUiThread { state.value = state.value.copy(pendingWorkspace = null, projection = live.copy(
            subtabs = live.subtabs!!.copy(tabs = listOf(RemoteSubtab(tab, "main", "main", 64))))) }
        increase.assertIsNotEnabled(); decrease.assertIsEnabled().performClick()
        assertEquals(tab to 63, changes.last())
        for (phase in listOf(PairingPhase.ERROR, PairingPhase.OFFLINE)) {
            compose.runOnUiThread { state.value = state.value.copy(phase = phase) }
            increase.assertIsNotEnabled(); decrease.assertIsNotEnabled()
        }
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.CONNECTED, projection = live.copy(
            projects = live.projects.map { it.copy(workerStatuses = listOf(RemoteWorkerStatus("busy", project, "session", tab, "running"))) })) }
        increase.assertIsEnabled(); decrease.assertIsNotEnabled()
        compose.runOnUiThread { state.value = AuthorityLossPolicy.displayOnly(state.value) }
        increase.assertDoesNotExist(); decrease.assertDoesNotExist()
        assertEquals(3, changes.size)
    }
    @Test fun completedOutputAndWithheldStatesAreVisibleWithoutExecutionControls() {
        val project = "a".repeat(64)
        val tab = "b".repeat(64)
        val projection = HostProjection(listOf(RemoteProject(project, "Workspace", "session", workerStatuses = listOf(
            RemoteWorkerStatus("done", project, "session", tab, "succeeded", outputText = "Verified task output", outputTruncated = true),
            RemoteWorkerStatus("withheld", project, "session", tab, "succeeded", outputWithheld = true)))),
            project, "session", "idle", emptyList(), "1", false, "0")
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(PairingState(phase = PairingPhase.CONNECTED, projection = projection), {}, {}, {})
        } } }
        compose.onNodeWithContentDescription("Worktree tasks").performScrollTo().performClick()
        compose.onNodeWithText("Verified task output").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Task output truncated").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Task output withheld").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("Cancel worktree task done").assertDoesNotExist()
        compose.onNodeWithContentDescription("Send worktree task").assertDoesNotExist()
    }
    @Test fun workerSubmissionAndCancellationAreSeparateFromMasterAndLoseAuthorityOffline() {
        val project = "a".repeat(64)
        val tab = "b".repeat(64)
        val live = HostProjection(listOf(RemoteProject(project, "Workspace", "session", workerControl = true,
            workerStatuses = listOf(RemoteWorkerStatus("running-worker", project, "session", tab, "running", streamText = "Live task output", streamTruncated = true)))),
            project, "session", "idle", emptyList(), "1", false, "0",
            subtabs = RemoteSubtabs(project, "0", tab, listOf(RemoteSubtab(tab, "main", "main", 2)), false))
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = live))
        val sent = mutableListOf<String>()
        val cancelled = mutableListOf<String>()
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onWorkerPrompt = { sent.add(it) }, onCancelWorker = { cancelled.add(it) })
        } } }
        compose.onNodeWithContentDescription("Send message").assertDoesNotExist()
        compose.onNodeWithContentDescription("Worktree tasks").performScrollTo().performClick()
        compose.onNodeWithText("Task running").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Live task output").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Task preview truncated").performScrollTo().assertIsDisplayed()
        compose.onNodeWithTag("Cancel worktree task running-worker").performScrollTo().performClick()
        assertEquals(listOf("running-worker"), cancelled)
        compose.onNodeWithText("Task: main").performTextInput("review the worktree")
        compose.onNodeWithContentDescription("Send worktree task").assertIsEnabled().performClick()
        assertEquals(listOf("review the worktree"), sent)
        compose.runOnUiThread { state.value = state.value.copy(projection = live.copy(eventSequence = "1")) }
        compose.onNodeWithText("Task: main").assertTextContains("review the worktree")
        val revised = live.copy(subtabs = live.subtabs?.copy(revision = "1"))
        compose.runOnUiThread { state.value = state.value.copy(projection = revised) }
        compose.onNodeWithContentDescription("Send worktree task").assertIsNotEnabled()
        compose.onNodeWithText("Task: main").performTextInput("new scope task")
        compose.onNodeWithContentDescription("Send worktree task").assertIsDisplayed().assertIsEnabled().performClick()
        compose.runOnIdle { assertEquals(listOf("review the worktree", "new scope task"), sent) }
        val tabs = requireNotNull(revised.subtabs)
        val other = RemoteSubtab("d".repeat(64), "other", "in_place", 1)
        for (invalid in listOf(revised.copy(subtabs = tabs.copy(tabs = tabs.tabs + other + other)),
            revised.copy(subtabs = tabs.copy(activeId = other.id)), revised.copy(sessionId = "foreign"))) {
            compose.runOnUiThread { state.value = state.value.copy(projection = invalid) }
            compose.onNodeWithContentDescription("Send worktree task").assertDoesNotExist()
        }
        compose.runOnUiThread { state.value = state.value.copy(projection = revised.copy(subtabs = tabs.copy(tabs = tabs.tabs + other, activeId = other.id))) }
        // The foreign session above returned to the conversation (pages are scoped to the session).
        compose.onNodeWithContentDescription("Worktree tasks").performScrollTo().performClick()
        compose.onNodeWithText("Task: other").performTextInput("review other tab")
        compose.onNodeWithContentDescription("Send worktree task").performClick()
        assertEquals("review other tab", sent.last())
        compose.onNodeWithTag("Cancel worktree task running-worker").performScrollTo().assertIsEnabled()
        compose.runOnUiThread { state.value = state.value.copy(projection = revised) }
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.ERROR) }
        compose.onNodeWithContentDescription("Send worktree task").assertIsNotEnabled()
        compose.onNodeWithTag("Cancel worktree task running-worker").performScrollTo().assertIsEnabled()
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithTag("Cancel worktree task running-worker").assertIsNotEnabled()
        compose.runOnUiThread { state.value = AuthorityLossPolicy.displayOnly(state.value) }
        compose.onNodeWithContentDescription("Send worktree task").assertDoesNotExist()
        compose.onNodeWithTag("Cancel worktree task running-worker").assertDoesNotExist()
        assertEquals(listOf("review the worktree", "new scope task", "review other tab"), sent)
    }
}
