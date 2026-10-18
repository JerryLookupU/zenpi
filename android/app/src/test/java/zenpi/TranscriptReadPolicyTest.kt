package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class TranscriptReadPolicyTest {
    @Test fun malformedLiveAndWorkerSummariesRefuseInitialReadsContinuationsAndRetainedSelection() {
        val firstTool = first.copy(toolCallId = call.id, toolName = call.name)
        val opened = state.copy(transcriptToolCall = call, transcriptPage = second.copy(toolCallId = call.id, toolName = call.name),
            transcriptPreviousPages = listOf(firstTool))
        val task = RemoteWorkerStatus("task", "project", "session", "b".repeat(64), "succeeded")
        val workerProjection = projection.copy(projects = listOf(projection.projects.single().copy(workerStatuses = listOf(task))))
        val workerFirst = firstTool.copy(workerRequestId = task.requestId, subtabId = task.subtabId)
        val candidates = listOf(turn.copy(toolCallsTruncated = true), turn.copy(toolCalls = listOf(call, call)),
            turn.copy(toolCalls = listOf(call.copy(name = "bad\n"))),
            turn.copy(toolCalls = listOf(call.copy(argumentsPreview = "x".repeat(513)))),
            turn.copy(toolCalls = listOf(call.copy(argumentsPreview = "bad\u0000"))),
            turn.copy(toolCalls = listOf(call.copy(id = "x".repeat(129)))))
        for (bad in candidates) {
            val selectedCall = bad.toolCalls.first()
            val changed = projection.copy(turns = listOf(bad))
            val live = state.copy(projection = changed, transcriptTurn = null)
            assertTrue(TranscriptReadPolicy.browse(live))
            assertFalse(TranscriptReadPolicy.tool(live, bad.id, selectedCall.id))
            assertFalse(TranscriptPageCodec.retainsTurn(changed, changed, bad.id))
            assertFalse(TranscriptReadPolicy.navigation(opened.copy(projection = changed, transcriptToolCall = selectedCall)))
            for (previous in listOf(null, firstTool)) assertTrue(runCatching {
                TranscriptPageCodec.request(changed, bad.id, previous, toolCall = selectedCall)
            }.isFailure)
            for (previous in listOf(null, workerFirst)) assertTrue(runCatching {
                TranscriptPageCodec.request(workerProjection, task.requestId, previous, true, bad, selectedCall)
            }.isFailure)
            val worker = opened.copy(projection = workerProjection, transcriptTurn = task.requestId, transcriptWorker = true,
                transcriptHistoryTurn = bad, transcriptToolCall = selectedCall,
                transcriptPage = opened.transcriptPage?.copy(workerRequestId = task.requestId, subtabId = task.subtabId),
                transcriptPreviousPages = listOf(workerFirst))
            assertFalse(TranscriptReadPolicy.navigation(worker))
        }
        val ready = state.copy(transcriptTurn = null)
        assertTrue(TranscriptReadPolicy.tool(ready, turn.id, call.id))
        assertFalse(TranscriptReadPolicy.tool(ready.copy(scanInProgress = true), turn.id, call.id))
        val bounded = turn.copy(toolCalls = (0..7).map { call.copy(id = "call-$it") }, toolCallsTruncated = true)
        assertTrue(TranscriptReadPolicy.tool(ready.copy(projection = projection.copy(turns = listOf(bounded))), turn.id, "call-0"))
    }

    private val call = RemoteToolCall("call", "read_file", "preview", true)
    private val turn = RemoteTurn("turn", "assistant", "prefix", true, toolCalls = listOf(call))
    private val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session")), "project", "session", "idle",
        listOf(turn), "3", false, "1")
    private val first = TranscriptPage("project", "session", "turn", "assistant", "a".repeat(64), 0, 3, 9, true, "abc")
    private val second = first.copy(offset = 3, nextOffset = 6, text = "def")
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, transcriptTurn = "turn",
        transcriptPage = second, transcriptPreviousPages = listOf(first))

    @Test fun exactMessageAndToolSelectionAllowBoundedHistory() {
        assertTrue(TranscriptReadPolicy.navigation(state)); assertEquals(first, TranscriptReadPolicy.previous(state))
        val tool = state.copy(transcriptToolCall = call, transcriptPage = second.copy(toolCallId = call.id, toolName = call.name),
            transcriptPreviousPages = listOf(first.copy(toolCallId = call.id, toolName = call.name)))
        assertTrue(TranscriptReadPolicy.navigation(tool))
        assertFalse(TranscriptReadPolicy.navigation(tool.copy(transcriptToolCall = call.copy(name = "different"))))
        assertFalse(TranscriptReadPolicy.navigation(tool.copy(projection = projection.copy(turns = listOf(turn.copy(toolCalls = emptyList()))))))
        assertFalse(TranscriptReadPolicy.navigation(state.copy(transcriptPage = second.copy(toolCallId = call.id, toolName = call.name))))
    }
    @Test fun foreignRemovedRoleChangedOrBrokenHistoryNeverNavigates() {
        for (invalid in listOf(state.copy(transcriptTurn = "foreign"), state.copy(projection = projection.copy(turns = emptyList())),
            state.copy(projection = projection.copy(projects = emptyList())), state.copy(projection = projection.copy(sessionId = "foreign")),
            state.copy(transcriptPage = second.copy(role = "user")), state.copy(transcriptPage = second.copy(projectId = "foreign")),
            state.copy(transcriptPage = second.copy(workerRequestId = "worker")), state.copy(transcriptPage = second.copy(subtabId = "tab")),
            state.copy(transcriptPreviousPages = listOf(first.copy(hasMore = false))), state.copy(transcriptPreviousPages = listOf(first, first)),
            state.copy(transcriptPage = second.copy(text = "invalid size")), state.copy(transcriptPage = second.copy(digest = "bad")),
            state.copy(transcriptPage = second.copy(nextOffset = 9)), state.copy(transcriptPage = second.copy(totalBytes = -1)))) {
            assertFalse(TranscriptReadPolicy.navigation(invalid)); assertNull(TranscriptReadPolicy.previous(invalid))
        }
        for (invalid in listOf(state.copy(phase = PairingPhase.OFFLINE), state.copy(resetting = true), state.copy(resetRequired = true),
            state.copy(pendingRevoke = "unknown"), state.copy(transcriptLoading = true), state.copy(transcriptError = "changed"))) {
            assertFalse(TranscriptReadPolicy.navigation(invalid)); assertNull(TranscriptReadPolicy.previous(invalid))
        }
    }
    @Test fun workerRouteRequiresExactSucceededTaskSubtabAndHistoryTurn() {
        val task = RemoteWorkerStatus("task", "project", "session", "b".repeat(64), "succeeded")
        val workerProjection = projection.copy(projects = listOf(projection.projects.single().copy(workerStatuses = listOf(task))))
        val worker = state.copy(projection = workerProjection, transcriptWorker = true, transcriptTurn = task.requestId,
            transcriptHistoryTurn = turn, transcriptPage = second.copy(workerRequestId = task.requestId, subtabId = task.subtabId),
            transcriptPreviousPages = listOf(first.copy(workerRequestId = task.requestId, subtabId = task.subtabId)))
        assertTrue(TranscriptReadPolicy.navigation(worker))
        for (invalid in listOf(worker.copy(transcriptPage = worker.transcriptPage?.copy(subtabId = "foreign")),
            worker.copy(transcriptHistoryTurn = turn.copy(id = "other")), worker.copy(transcriptHistoryTurn = turn.copy(role = "tool")),
            worker.copy(projection = workerProjection.copy(projects = listOf(workerProjection.projects.single().copy(workerStatuses = listOf(task.copy(state = "failed")))))))) {
            assertFalse(TranscriptReadPolicy.navigation(invalid))
        }
    }
    @Test fun scannerScopeAndPendingWritesFenceInitialReadsAndAllPageRoutes() {
        val pending = WorkspaceCommand.prompt(projection.copy(projects = listOf(projection.projects.single().copy(promptControl = true)),
            workspaceOwner = "b".repeat(64), workspaceRevision = "1"), "pending")
        for (invalid in listOf(state.copy(scanInProgress = true), state.copy(resetRequired = true), state.copy(resetting = true),
            state.copy(phase = PairingPhase.OFFLINE), state.copy(transcriptLoading = true), state.copy(pendingWorkspace = pending),
            state.copy(pendingRevoke = "pending"), state.copy(projection = null), state.copy(projection = projection.copy(projects = emptyList())),
            state.copy(projection = projection.copy(projects = projection.projects + projection.projects)),
            state.copy(projection = projection.copy(sessionId = "foreign")))) {
            assertFalse(TranscriptReadPolicy.browse(invalid)); assertFalse(TranscriptReadPolicy.navigation(invalid))
            assertNull(TranscriptReadPolicy.previous(invalid))
        }
        for (invalid in listOf(projection.copy(projects = emptyList()), projection.copy(projects = projection.projects + projection.projects),
            projection.copy(sessionId = "foreign"))) {
            assertTrue(runCatching { TranscriptPageCodec.request(invalid, turn.id, null) }.isFailure)
        }
        val workerProjection = projection.copy(projects = listOf(projection.projects.single().copy(workerStatuses =
            listOf(RemoteWorkerStatus("task", "project", "session", "b".repeat(64), "succeeded")))))
        assertFalse(TranscriptReadPolicy.browse(state.copy(projection = workerProjection, scanInProgress = true)))
    }
    @Test fun initialReadRemainsAvailableAfterPageFailureWithoutBorrowingWriteCapability() {
        assertFalse(projection.projects.single().promptControl)
        assertTrue(TranscriptReadPolicy.browse(state.copy(transcriptError = "changed")))
        assertFalse(TranscriptReadPolicy.navigation(state.copy(transcriptError = "changed")))
        val running = projection.copy(projects = listOf(projection.projects.single().copy(promptStatuses =
            listOf(RemotePromptStatus("request", "project", "session", "running")))))
        assertTrue(TranscriptReadPolicy.browse(state.copy(projection = running)))
        assertTrue(TranscriptReadPolicy.navigation(state.copy(projection = running)))
    }
}
