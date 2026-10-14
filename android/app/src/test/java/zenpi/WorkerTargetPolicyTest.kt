package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class WorkerTargetPolicyTest {
    @Test fun malformedSummaryCannotCreateOrAdmitFreshCancelButExactSavedRetryRetainsIdentity() {
        val command = WorkspaceCommand.cancelWorker(projection, task)
        for (bad in listOf(task.copy(subtabId = "invalid"), task.copy(failureCode = "busy"),
            task.copy(outputText = "not completed"), task.copy(outputWithheld = true), task.copy(outputTruncated = true),
            task.copy(streamText = "x".repeat(4097)), task.copy(streamText = "bad\u0000text"), task.copy(streamTruncated = true),
            task.copy(state = "unknown_outcome", runtimeActive = true, streamText = "late"))) {
            val current = projection.copy(projects = projection.projects.map { it.copy(workerStatuses = listOf(bad)) })
            val malformed = state.copy(projection = current)
            assertNull(WorkerTargetPolicy.cancellable(malformed, task.requestId))
            assertTrue(runCatching { WorkspaceCommand.cancelWorker(current, task.requestId) }.isFailure)
            assertTrue(runCatching { WorkspaceCommand.cancelWorker(current, bad) }.isFailure)
            assertTrue(runCatching { WorkspaceCommand.cancelWorker(projection, bad) }.isFailure)
            assertFalse(WorkspaceDispatchPolicy.allows(malformed, true, true, command))
            val saved = malformed.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = command)
            assertTrue(WorkspaceDispatchPolicy.allows(saved, true, true, WorkspaceCommand.restore(command.envelope)))
            assertFalse(WorkspaceDispatchPolicy.allows(saved, true, true, WorkspaceCommand.cancelWorker(projection, task)))
        }
    }
    @Test fun validLivePreviewAndRuntimeEvidenceRemainCancellableWithoutSubmissionTabs() {
        for (valid in listOf(task, task.copy(state = "accepted", runtimeActive = true),
            task.copy(streamText = "x".repeat(4096), streamTruncated = true),
            task.copy(state = "unknown_outcome", runtimeActive = true))) {
            val current = projection.copy(subtabs = null, projects = projection.projects.map { it.copy(workerStatuses = listOf(valid)) })
            val live = state.copy(projection = current, workerStatusesOpen = false, workerStatusesPage = null)
            assertEquals(valid, WorkerTargetPolicy.cancellable(live, valid.requestId))
            assertFalse(PromptControlPolicy.workerSend(live))
            val command = WorkspaceCommand.cancelWorker(current, valid)
            assertTrue(WorkspaceDispatchPolicy.allows(live, true, true, command))
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            assertFalse(WorkspaceDispatchPolicy.allows(live.copy(scanInProgress = true), true, true, command))
        }
    }
    @Test fun malformedMetadataOrBrokenRetainedChainCannotSupplyAPageOnlyCancel() {
        val command = WorkspaceCommand.cancelWorker(projection, task)
        val first = page.copy(offset = 15, nextOffset = 16, hasMore = true,
            tasks = listOf(task.copy(requestId = "preceding")))
        assertTrue(WorkerStatusPageCodec.validChain(listOf(first), page))
        assertEquals(task, WorkerTargetPolicy.cancellable(state.copy(workerStatusesPrevious = listOf(first)), task.requestId))
        for (bad in listOf(state.copy(workerStatusesPage = page.copy(nextOffset = 18)),
            state.copy(workerStatusesPage = page.copy(tasks = listOf(task.copy(failureCode = "busy")))),
            state.copy(workerStatusesPage = page.copy(tasks = listOf(task.copy(subtabId = "invalid")))),
            state.copy(workerStatusesPage = page.copy(tasks = listOf(task.copy(streamText = "not metadata")))),
            state.copy(workerStatusesPrevious = listOf(first.copy(digest = "e".repeat(64)))),
            state.copy(workerStatusesPrevious = listOf(first.copy(tasks = listOf(task)))),
            state.copy(workerStatusesPrevious = List(8) { first }))) {
            assertNull(WorkerTargetPolicy.cancellable(bad, task.requestId))
            assertFalse(WorkspaceDispatchPolicy.allows(bad, true, true, command))
            assertTrue(WorkspaceDispatchPolicy.allows(bad.copy(pendingWorkspace = command), true, true, command))
        }
    }

    @Test fun explicitCancellationDoesNotRequireValidSubmissionTabsButStillNeedsUniqueTaskIdentity() {
        val summarized = projection.copy(projects = projection.projects.map { it.copy(workerStatuses = listOf(task)) })
        for (current in listOf(summarized.copy(subtabs = null), summarized.copy(subtabs = summarized.subtabs!!.copy(activeId = "f".repeat(64))),
            summarized.copy(subtabs = summarized.subtabs!!.copy(tabs = emptyList())))) {
            val live = state.copy(projection = current)
            assertFalse(PromptControlPolicy.workerSend(live))
            assertTrue(runCatching { WorkspaceCommand.workerPrompt(current, "new task") }.isFailure)
            assertEquals(task, WorkerTargetPolicy.cancellable(live, task.requestId))
            val command = WorkspaceCommand.cancelWorker(current, task.requestId)
            assertEquals(setOf("request_id"), command.envelope.getAsJsonObject("payload").getAsJsonObject("arguments").keySet())
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            assertTrue(WorkspaceDispatchPolicy.allows(live, true, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(live.copy(scanInProgress = true), true, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(live.copy(projection = current.copy(sessionId = "foreign")), true, true, command))
        }
        val duplicate = summarized.copy(projects = summarized.projects.map { it.copy(workerStatuses = listOf(task, task.copy(state = "succeeded"))) })
        assertTrue(runCatching { WorkspaceCommand.cancelWorker(duplicate, task.requestId) }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.cancelWorker(duplicate, task) }.isFailure)
        assertNull(WorkerTargetPolicy.cancellable(state.copy(projection = duplicate), task.requestId))
        val pageOnly = state.copy(projection = projection.copy(subtabs = null))
        val command = WorkspaceCommand.cancelWorker(pageOnly.projection!!, requireNotNull(WorkerTargetPolicy.cancellable(pageOnly, task.requestId)))
        assertTrue(WorkspaceDispatchPolicy.allows(pageOnly, true, true, command))
        for (mixed in listOf(task.copy(state = "succeeded"), task.copy(sessionId = "foreign"), task.copy(projectId = "foreign"))) {
            val invalid = pageOnly.copy(workerStatusesPage = page.copy(tasks = listOf(task, mixed)))
            assertNull(WorkerTargetPolicy.cancellable(invalid, task.requestId))
            assertFalse(WorkspaceDispatchPolicy.allows(invalid, true, true, command))
            assertTrue(WorkspaceDispatchPolicy.allows(invalid.copy(pendingWorkspace = command), true, true, command))
        }
    }
    private val project = "a".repeat(64)
    private val tab = "b".repeat(64)
    private val task = RemoteWorkerStatus("page-only", project, "session", tab, "running")
    private val projection = HostProjection(listOf(RemoteProject(project, "Workspace", "session", workerControl = true)),
        project, "session", "idle", emptyList(), "1", false, "0", workspaceOwner = "c".repeat(64), workspaceRevision = "4",
        subtabs = RemoteSubtabs(project, "2", tab, listOf(RemoteSubtab(tab, "main", "main", 1)), false))
    private val page = WorkerStatusPage(project, "session", "d".repeat(64), 16, 17, 17, false, listOf(task))
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, workerStatusesOpen = true, workerStatusesPage = page)
    @Test fun pageOnlyRunningTaskUsesExactHostBoundCancelAndExplicitRetry() {
        assertEquals(task, WorkerTargetPolicy.cancellable(state, task.requestId))
        val command = WorkspaceCommand.cancelWorker(projection, requireNotNull(WorkerTargetPolicy.cancellable(state, task.requestId)))
        assertEquals(task.requestId, command.envelope.getAsJsonObject("payload").getAsJsonObject("arguments")["request_id"].asString)
        assertEquals("session", command.envelope["session_id"].asString)
        assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
        val closed = state.copy(workerStatusesOpen = false, workerStatusesPage = null)
        assertFalse(WorkspaceDispatchPolicy.allows(closed, true, true, command))
        assertTrue(WorkspaceDispatchPolicy.allows(closed.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = command), true, true, command))
    }
    @Test fun pageCannotSupplyControlAfterLoadingErrorOfflineOrScopeAndCapabilityLoss() {
        for (changed in listOf(state.copy(workerStatusesOpen = false), state.copy(workerStatusesLoading = true),
            state.copy(workerStatusesError = "changed"), state.copy(workerStatusesPage = null), state.copy(phase = PairingPhase.ERROR),
            state.copy(phase = PairingPhase.OFFLINE), state.copy(projection = projection.copy(sessionId = "replacement")),
            state.copy(workerStatusesPage = page.copy(projectId = "foreign")),
            state.copy(projection = projection.copy(projects = projection.projects.map { it.copy(workerControl = false) })),
            state.copy(workerStatusesPage = page.copy(tasks = listOf(task.copy(sessionId = "foreign")))))) {
            assertNull(WorkerTargetPolicy.cancellable(changed, task.requestId))
        }
        assertNull(WorkerTargetPolicy.cancellable(state, "missing"))
    }
    @Test fun currentSummaryTerminalStateOverridesStalePageRuntimeEvidence() {
        for (status in listOf(task.copy(state = "succeeded"), task.copy(state = "cancelled"),
            task.copy(state = "unknown_outcome", runtimeActive = false))) {
            val current = state.copy(projection = projection.copy(projects = projection.projects.map { it.copy(workerStatuses = listOf(status)) }))
            assertNull(WorkerTargetPolicy.cancellable(current, task.requestId))
        }
        val active = task.copy(state = "unknown_outcome", runtimeActive = true)
        assertEquals(active, WorkerTargetPolicy.cancellable(state.copy(workerStatusesPage = page.copy(tasks = listOf(active))), task.requestId))
        assertNull(WorkerTargetPolicy.cancellable(state.copy(workerStatusesPage = page.copy(tasks = listOf(active.copy(runtimeActive = false)))), task.requestId))
    }
    @Test fun foreignCompletedOrDuplicateTargetsNeverCreateOrAdmitAWrite() {
        for (target in listOf(task.copy(projectId = "foreign"), task.copy(sessionId = "foreign"), task.copy(state = "succeeded"))) {
            assertTrue(runCatching { WorkspaceCommand.cancelWorker(projection, target) }.isFailure)
        }
        assertNull(WorkerTargetPolicy.cancellable(state.copy(workerStatusesPage = page.copy(tasks = listOf(task, task))), task.requestId))
        for (statuses in listOf(listOf(task, task), listOf(task.copy(sessionId = "foreign")))) {
            assertNull(WorkerTargetPolicy.cancellable(state.copy(projection = projection.copy(
                projects = projection.projects.map { it.copy(workerStatuses = statuses) })), task.requestId))
        }
        val command = WorkspaceCommand.cancelWorker(projection, task)
        assertFalse(WorkspaceDispatchPolicy.allows(state, false, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state, true, false, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state.copy(pendingRevoke = "revoke"), true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state.copy(resetRequired = true), true, true, command))
    }
}
