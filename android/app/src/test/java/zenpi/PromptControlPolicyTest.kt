package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class PromptControlPolicyTest {
    @Test fun damagedRetainedPromptCannotAuthorizeFreshSteerOrCancelButSavedIntentCanReconcile() {
        val commands = listOf(WorkspaceCommand.cancelPrompt(projection, target.requestId),
            WorkspaceCommand.steer(projection, target.requestId, "continue carefully"))
        for (bad in listOf(target.copy(failureCode = "busy"), target.copy(streamText = "x".repeat(4097)),
            target.copy(streamText = "bad\u0000text"), target.copy(streamTruncated = true))) {
            val malformed = projection.copy(projects = listOf(project.copy(promptStatuses = listOf(bad))))
            val current = state.copy(projection = malformed)
            assertNull(PromptStatusCodec.active(malformed, target.requestId))
            assertFalse(PromptControlPolicy.active(current, target.requestId))
            assertTrue(runCatching { WorkspaceCommand.cancelPrompt(malformed, target.requestId) }.isFailure)
            assertTrue(runCatching { WorkspaceCommand.steer(malformed, target.requestId, "continue carefully") }.isFailure)
            for (command in commands) {
                assertFalse(WorkspaceDispatchPolicy.allows(current, true, true, command))
                assertTrue(WorkspaceDispatchPolicy.allows(current.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = command),
                    true, true, WorkspaceCommand.restore(command.envelope)))
                assertFalse(WorkspaceDispatchPolicy.allows(current.copy(scanInProgress = true, pendingWorkspace = command), true, true, command))
            }
        }
    }
    @Test fun acceptedAndRunningBoundedPreviewsKeepSteerAndCancelAvailable() {
        for (valid in listOf(target, target.copy(state = "accepted"),
            target.copy(streamText = "x".repeat(4096), streamTruncated = true),
            target.copy(streamText = "", streamTruncated = true))) {
            val current = projection.copy(projects = listOf(project.copy(promptStatuses = listOf(valid))))
            val live = state.copy(projection = current)
            assertEquals(valid, PromptStatusCodec.active(current, target.requestId))
            assertTrue(PromptControlPolicy.active(live, target.requestId))
            for (command in listOf(WorkspaceCommand.cancelPrompt(current, target.requestId),
                WorkspaceCommand.steer(current, target.requestId, "continue carefully"))) {
                assertTrue(WorkspaceDispatchPolicy.allows(live, true, true, command))
                assertTrue(WorkspaceDispatchPolicy.allows(live.copy(phase = PairingPhase.ERROR), true, true, command))
                assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            }
        }
    }
    @Test fun workerSubmissionRequiresCompleteUniqueTabScopeButCancellationCanTargetAnotherTab() {
        val tabs = requireNotNull(projection.subtabs)
        val other = RemoteSubtab("c".repeat(64), "Other", "in_place", 1)
        val live = projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + other))
        val command = WorkspaceCommand.workerPrompt(live, "review current tab")
        val connected = state.copy(projection = live)
        assertTrue(PromptControlPolicy.workerSend(connected))
        assertTrue(WorkspaceDispatchPolicy.allows(connected, true, true, command))
        for (invalid in listOf(live.copy(subtabs = live.subtabs!!.copy(tabs = tabs.tabs + other + other)),
            live.copy(subtabs = tabs.copy(tabs = emptyList())), live.copy(subtabs = tabs.copy(projectId = "foreign")),
            live.copy(subtabs = tabs.copy(activeId = other.id)))) {
            val invalidState = state.copy(projection = invalid)
            assertFalse(PromptControlPolicy.workerSend(invalidState))
            assertTrue(runCatching { WorkspaceCommand.workerPrompt(invalid, "review") }.isFailure)
            assertFalse(WorkspaceDispatchPolicy.allows(invalidState, true, true, command))
            assertTrue(WorkspaceDispatchPolicy.allows(invalidState.copy(pendingWorkspace = command), true, true, command))
        }
        val switched = live.copy(subtabs = live.subtabs!!.copy(activeId = other.id))
        val switchedState = state.copy(projection = switched)
        assertFalse(WorkspaceDispatchPolicy.allows(switchedState, true, true, command))
        assertEquals(task, WorkerTargetPolicy.cancellable(switchedState, task.requestId))
        val cancel = WorkspaceCommand.cancelWorker(switched, task)
        assertTrue(WorkspaceDispatchPolicy.allows(switchedState, true, true, cancel))
        assertEquals(task.requestId, cancel.envelope.getAsJsonObject("payload").getAsJsonObject("arguments")["request_id"].asString)
        assertEquals(other.id, WorkspaceCommand.workerPrompt(switched, "review other tab").envelope
            .getAsJsonObject("payload").getAsJsonObject("arguments")["subtab_id"].asString)
    }
    private val target = RemotePromptStatus("running", "project", "session", "running")
    private val task = RemoteWorkerStatus("worker", "project", "session", "b".repeat(64), "running")
    private val project = RemoteProject("project", "Workspace", "session", promptControl = true,
        promptStatuses = listOf(target), workerControl = true, workerStatuses = listOf(task))
    private val projection = HostProjection(listOf(project), project.id, project.sessionId, "running", emptyList(), "1", false,
        workspaceOwner = "a".repeat(64), workspaceRevision = "1",
        subtabs = RemoteSubtabs(project.id, "0", task.subtabId, listOf(RemoteSubtab(task.subtabId, "Main", "main", 1)), false))
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection)

    @Test fun activePromptTargetsAreUniqueAndMatchActualProjectSessionBeforeBuildingOrDispatching() {
        val cancel = WorkspaceCommand.cancelPrompt(projection, target.requestId)
        val steer = WorkspaceCommand.steer(projection, target.requestId, "continue carefully")
        assertTrue(PromptControlPolicy.active(state, target.requestId))
        assertTrue(PromptControlPolicy.active(state.copy(phase = PairingPhase.ERROR), target.requestId))
        fun statuses(vararg values: RemotePromptStatus) = projection.copy(projects = listOf(project.copy(promptStatuses = values.toList())))
        for (invalid in listOf(projection.copy(projects = listOf(project, project.copy(sessionId = "foreign"))),
            statuses(target, target.copy(state = "succeeded")), statuses(target.copy(projectId = "foreign")),
            statuses(target.copy(sessionId = "foreign")), statuses(target.copy(state = "succeeded")))) {
            assertNull(PromptStatusCodec.active(invalid, target.requestId))
            assertFalse(PromptControlPolicy.active(state.copy(projection = invalid), target.requestId))
            assertTrue(runCatching { WorkspaceCommand.cancelPrompt(invalid, target.requestId) }.isFailure)
            assertTrue(runCatching { WorkspaceCommand.steer(invalid, target.requestId, "continue carefully") }.isFailure)
            for (command in listOf(cancel, steer)) {
                assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = invalid), true, true, command))
                assertTrue(WorkspaceDispatchPolicy.allows(state.copy(projection = invalid, pendingWorkspace = command), true, true,
                    WorkspaceCommand.restore(command.envelope)))
            }
        }
    }

    @Test fun scannerResetAndAmbiguousTabsFencePromptWorkerAndSummaryCancellation() {
        assertTrue(PromptControlPolicy.send(state)); assertTrue(PromptControlPolicy.workerSend(state))
        assertEquals(task, WorkerTargetPolicy.cancellable(state, task.requestId))
        for (invalid in listOf(state.copy(scanInProgress = true), state.copy(resetting = true), state.copy(resetRequired = true),
            state.copy(pendingRevoke = "pending"), state.copy(phase = PairingPhase.OFFLINE), state.copy(phase = PairingPhase.CONNECTING))) {
            assertFalse(PromptControlPolicy.send(invalid)); assertFalse(PromptControlPolicy.workerSend(invalid))
            assertFalse(PromptControlPolicy.active(invalid, target.requestId)); assertNull(WorkerTargetPolicy.cancellable(invalid, task.requestId))
        }
        val ambiguous = projection.copy(projects = listOf(project, project.copy(sessionId = "foreign")))
        assertFalse(PromptControlPolicy.send(state.copy(projection = ambiguous)))
        assertNull(WorkerTargetPolicy.cancellable(state.copy(projection = ambiguous), task.requestId))
        assertTrue(runCatching { WorkspaceCommand.prompt(ambiguous, "hello") }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.workerPrompt(ambiguous, "hello") }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.cancelWorker(ambiguous, task) }.isFailure)
        val tabs = requireNotNull(projection.subtabs)
        for (invalid in listOf(projection.copy(subtabs = tabs.copy(tabs = emptyList())),
            projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs)))) {
            assertFalse(PromptControlPolicy.workerSend(state.copy(projection = invalid)))
            assertTrue(runCatching { WorkspaceCommand.workerPrompt(invalid, "hello") }.isFailure)
        }
        assertFalse(PromptControlPolicy.send(state.copy(phase = PairingPhase.ERROR)))
        assertFalse(PromptControlPolicy.workerSend(state.copy(phase = PairingPhase.ERROR)))
    }
}
