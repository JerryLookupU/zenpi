package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class ApprovalDecisionPolicyTest {
    @Test fun malformedRetainedSummaryOrPreviewCannotAuthorizeAnyFreshDecision() {
        val command = WorkspaceCommand.approve(projection, target, "allow")
        val preview = RemoteApprovalPreview("note.txt", "+hello\n", true, false, false, false)
        val malformed = listOf(target.copy(turnId = ""), target.copy(callId = "bad\n"), target.copy(tool = "x".repeat(129)),
            target.copy(sideEffect = "unknown"), target.copy(origin = "unknown"), target.copy(digest = "bad"),
            target.copy(workerSessionId = null), target.copy(subtabId = null), target.copy(subtabId = "bad"),
            target.copy(preview = preview.copy(path = "bad\n")), target.copy(preview = preview.copy(patch = "x".repeat(2049))),
            target.copy(preview = preview.copy(patch = "bad\u001b")), target.copy(preview = preview.copy(withheld = true)))
        for (bad in malformed) {
            val changed = projection.copy(projects = listOf(project.copy(pendingApprovals = listOf(bad))))
            val state = state.copy(projection = changed)
            assertFalse(ApprovalProjectionCodec.current(changed, bad))
            assertFalse(ApprovalDecisionPolicy.allows(state, bad))
            for (decision in listOf("allow", "deny")) assertTrue(runCatching {
                WorkspaceCommand.approve(changed, bad, decision)
            }.isFailure)
            assertFalse(WorkspaceDispatchPolicy.allows(state, true, true, command))
            assertTrue(WorkspaceDispatchPolicy.allows(state.copy(pendingWorkspace = command, phase = PairingPhase.OFFLINE),
                true, true, WorkspaceCommand.restore(command.envelope)))
        }
        for (targets in listOf(listOf(target, target.copy(requestId = "other", tool = "bad\n")),
            listOf(target, target.copy(requestId = "other", projectId = "foreign")),
            listOf(target) + (1..16).map { target.copy(requestId = "other-$it") })) {
            val changed = projection.copy(projects = listOf(project.copy(pendingApprovals = targets)))
            assertFalse(ApprovalProjectionCodec.current(changed, target))
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = changed), true, true, command))
        }
        val withheld = target.copy(preview = preview.copy(path = "<withheld>", patch = "<withheld>", withheld = true, redacted = true))
        assertTrue(ApprovalProjectionCodec.current(projection.copy(projects = listOf(project.copy(pendingApprovals = listOf(withheld)))), withheld))
    }

    private val target = RemoteApproval("approval", "project", "session", "turn", "call", "write_file",
        "workspace_write", "agent_tool", "a".repeat(64), workerSessionId = "worker", subtabId = "c".repeat(64))
    private val project = RemoteProject("project", "Workspace", "session", pendingApprovals = listOf(target), approvalControl = true)
    private val projection = HostProjection(listOf(project), project.id, project.sessionId, "running", emptyList(), "1", false,
        workspaceOwner = "b".repeat(64), workspaceRevision = "0")
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection)

    @Test fun confirmationUsesUniqueCurrentApprovalScopeAndNeverBorrowsPromptAuthority() {
        assertFalse(project.promptControl)
        assertTrue(ApprovalDecisionPolicy.allows(state, target))
        assertTrue(ApprovalDecisionPolicy.allows(state.copy(phase = PairingPhase.ERROR), target))
        val command = WorkspaceCommand.approve(projection, target, "allow")
        fun pending(vararg targets: RemoteApproval) = projection.copy(projects = listOf(project.copy(pendingApprovals = targets.toList())))
        for (invalid in listOf(projection.copy(projects = listOf(project, project.copy(sessionId = "foreign"))),
            projection.copy(projects = listOf(project.copy(approvalControl = false))), projection.copy(sessionId = "foreign"),
            pending(target, target.copy(digest = "d".repeat(64))), pending(target.copy(workerSessionId = "replacement")),
            pending(target.copy(sessionId = "foreign")))) {
            assertFalse(ApprovalProjectionCodec.current(invalid, target))
            assertFalse(ApprovalDecisionPolicy.allows(state.copy(projection = invalid), target))
            assertTrue(runCatching { WorkspaceCommand.approve(invalid, target, "allow") }.isFailure)
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = invalid), true, true, command))
        }
        val otherWorker = target.copy(workerSessionId = "other-worker")
        val both = projection.copy(projects = listOf(project.copy(pendingApprovals = listOf(target, otherWorker))))
        assertTrue(ApprovalProjectionCodec.current(both, target))
        assertTrue(ApprovalProjectionCodec.current(both, otherWorker))
        assertNotNull(WorkspaceCommand.approve(both, otherWorker, "deny"))
    }

    @Test fun scannerResetAndPendingIntentFenceConfirmationButExactSavedRetryRemainsDistinct() {
        val command = WorkspaceCommand.approve(projection, target, "deny")
        for (invalid in listOf(state.copy(scanInProgress = true), state.copy(resetting = true), state.copy(resetRequired = true),
            state.copy(pendingRevoke = "pending"), state.copy(pendingWorkspace = command), state.copy(projection = null),
            state.copy(phase = PairingPhase.OFFLINE), state.copy(phase = PairingPhase.CONNECTING))) {
            assertFalse(ApprovalDecisionPolicy.allows(invalid, target))
        }
        val pending = state.copy(projection = null, pendingWorkspace = command, phase = PairingPhase.OFFLINE)
        assertTrue(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.restore(command.envelope)))
        assertFalse(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.approve(projection, target, "deny")))
        assertFalse(WorkspaceDispatchPolicy.allows(pending.copy(scanInProgress = true), true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(pending, false, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(pending, true, false, command))
    }
}
