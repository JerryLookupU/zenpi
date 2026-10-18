package zenpi

import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class WorkerReadPolicyTest {
    private val task = RemoteWorkerStatus("page-only", "project", "session", "a".repeat(64), "succeeded")
    private val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session")),
        "project", "session", "idle", emptyList(), "1", false, "0", workspaceOwner = "b".repeat(64), workspaceRevision = "3")
    private val page = WorkerStatusPage("project", "session", "c".repeat(64), 16, 17, 17, false, listOf(task))
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, workerStatusesOpen = true, workerStatusesPage = page)
    @Test fun malformedSummaryOrSavedScopeCannotAuthorizeHistoryOrDetailRequests() {
        fun display(target: RemoteWorkerStatus) = projection.copy(projects = projection.projects.map { it.copy(workerStatuses = listOf(target)) })
        val healthy = display(task)
        val scope = WorkerReadPolicy.scope(healthy, task.requestId)
        val duplicate = healthy.copy(projects = healthy.projects.map { it.copy(workerStatuses = listOf(task, task.copy(state = "running"))) })
        assertFalse(TranscriptPageCodec.retainsTurn(duplicate, duplicate, task.requestId, true))
        assertNull(WorkerReadPolicy.projection(state.copy(projection = duplicate), task.requestId))
        assertTrue(runCatching { WorkerReadPolicy.scope(duplicate, task.requestId) }.isFailure)
        for (bad in listOf(task.copy(subtabId = "bad"), task.copy(projectId = "foreign"), task.copy(sessionId = "foreign"),
            task.copy(failureCode = "internal_error"), task.copy(runtimeActive = true), task.copy(state = "unknown"),
            task.copy(outputText = "x".repeat(4097)), task.copy(outputText = "bad\u0000text"),
            task.copy(outputTruncated = true), task.copy(outputText = "answer", outputWithheld = true),
            task.copy(streamText = "late"), task.copy(streamTruncated = true))) {
            val malformed = display(bad)
            assertNull(WorkerReadPolicy.projection(state.copy(projection = malformed), task.requestId))
            assertFalse(TranscriptPageCodec.retainsTurn(malformed, malformed, task.requestId, true))
            assertTrue(runCatching { WorkerReadPolicy.scope(malformed, task.requestId) }.isFailure)
            assertTrue(runCatching { WorkerHistoryCodec.request(malformed, task.requestId, null) }.isFailure)
            assertTrue(runCatching { TranscriptPageCodec.request(malformed, task.requestId, null, true) }.isFailure)
            val saved = state.copy(workerStatusesOpen = false, workerStatusesPage = null, transcriptWorker = true,
                transcriptTurn = task.requestId, workerReadScope = scope.copy(target = bad))
            assertNull(WorkerReadPolicy.projection(saved, task.requestId))
            assertNull(WorkerReadPolicy.projection(saved.copy(projection = healthy), task.requestId))
        }
    }
    @Test fun validOutputPreviewAndWithheldSummaryStillPermitReadOnlyHistoryAndDetail() {
        for (valid in listOf(task, task.copy(outputText = "x".repeat(4096), outputTruncated = true), task.copy(outputWithheld = true))) {
            val current = projection.copy(projects = projection.projects.map { it.copy(workerStatuses = listOf(valid)) })
            val read = requireNotNull(WorkerReadPolicy.projection(state.copy(projection = current), task.requestId))
            assertTrue(TranscriptPageCodec.retainsTurn(read, read, task.requestId, true))
            FrameCodec.encode(WorkerHistoryCodec.request(read, task.requestId, null))
            FrameCodec.encode(TranscriptPageCodec.request(read, task.requestId, null, true))
            assertEquals(valid, WorkerReadPolicy.scope(read, task.requestId).target)
            assertFalse(WorkerStatusCodec.validMetadata(valid, "project", "session") && valid != task)
        }
    }
    @Test fun pageOnlyCompletedTaskBuildsReadRequestsWithoutMutatingDisplayCacheOrWriteAuthority() {
        val read = requireNotNull(WorkerReadPolicy.projection(state, task.requestId))
        val output = TranscriptPageCodec.request(read, task.requestId, null, true)
        val history = WorkerHistoryCodec.request(read, task.requestId, null)
        assertEquals(task.requestId, output.getAsJsonObject("payload").getAsJsonObject("arguments")["request_id"].asString)
        assertEquals("history", history.getAsJsonObject("payload").getAsJsonObject("arguments")["view"].asString)
        assertTrue(state.projection!!.projects.single().workerStatuses.isEmpty())
        assertEquals(zenpi.state.ProjectionCacheCodec.encode(projection), zenpi.state.ProjectionCacheCodec.encode(read))
        assertTrue(runCatching { WorkspaceCommand.cancelWorker(read, task) }.isFailure)
        assertFalse(read.projects.single().workerControl)
        val receipt = JsonParser.parseString("""{"project_id":"project","session_id":"session","request_id":"page-only","subtab_id":"${task.subtabId}","turn_id":"answer","role":"assistant","digest":"${"d".repeat(64)}","offset":0,"next_offset":3,"total_bytes":3,"has_more":false,"text":"abc"}""").asJsonObject
        assertEquals("abc", TranscriptPageCodec.decode(receipt, read, task.requestId, null, true).text)
        assertTrue(runCatching { TranscriptPageCodec.decode(receipt.deepCopy().apply { addProperty("subtab_id", "e".repeat(64)) }, read, task.requestId, null, true) }.isFailure)
    }
    @Test fun readScopeSurvivesClosingListButNeverHostOwnerRevisionSessionOrAuthorityLoss() {
        val read = requireNotNull(WorkerReadPolicy.projection(state, task.requestId))
        val opened = state.copy(workerStatusesOpen = false, workerStatusesPage = null, transcriptTurn = task.requestId,
            transcriptWorker = true, workerReadScope = WorkerReadPolicy.scope(read, task.requestId))
        assertEquals(read, WorkerReadPolicy.projection(opened, task.requestId))
        assertNull(WorkerReadPolicy.projection(opened, task.requestId, projection.copy(projects = projection.projects.map {
            it.copy(workerStatuses = listOf(task.copy(subtabId = "e".repeat(64))))
        })))
        for (next in listOf(projection.copy(workspaceOwner = "e".repeat(64)), projection.copy(workspaceRevision = "4"),
            projection.copy(sessionId = "replacement"), projection.copy(selectedId = "foreign"))) {
            assertNull(WorkerReadPolicy.projection(opened, task.requestId, next))
        }
        for (next in listOf(opened.copy(phase = PairingPhase.OFFLINE), opened.copy(phase = PairingPhase.ERROR),
            opened.copy(scanInProgress = true), opened.copy(resetting = true), opened.copy(resetRequired = true), opened.copy(pendingRevoke = "revoke"),
            opened.copy(transcriptTurn = null), opened.copy(transcriptWorker = false))) {
            assertNull(WorkerReadPolicy.projection(next, task.requestId))
        }
        assertNull(AuthorityLossPolicy.displayOnly(opened).workerReadScope)
        assertNull(WorkerHistoryNavigation.dismissMessage(opened).workerReadScope)
    }
    @Test fun failedLoadingForeignDuplicateOrNewerSummaryTaskCannotSupplyCompletedRead() {
        for (changed in listOf(state.copy(workerStatusesLoading = true), state.copy(workerStatusesError = "changed"),
            state.copy(workerStatusesOpen = false), state.copy(workerStatusesPage = page.copy(sessionId = "foreign")),
            state.copy(workerStatusesPage = page.copy(tasks = listOf(task, task))),
            state.copy(workerStatusesPage = page.copy(tasks = listOf(task.copy(state = "running")))),
            state.copy(workerStatusesPage = page.copy(tasks = listOf(task.copy(projectId = "foreign")))),
            state.copy(projection = projection.copy(projects = projection.projects.map { it.copy(workerStatuses = listOf(task.copy(state = "failed"))) })) )) {
            assertNull(WorkerReadPolicy.projection(changed, task.requestId))
        }
    }
    @Test fun selectedHistoryMessageCanReturnToPageOnlyHistoryWithoutKeepingMessageContent() {
        val read = requireNotNull(WorkerReadPolicy.projection(state, task.requestId))
        val turn = RemoteTurn("turn", "tool", "result")
        val history = WorkerHistoryPage("project", "session", task.requestId, task.subtabId, "d".repeat(64), 0, 1, 1, false, listOf(turn))
        val opened = state.copy(workerStatusesOpen = false, workerStatusesPage = null, workerHistoryRequest = task.requestId,
            workerHistoryPage = history, transcriptTurn = task.requestId, transcriptWorker = true, transcriptHistoryTurn = turn,
            workerReadScope = WorkerReadPolicy.scope(read, task.requestId))
        val restored = WorkerHistoryNavigation.dismissMessage(opened)
        assertEquals(history, restored.workerHistoryPage); assertNotNull(restored.workerReadScope)
        assertNull(restored.transcriptTurn); assertNull(restored.transcriptHistoryTurn)
        assertNotNull(WorkerReadPolicy.projection(restored, task.requestId))
    }
    @Test fun fragmentedPageOnlyHistoryReceiptOpensBoundedReaderWithoutPublishingTaskAuthority() = kotlinx.coroutines.runBlocking {
        val read = requireNotNull(WorkerReadPolicy.projection(state, task.requestId))
        val request = WorkerHistoryCodec.request(read, task.requestId, null)
        val result = JsonParser.parseString("""{"project_id":"project","session_id":"session","request_id":"page-only","subtab_id":"${task.subtabId}","digest":"${"d".repeat(64)}","offset":0,"next_offset":2,"total_turns":2,"has_more":false,"turns":[{"id":"input","role":"user","content":"page task","content_truncated":false},{"id":"answer","role":"assistant","content":"completed output","content_truncated":false}]}""").asJsonObject
        fun wire(value: com.google.gson.JsonObject) = java.io.ByteArrayOutputStream().also { output ->
            FrameCodec.write(output, JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"${request["id"].asString}","execution_state":"succeeded","payload":{"type":"response","result":{}}}""").asJsonObject.apply {
                getAsJsonObject("payload").add("result", value)
            })
        }.toByteArray()
        val bytes = wire(result)
        val input = object : java.io.ByteArrayInputStream(bytes) {
            override fun read(buffer: ByteArray, offset: Int, length: Int) = super.read(buffer, offset, minOf(length, 3))
        }
        val response = ResponseReader.read(input, request, {})
        val history = WorkerHistoryCodec.decode(response.result, read, task.requestId, null)
        assertEquals(listOf("page task", "completed output"), history.turns.map { it.content })
        val opened = state.copy(workerStatusesOpen = false, workerStatusesPage = null,
            workerHistoryRequest = task.requestId, workerHistoryPage = history, workerReadScope = WorkerReadPolicy.scope(read, task.requestId))
        assertTrue(WorkerHistoryNavigation.validScope(history, WorkerReadPolicy.projection(opened, task.requestId)))
        assertEquals(projection, opened.projection); assertNull(opened.pendingWorkspace)
        val before = zenpi.state.ProjectionCacheCodec.encode(projection)
        assertEquals(before, zenpi.state.ProjectionCacheCodec.encode(requireNotNull(opened.projection)))
        for (field in listOf("project_id", "session_id", "request_id", "subtab_id")) {
            val malformed = result.deepCopy().apply { addProperty(field, "foreign") }
            val parsed = ResponseReader.read(java.io.ByteArrayInputStream(wire(malformed)), request, {})
            assertTrue(runCatching { WorkerHistoryCodec.decode(parsed.result, read, task.requestId, null) }.isFailure)
            assertEquals(history, opened.workerHistoryPage)
        }
    }
}
