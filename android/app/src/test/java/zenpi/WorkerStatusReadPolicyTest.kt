package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class WorkerStatusReadPolicyTest {
    @Test fun invalidTaskMetadataCannotBecomeARetainedSelectionEvenWithConsistentPageCounts() {
        val metadata = listOf(task.copy(requestId = ""), task.copy(requestId = "x".repeat(129)), task.copy(requestId = "task\n"),
            task.copy(subtabId = "not-a-tab"), task.copy(state = "invented"), task.copy(failureCode = "busy"),
            task.copy(state = "failed", failureCode = "invented"), task.copy(runtimeActive = true),
            task.copy(outputTruncated = true), task.copy(streamTruncated = true), task.copy(outputWithheld = true))
        val pageOnly = projection.copy(projects = listOf(project.copy(workerStatuses = emptyList())))
        for (invalid in metadata) {
            val bad = page.copy(tasks = listOf(invalid))
            assertFalse(WorkerStatusPageCodec.sameScope(bad, projection))
            assertFalse(WorkerStatusPageCodec.validChain(emptyList(), bad))
            assertFalse(WorkerStatusReadPolicy.navigation(state.copy(workerStatusesPage = bad)))
            assertNull(WorkerReadPolicy.projection(state.copy(projection = pageOnly, workerStatusesPage = bad), invalid.requestId))
        }
        for (valid in listOf(task, task.copy(state = "failed", failureCode = "busy"),
            task.copy(state = "running", runtimeActive = true), task.copy(state = "unknown_outcome", runtimeActive = true))) {
            assertTrue(WorkerStatusPageCodec.sameScope(page.copy(tasks = listOf(valid)), projection))
        }
        val empty = page.copy(nextOffset = 0, totalTasks = 0, hasMore = false, tasks = emptyList())
        assertTrue(WorkerStatusPageCodec.validChain(emptyList(), empty))
        assertFalse(WorkerStatusPageCodec.validChain(emptyList(), empty.copy(offset = 1, nextOffset = 1, totalTasks = 1)))
        val result = com.google.gson.JsonParser.parseString("""{"project_id":"project","session_id":"session","digest":"${page.digest}","offset":1,"next_offset":1,"total_tasks":1,"has_more":false,"tasks":[]}""").asJsonObject
        val preceding = page.copy(totalTasks = 1, hasMore = false)
        assertTrue(runCatching { WorkerStatusPageCodec.decode(result, projection, preceding, 1) }.isFailure)
    }

    @Test fun malformedCurrentAndRetainedTaskPagesCannotAuthorizeNavigationOrContinuation() {
        for (bad in listOf(page.copy(digest = "invalid"), page.copy(offset = -1), page.copy(nextOffset = 2),
            page.copy(totalTasks = 0), page.copy(totalTasks = 4097), page.copy(hasMore = false),
            page.copy(tasks = emptyList()), page.copy(tasks = listOf(task.copy(sessionId = "foreign"))),
            page.copy(tasks = listOf(task.copy(outputText = "not metadata"))))) {
            assertFalse(WorkerStatusPageCodec.sameScope(bad, projection))
            assertFalse(WorkerStatusReadPolicy.navigation(state.copy(workerStatusesPage = bad)))
            assertTrue(WorkerStatusReadPolicy.browse(state.copy(workerStatusesPage = bad)))
            assertTrue(runCatching { WorkerStatusPageCodec.request(projection, bad, bad.nextOffset) }.isFailure)
            assertTrue(runCatching { WorkerStatusPageCodec.remember(emptyList(), null, bad) }.isFailure)
        }
        val second = page.copy(offset = 1, nextOffset = 2, totalTasks = 3, tasks = listOf(task.copy(requestId = "second")))
        val first = page.copy(totalTasks = 3)
        val third = second.copy(offset = 2, nextOffset = 3, hasMore = false, tasks = listOf(task.copy(requestId = "third")))
        assertTrue(WorkerStatusPageCodec.validChain(listOf(first), second))
        assertEquals(first, WorkerStatusPageCodec.previousPage(second, listOf(first), projection, 0))
        for (broken in listOf(listOf(first.copy(nextOffset = 2)), listOf(first.copy(digest = "c".repeat(64))),
            listOf(first.copy(tasks = second.tasks)), List(8) { first })) {
            assertFalse(WorkerStatusPageCodec.validChain(broken, second))
            assertNull(WorkerStatusPageCodec.previousPage(second, broken, projection, 0))
            assertFalse(WorkerStatusReadPolicy.navigation(state.copy(workerStatusesPage = second, workerStatusesPrevious = broken)))
            assertTrue(runCatching { WorkerStatusPageCodec.remember(broken, second, third) }.isFailure)
        }
    }

    private val task = RemoteWorkerStatus("task", "project", "session", "a".repeat(64), "succeeded")
    private val project = RemoteProject("project", "Workspace", "session", workerStatuses = listOf(task))
    private val projection = HostProjection(listOf(project), "project", "session", "running", emptyList(), "1", false, "0")
    private val page = WorkerStatusPage("project", "session", "b".repeat(64), 0, 1, 2, true, listOf(task))
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, workerStatusesOpen = true, workerStatusesPage = page)

    @Test fun taskReadsRemainAvailableWithoutControlAndFailedListsCanRefreshNotContinue() {
        assertTrue(WorkerStatusReadPolicy.browse(state)); assertTrue(WorkerStatusReadPolicy.navigation(state))
        assertNotNull(WorkerReadPolicy.projection(state, task.requestId))
        assertFalse(project.workerControl)
        for (changed in listOf(state.copy(workerStatusesOpen = false), state.copy(workerStatusesPage = null),
            state.copy(workerStatusesError = "changed"), state.copy(workerStatusesPage = page.copy(sessionId = "foreign")))) {
            assertTrue(WorkerStatusReadPolicy.browse(changed)); assertFalse(WorkerStatusReadPolicy.navigation(changed))
        }
    }

    @Test fun scanResetPendingAndMissingOrAmbiguousScopesRefuseListAndCompletedTaskReads() {
        val read = requireNotNull(WorkerReadPolicy.projection(state, task.requestId))
        val opened = state.copy(workerHistoryRequest = task.requestId, workerReadScope = WorkerReadPolicy.scope(read, task.requestId))
        for (changed in listOf(state.copy(scanInProgress = true), state.copy(resetting = true), state.copy(resetRequired = true),
            state.copy(phase = PairingPhase.OFFLINE), state.copy(pendingRevoke = "revoke"),
            state.copy(projection = null), state.copy(projection = projection.copy(selectedId = null)),
            state.copy(projection = projection.copy(sessionId = "foreign")), state.copy(projection = projection.copy(projects = emptyList())),
            state.copy(projection = projection.copy(projects = listOf(project, project.copy(sessionId = "foreign")))))) {
            assertFalse(WorkerStatusReadPolicy.browse(changed)); assertFalse(WorkerStatusReadPolicy.navigation(changed))
            assertNull(WorkerReadPolicy.projection(changed, task.requestId))
            assertNull(WorkerReadPolicy.projection(opened.copy(scanInProgress = changed.scanInProgress, resetting = changed.resetting,
                resetRequired = changed.resetRequired, phase = changed.phase, pendingRevoke = changed.pendingRevoke,
                projection = changed.projection), task.requestId))
        }
        val command = WorkspaceCommand.concurrency(projection.copy(workspaceOwner = "c".repeat(64), workspaceRevision = "1",
            projects = listOf(project.copy(workspaceControl = true)),
            subtabs = RemoteSubtabs("project", "1", task.subtabId, listOf(RemoteSubtab(task.subtabId, "main", "main", 1)), true,
                concurrencyControl = true)), task.subtabId, 2)
        assertFalse(WorkerStatusReadPolicy.browse(state.copy(pendingWorkspace = command)))
        assertNull(WorkerReadPolicy.projection(state.copy(pendingWorkspace = command), task.requestId))
    }

    @Test fun wireRequestsRefuseUnlistedForeignAndMixedDuplicateSelectionBeforeNetworkDispatch() {
        val result = com.google.gson.JsonParser.parseString("""{"project_id":"project","session_id":"session","digest":"${page.digest}","offset":0,"next_offset":0,"total_tasks":0,"has_more":false,"tasks":[]}""").asJsonObject
        for (changed in listOf(projection.copy(projects = emptyList()), projection.copy(sessionId = "foreign"),
            projection.copy(projects = listOf(project, project)), projection.copy(projects = listOf(project, project.copy(sessionId = "foreign"))))) {
            assertFalse(WorkerStatusPageCodec.readable(changed))
            assertTrue(runCatching { WorkerStatusPageCodec.request(changed, null, 0) }.isFailure)
            assertTrue(runCatching { WorkerStatusPageCodec.decode(result, changed, null, 0) }.isFailure)
            assertTrue(runCatching { WorkerHistoryCodec.request(changed, task.requestId, null) }.isFailure)
        }
        assertEquals(0, WorkerStatusPageCodec.decode(result, projection, null, 0).totalTasks)
    }
}
