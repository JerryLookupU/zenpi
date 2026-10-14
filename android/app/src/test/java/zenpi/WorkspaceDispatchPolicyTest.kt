package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class WorkspaceDispatchPolicyTest {
    private val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", true, true,
        listOf(RemotePromptStatus("running", "project", "session", "running")))), "project", "session", "idle",
        emptyList(), "1", false, "0", workspaceOwner = "a".repeat(64), workspaceRevision = "1")
    private val cancel = WorkspaceCommand.cancelPrompt(projection, "running")
    private val prompt = WorkspaceCommand.prompt(projection, "hello")
    private val modelStatus = RemoteModelStatus("project", "session", "gpt-fixture", "high", true, "0", null,
        "32768", "4096", listOf(null, "none", "low"))
    private val model = WorkspaceCommand.selectModel(projection.copy(projects = projection.projects.map { it.copy(modelControl = true) }),
        modelStatus, "gpt-next")
    private val reasoning = WorkspaceCommand.selectReasoning(projection.copy(projects = projection.projects.map { it.copy(reasoningControl = true) }),
        RemoteModelStatus("project","session","gpt-fixture","high",true,"0",null,"32768","4096",listOf(null,"none","low")),null)
    private val state = PairingState(phase = PairingPhase.CONNECTED, modelStatus = modelStatus,
        projection = projection.copy(projects = projection.projects.map { it.copy(modelControl = true, reasoningControl = true) }))
    private fun allowed(value: PairingState = state, foreground: Boolean = true, network: Boolean = true,
        command: WorkspaceCommand = cancel) = WorkspaceDispatchPolicy.allows(value, foreground, network, command)

    @Test fun modelWritesRevalidateCurrentStatusAndReadStateBeforeSaving() {
        for (current in listOf(state.copy(modelStatus = null), state.copy(modelLoading = true), state.copy(modelError = "read failed"),
            state.copy(modelStatus = modelStatus.copy(projectId = "foreign")), state.copy(modelStatus = modelStatus.copy(sessionId = "replacement")),
            state.copy(modelStatus = modelStatus.copy(registryBound = false)), state.copy(modelStatus = modelStatus.copy(selectionRevision = "1")))) {
            for (command in listOf(model, reasoning)) assertFalse(allowed(current, command = command))
        }
        assertTrue(allowed(command = model)); assertTrue(allowed(command = reasoning))
        // A manually entered model is still legal; the host validates the registry.
        assertTrue(allowed(command = WorkspaceCommand.selectModel(requireNotNull(state.projection), modelStatus, "manual-model")))
    }
    @Test fun liveOwnerSelectionCanInvalidateADisplayedModelPageWithoutWorkspaceChange() {
        val live = requireNotNull(state.projection).copy(modelStatus = modelStatus)
        assertTrue(allowed(state.copy(projection = live), command = model))
        for (current in listOf(modelStatus.copy(selectionRevision = "1"), modelStatus.copy(registryBound = false),
            modelStatus.copy(projectId = "foreign"), modelStatus.copy(sessionId = "replacement"))) {
            for (command in listOf(model, reasoning)) assertFalse(allowed(state.copy(projection = live.copy(modelStatus = current)), command = command))
        }
        val advanced = state.copy(modelStatus = modelStatus.copy(selectionRevision = "1"),
            projection = live.copy(modelStatus = modelStatus.copy(selectionRevision = "1")))
        val fresh = WorkspaceCommand.selectModel(requireNotNull(advanced.projection), requireNotNull(advanced.modelStatus), "gpt-next")
        assertTrue(allowed(advanced, command = fresh)); assertFalse(allowed(advanced, command = model))
    }
    @Test fun reasoningSelectionRechecksDefaultAndExplicitNoneAgainstCurrentChoices() {
        val explicit = WorkspaceCommand.selectReasoning(requireNotNull(state.projection), modelStatus, "none")
        assertTrue(allowed(command = explicit)); assertTrue(allowed(command = reasoning))
        assertFalse(allowed(state.copy(modelStatus = modelStatus.copy(reasoningChoices = listOf("none", "low"))), command = reasoning))
        assertFalse(allowed(state.copy(modelStatus = modelStatus.copy(reasoningChoices = listOf(null, "low"))), command = explicit))
        val changed = requireNotNull(state.projection).copy(modelStatus = modelStatus.copy(reasoningChoices = listOf("low")))
        for (command in listOf(reasoning, explicit)) assertFalse(allowed(state.copy(projection = changed), command = command))
    }
    @Test fun savedModelRetryRemainsExactDespiteMissingStaleOrFailedModelReads() {
        val changed = state.copy(modelStatus = null, modelLoading = true, modelError = "unavailable",
            projection = requireNotNull(state.projection).copy(modelStatus = modelStatus.copy(selectionRevision = "1", reasoningChoices = emptyList())))
        for (command in listOf(model, reasoning)) {
            for (phase in listOf(PairingPhase.OFFLINE, PairingPhase.ERROR, PairingPhase.CONNECTED)) {
                val pending = changed.copy(phase = phase, pendingWorkspace = command)
                assertTrue(allowed(pending, command = WorkspaceCommand.restore(command.envelope)))
                assertFalse(allowed(pending, command = WorkspaceCommand.selectModel(requireNotNull(state.projection), modelStatus, "gpt-next")))
                assertFalse(allowed(pending, foreground = false, command = command))
                assertFalse(allowed(pending, network = false, command = command))
            }
        }
    }

    @Test fun retainedLiveAuthorityCannotStartBackgroundOrNetworklessWrites() {
        for (phase in PairingPhase.values()) for (command in listOf(cancel, prompt, model, reasoning)) {
            assertFalse(allowed(state.copy(phase = phase), foreground = false, command = command))
            assertFalse(allowed(state.copy(phase = phase), network = false, command = command))
        }
    }
    @Test fun freshWritesRequireLivePhaseButErrorRetainsExactRunningControls() {
        assertTrue(allowed()); assertTrue(allowed(command = prompt))
        assertTrue(allowed(state.copy(phase = PairingPhase.ERROR)))
        assertTrue(allowed(state.copy(phase = PairingPhase.ERROR), command = WorkspaceCommand.steer(projection, "running", "change direction")))
        val approval = RemoteApproval("approval", "project", "session", "turn", "call", "write_file", "workspace_write", "agent_tool", "b".repeat(64))
        val approving = projection.copy(projects = projection.projects.map { it.copy(approvalControl = true, pendingApprovals = listOf(approval)) })
        assertTrue(allowed(state.copy(phase = PairingPhase.ERROR, projection = approving), command = WorkspaceCommand.approve(approving, approval, "deny")))
        assertFalse(allowed(state.copy(phase = PairingPhase.ERROR), command = prompt))
        assertFalse(allowed(state.copy(phase = PairingPhase.ERROR), command = model))
        assertTrue(allowed(command = model))
        assertTrue(allowed(command = reasoning));assertFalse(allowed(state.copy(phase=PairingPhase.ERROR),command=reasoning))
        for (phase in listOf(PairingPhase.OFFLINE, PairingPhase.CONFIRMING, PairingPhase.ACCEPTED, PairingPhase.CONNECTING)) {
            assertFalse(allowed(state.copy(phase = phase)))
            assertFalse(allowed(state.copy(phase = phase), command = prompt))
        }
    }
    @Test fun foregroundExplicitRetryAllowsOnlyTheSavedEnvelope() {
        val pending = state.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = prompt)
        assertTrue(allowed(pending, command = WorkspaceCommand.restore(prompt.envelope)))
        assertFalse(allowed(pending, command = WorkspaceCommand.prompt(projection, "hello")))
        assertFalse(allowed(pending, command = cancel))
        assertFalse(allowed(pending, foreground = false, command = prompt))
        assertFalse(allowed(pending, network = false, command = prompt))
        assertFalse(allowed(pending.copy(phase = PairingPhase.CONNECTING), command = prompt))
        assertFalse(allowed(pending.copy(phase = PairingPhase.CONFIRMING), command = prompt))
        assertFalse(allowed(pending.copy(phase = PairingPhase.ACCEPTED), command = prompt))
        val modelPending = state.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = model)
        assertTrue(allowed(modelPending, command = WorkspaceCommand.restore(model.envelope)))
        assertFalse(allowed(modelPending, command = prompt))
        val reasoningPending=state.copy(phase=PairingPhase.OFFLINE,pendingWorkspace=reasoning)
        assertTrue(allowed(reasoningPending,command=WorkspaceCommand.restore(reasoning.envelope)))
        assertFalse(allowed(reasoningPending,command=model))
    }
    @Test fun resetAndUnresolvedRevocationFenceEvenExactPendingRetries() {
        for (value in listOf(state.copy(resetting = true), state.copy(resetRequired = true), state.copy(pendingRevoke = "revoke"))) {
            assertFalse(allowed(value)); assertFalse(allowed(value.copy(pendingWorkspace = cancel)))
        }
    }
    @Test fun retainedErrorAuthorityMatchesWorkerSessionNotJustApprovalIdAndDigest() {
        val target = RemoteApproval("approval", "project", "session", "turn", "call", "write_file", "workspace_write", "agent_tool", "b".repeat(64),
            workerSessionId = "worker-one", subtabId = "c".repeat(64))
        val live = projection.copy(projects = projection.projects.map { it.copy(approvalControl = true, pendingApprovals = listOf(target)) })
        val command = WorkspaceCommand.approve(live, target, "allow")
        assertTrue(allowed(state.copy(phase = PairingPhase.ERROR, projection = live), command = command))
        for (replacement in listOf(target.copy(workerSessionId = "worker-two"), target.copy(workerSessionId = null, subtabId = null))) {
            val foreign = live.copy(projects = live.projects.map { it.copy(pendingApprovals = listOf(replacement)) })
            assertFalse(allowed(state.copy(phase = PairingPhase.ERROR, projection = foreign), command = command))
            assertFalse(allowed(state.copy(projection = foreign), command = command))
        }
        assertFalse(allowed(state.copy(phase = PairingPhase.ERROR, projection = live), foreground = false, command = command))
        val pending = state.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = command)
        assertTrue(allowed(pending, command = WorkspaceCommand.restore(command.envelope)))
        assertFalse(allowed(pending, command = WorkspaceCommand.approve(live, target, "allow")))
    }
    @Test fun connectedAdmissionRejectsOldScopeOwnerRevisionOrLostCapabilityBeforeSaving() {
        val changed = listOf(projection.copy(selectedId = "foreign"), projection.copy(sessionId = "replacement"),
            projection.copy(workspaceOwner = "b".repeat(64)), projection.copy(workspaceRevision = "2"),
            projection.copy(workspaceOwner = null, workspaceRevision = null), projection.copy(projects = emptyList()),
            projection.copy(projects = projection.projects.map { it.copy(sessionId = "replacement") }))
        for (current in changed) for (command in listOf(prompt, cancel)) {
            assertFalse(allowed(state.copy(projection = current), command = command))
        }
        assertFalse(allowed(state.copy(projection = null), command = prompt))
        val noControls = projection.copy(projects = projection.projects.map { it.copy(promptControl = false) })
        assertFalse(allowed(state.copy(projection = noControls), command = prompt))
        assertFalse(allowed(state.copy(projection = noControls), command = cancel))
        val finished = projection.copy(projects = projection.projects.map { it.copy(promptStatuses = emptyList()) })
        assertFalse(allowed(state.copy(projection = finished), command = cancel))
        assertFalse(allowed(state.copy(projection = projection), command = model))
        assertFalse(allowed(state.copy(projection = projection), command = reasoning))
    }
    @Test fun workspaceSelectionCanTargetAnotherListedProjectButCannotBorrowItsOldSession() {
        val other = RemoteProject("other", "Other", "other-session", workspaceControl = true)
        val both = projection.copy(projects = projection.projects + other)
        val command = WorkspaceCommand.create(both, other.id, "workspace_select")
        assertTrue(allowed(state.copy(projection = both), command = command))
        assertFalse(allowed(state.copy(projection = both.copy(projects = both.projects + other.copy(sessionId = "foreign"))), command = command))
        assertFalse(allowed(state.copy(projection = both.copy(projects = both.projects.map {
            if (it.id == other.id) it.copy(sessionId = "replacement") else it
        })), command = command))
        assertFalse(allowed(state.copy(projection = both.copy(projects = both.projects.map {
            if (it.id == other.id) it.copy(workspaceControl = false) else it
        })), command = command))
    }
    @Test fun mixedDuplicateProjectIdentityCannotAuthorizeFreshSharedCommandsButDoesNotRetargetSavedRetry() {
        val ambiguous = requireNotNull(state.projection).copy(projects = requireNotNull(state.projection).projects +
            requireNotNull(state.projection).projects.single().copy(sessionId = "foreign"))
        for (command in listOf(prompt, cancel, model, reasoning)) {
            assertFalse(allowed(state.copy(projection = ambiguous), command = command))
            assertTrue(allowed(state.copy(projection = ambiguous, pendingWorkspace = command), command = WorkspaceCommand.restore(command.envelope)))
            assertFalse(allowed(state.copy(projection = ambiguous, pendingWorkspace = command, scanInProgress = true), command = command))
        }
    }
    @Test fun exactPendingRetryDoesNotRetargetAfterTheCurrentProjectionHasChanged() {
        val pending = state.copy(projection = projection.copy(sessionId = "replacement", workspaceOwner = null, workspaceRevision = null),
            pendingWorkspace = prompt)
        for (phase in listOf(PairingPhase.CONNECTED, PairingPhase.ERROR, PairingPhase.OFFLINE)) {
            assertTrue(allowed(pending.copy(phase = phase), command = WorkspaceCommand.restore(prompt.envelope)))
            assertFalse(allowed(pending.copy(phase = phase), command = WorkspaceCommand.prompt(projection, "hello")))
        }
        assertEquals("session", pending.pendingWorkspace?.envelope?.get("session_id")?.asString)
    }
}
