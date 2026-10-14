package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.security.PairedHost
import zenpi.transport.*

class AuthorityLossPolicyTest {
    private val tab = "a".repeat(64)
    private val approval = RemoteApproval("approval", "project", "session", "turn", "call", "write_file",
        "workspace_write", "agent_tool", "b".repeat(64))
    private val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", workspaceControl = true,
        promptControl = true, approvalControl = true, diffAvailable = true, modelControl = true, reasoningControl = true,
        pendingApprovals = listOf(approval), desktopStreamText = "TRANSIENT",
        terminal = RemoteTerminal("project", "session", tab, "7", "TRANSIENT", false),
        promptStatuses = listOf(RemotePromptStatus("running", "project", "session", "running", "TRANSIENT"),
            RemotePromptStatus("failed", "project", "session", "failed", failureCode = "invalid_request")))),
        "project", "session", "idle", listOf(RemoteTurn("turn", "assistant", "durable display")), "42", false, "8",
        workspaceOwner = "c".repeat(64), workspaceRevision = "4",
        subtabs = RemoteSubtabs("project", "7", tab, listOf(RemoteSubtab(tab, "main", "main", 1)), true))
    private val state = PairingState(phase = PairingPhase.ERROR, registered = true,
        host = PairedHost("192.168.1.2", 7777, "d".repeat(64), "e".repeat(64)), projection = projection,
        diffOpen = true, diffLoading = true, filesOpen = true, filesLoading = true,
        resourcesOpen = true, resourcesLoading = true, modelOpen = true, modelLoading = true,
        transcriptTurn = "turn", transcriptLoading = true)
    @Test fun onlyTypedAuthorityRefusalsFenceNotTimeoutsOrSimilarText() {
        for (code in listOf("unauthorized", "revoked_device", "owner_mismatch")) {
            assertTrue(AuthorityLossPolicy.refused(HostRefusedException(code)))
            assertTrue(AuthorityLossPolicy.refused(WorkspaceRefusedException("Host request refused: $code", code)))
            assertFalse(AuthorityLossPolicy.refused(WireException("Host request refused: $code")))
            assertFalse(AuthorityLossPolicy.refused(WorkspaceRefusedException("Host request refused: $code")))
        }
        for (code in listOf("busy", "revision_conflict", "invalid_request", "request_id_conflict", "resource_limit"))
            assertFalse(AuthorityLossPolicy.refused(HostRefusedException(code)))
        assertFalse(AuthorityLossPolicy.refused(java.net.SocketTimeoutException("unauthorized")))
        assertFalse(AuthorityLossPolicy.refused(kotlinx.coroutines.CancellationException("revoked_device")))
    }
    @Test fun refusalPreservesDurableDisplayAndCursorButRemovesEveryControlAndTransientSurface() {
        val next = AuthorityLossPolicy.displayOnly(state)
        assertEquals(state.host, next.host); assertTrue(next.registered)
        val display = requireNotNull(next.projection)
        assertEquals(projection.turns, display.turns)
        assertEquals("42", display.journalSequence); assertEquals("8", display.eventSequence)
        assertEquals("7", display.subtabs?.revision); assertFalse(requireNotNull(display.subtabs).control)
        val project = display.projects.single()
        assertFalse(project.workspaceControl); assertFalse(project.promptControl); assertFalse(project.approvalControl)
        assertFalse(project.diffAvailable); assertFalse(project.modelControl); assertFalse(project.reasoningControl)
        assertTrue(project.pendingApprovals.isEmpty()); assertNull(project.desktopStreamText); assertNull(project.terminal)
        assertEquals("unknown_outcome", project.promptStatuses.first().state)
        assertNull(project.promptStatuses.first().streamText)
        assertEquals("invalid_request", project.promptStatuses.last().failureCode)
        assertNull(display.workspaceOwner); assertNull(display.workspaceRevision); assertNull(next.modelStatus)
        assertFalse(next.diffOpen); assertFalse(next.diffLoading); assertNull(next.diff)
        assertFalse(next.filesOpen); assertFalse(next.filesLoading); assertNull(next.files)
        assertFalse(next.resourcesOpen); assertFalse(next.resourcesLoading); assertNull(next.resources)
        assertFalse(next.modelOpen); assertFalse(next.modelLoading)
        assertNull(next.transcriptTurn); assertNull(next.transcriptPage); assertFalse(next.transcriptLoading)
        for (operation in listOf<() -> WorkspaceCommand>({ WorkspaceCommand.prompt(display, "hello") },
            { WorkspaceCommand.cancelPrompt(display, "running") }, { WorkspaceCommand.steer(display, "running", "change") },
            { WorkspaceCommand.approve(display, approval, "deny") }, { WorkspaceCommand.subtab(display, "subtab_select", tab) }))
            assertTrue(runCatching { operation() }.isFailure)
        for (command in listOf(WorkspaceCommand.cancelPrompt(projection, "running"), WorkspaceCommand.steer(projection, "running", "change"),
            WorkspaceCommand.approve(projection, approval, "deny"))) {
            assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(next, true, true, command))
        }
    }
    @Test fun authorityLossNeverResolvesOrReplacesUnknownWritesAndRevokes() {
        val command = WorkspaceCommand.prompt(projection, "hello")
        val next = AuthorityLossPolicy.displayOnly(state.copy(pendingWorkspace = command))
        assertEquals(command.envelope, next.pendingWorkspace?.envelope)
        assertTrue(WorkspaceDispatchPolicy.allows(next, true, true, WorkspaceCommand.restore(command.envelope)))
        assertFalse(WorkspaceDispatchPolicy.allows(next, true, true, WorkspaceCommand.prompt(projection, "hello")))
        val revoking = AuthorityLossPolicy.displayOnly(state.copy(pendingRevoke = "unconfirmed"))
        assertEquals("unconfirmed", revoking.pendingRevoke)
        assertFalse(WorkspaceDispatchPolicy.allows(revoking, true, true, command))
    }
    @Test fun errorPhaseRejectsPrebuiltControlsWhoseOwnerRevisionGrantOrTargetChanged() {
        val cancel = WorkspaceCommand.cancelPrompt(projection, "running")
        val steer = WorkspaceCommand.steer(projection, "running", "change")
        val approve = WorkspaceCommand.approve(projection, approval, "deny")
        for (changed in listOf(projection.copy(workspaceOwner = "f".repeat(64)), projection.copy(workspaceRevision = "5"),
            projection.copy(sessionId = "replacement"), projection.copy(selectedId = "foreign"))) {
            for (command in listOf(cancel, steer, approve))
                assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = changed), true, true, command))
        }
        for (changed in listOf(projection.copy(projects = projection.projects.map { it.copy(promptControl = false) }),
            projection.copy(projects = projection.projects.map { it.copy(promptStatuses = emptyList()) }))) {
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = changed), true, true, cancel))
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = changed), true, true, steer))
        }
        for (changed in listOf(projection.copy(projects = projection.projects.map { it.copy(approvalControl = false) }),
            projection.copy(projects = projection.projects.map { it.copy(pendingApprovals = listOf(approval.copy(digest = "f".repeat(64)))) })))
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = changed), true, true, approve))
    }
    @Test fun unavailableCacheFailsClosedWithoutDroppingUnresolvedIdentity() {
        val command = WorkspaceCommand.prompt(projection, "hello")
        val corrupt = state.copy(projection = projection.copy(eventSequence = null), pendingWorkspace = command)
        val next = AuthorityLossPolicy.displayOnly(corrupt)
        assertNull(next.projection); assertEquals(corrupt.host, next.host)
        assertEquals(command.envelope, next.pendingWorkspace?.envelope)
        assertNull(AuthorityLossPolicy.displayOnly(state.copy(projection = null)).projection)
    }
    @Test fun removedProjectFencesOldControlWithoutClaimingRevocationOrResolvingPendingWrite() {
        val error = SelectedProjectUnavailableException("project")
        assertTrue(AuthorityLossPolicy.refused(error))
        assertFalse(AuthorityLossPolicy.refused(WireException(error.message.orEmpty())))
        val command = WorkspaceCommand.prompt(projection, "unresolved")
        val next = AuthorityLossPolicy.displayOnly(state.copy(pendingWorkspace = command))
        assertEquals(command.envelope, next.pendingWorkspace?.envelope)
        assertEquals(state.host, next.host); assertEquals(projection.turns, next.projection?.turns)
        assertNull(next.projection?.workspaceOwner)
        assertTrue(requireNotNull(next.projection).projects.all { !it.workspaceControl && !it.promptControl && !it.approvalControl })
        assertTrue(WorkspaceDispatchPolicy.allows(next, true, true, WorkspaceCommand.restore(command.envelope)))
        assertFalse(WorkspaceDispatchPolicy.allows(next, true, true, WorkspaceCommand.prompt(projection, "new")))
    }
    @Test fun correlatedWireRefusalFencesWithoutPublishingUnconfirmedEvents() = kotlinx.coroutines.runBlocking {
        val response = com.google.gson.JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"subscription","execution_state":"failed","code":"unauthorized","payload":{"type":"response","result":null}}""").asJsonObject
        val event = com.google.gson.JsonParser.parseString("""{"schema_version":1,"channel":"event","project_id":"project","session_id":"session","turn_id":"late","sequence":8,"payload":{"type":"event","name":"transcript_turn","data":{"id":"late","role":"assistant","content":"UNCONFIRMED"}}}""").asJsonObject
        val bytes = java.io.ByteArrayOutputStream().also { FrameCodec.write(it, event); FrameCodec.write(it, response) }.toByteArray()
        val request = com.google.gson.JsonParser.parseString("""{"id":"subscription","payload":{"type":"subscribe"}}""").asJsonObject
        var visible = state
        var publications = 0
        val error = runCatching {
            ResponseReader.read(java.io.ByteArrayInputStream(bytes), request, { envelope ->
                if (FrameCodec.string(envelope, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(envelope, "code"))
            }) { publications++ }
        }.exceptionOrNull()
        assertTrue(error is HostRefusedException)
        assertTrue(AuthorityLossPolicy.refused(requireNotNull(error) as Exception))
        visible = AuthorityLossPolicy.displayOnly(visible)
        assertEquals(0, publications)
        assertEquals(projection.turns, visible.projection?.turns)
        assertEquals("8", visible.projection?.eventSequence)
        assertTrue(requireNotNull(visible.projection).projects.single().pendingApprovals.isEmpty())
    }
    @Test fun pinnedHostFailureStripsPrebuiltDecisionAuthorityButRetainsUnknownRequestIdentity() {
        val error = javax.net.ssl.SSLHandshakeException("TLS refused").apply { initCause(HostIdentityException()) }
        assertTrue(AuthorityLossPolicy.refused(error))
        val next = AuthorityLossPolicy.displayOnly(state)
        for (command in listOf(WorkspaceCommand.cancelPrompt(projection, "running"),
            WorkspaceCommand.steer(projection, "running", "change"), WorkspaceCommand.approve(projection, approval, "deny"))) {
            assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(next, true, true, command))
        }
        val unknown = WorkspaceCommand.prompt(projection, "unconfirmed")
        val pending = AuthorityLossPolicy.displayOnly(state.copy(pendingWorkspace = unknown))
        assertEquals(unknown.envelope, pending.pendingWorkspace?.envelope)
        assertEquals(state.host, pending.host)
        assertEquals("8", pending.projection?.eventSequence)
        assertTrue(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.restore(unknown.envelope)))
        assertEquals("revoke", AuthorityLossPolicy.displayOnly(state.copy(pendingRevoke = "revoke")).pendingRevoke)
    }
    @Test fun protocolNegotiationFailureRevokesOldDecisionsWithoutResolvingUnknownWrites() {
        val failure = runCatching { HandshakePrelude.applicationProtocol("zenpi/2") }.exceptionOrNull() as Exception
        assertTrue(AuthorityLossPolicy.refused(failure))
        val fenced = AuthorityLossPolicy.displayOnly(state)
        for (command in listOf(WorkspaceCommand.cancelPrompt(projection, "running"),
            WorkspaceCommand.steer(projection, "running", "change"), WorkspaceCommand.approve(projection, approval, "deny"))) {
            assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(fenced, true, true, command))
        }
        val unknown = WorkspaceCommand.prompt(projection, "unconfirmed")
        val pending = AuthorityLossPolicy.displayOnly(state.copy(pendingWorkspace = unknown))
        assertEquals(unknown.envelope, pending.pendingWorkspace?.envelope)
        assertEquals(state.host, pending.host)
        assertEquals(projection.turns, pending.projection?.turns)
        assertEquals("8", pending.projection?.eventSequence)
        assertTrue(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.restore(unknown.envelope)))
        assertFalse(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.prompt(projection, "new")))
    }
    @Test fun localIdentityAdmissionFailureStopsRetryAndDropsAuthorityWithoutResolvingRequests() {
        val failure = HostAuthenticationException()
        assertTrue(AuthorityLossPolicy.refused(failure))
        assertTrue(zenpi.lifecycle.ResumeRetryPolicy.permanent(failure))
        assertFalse(AuthorityLossPolicy.refused(WireException(failure.message.orEmpty())))
        val pending = WorkspaceCommand.prompt(projection, "unconfirmed")
        val fenced = AuthorityLossPolicy.displayOnly(state.copy(pendingWorkspace = pending))
        assertEquals(pending.envelope, fenced.pendingWorkspace?.envelope)
        assertEquals(state.host, fenced.host)
        assertEquals(projection.turns, fenced.projection?.turns)
        assertNull(fenced.projection?.workspaceOwner)
        assertTrue(requireNotNull(fenced.projection).projects.all { !it.promptControl && !it.approvalControl })
    }
    @Test fun unconfirmedAuthenticationRetainsDisplayButCannotKeepOldDecisions() {
        val result = com.google.gson.JsonParser.parseString("""{"authenticated":false,"device_id":"${"e".repeat(64)}","control_available":false}""").asJsonObject
        val failure = runCatching { AuthenticationReceipt.validate(result, "e".repeat(64)) }.exceptionOrNull() as Exception
        assertTrue(AuthorityLossPolicy.refused(failure))
        val unknown = WorkspaceCommand.prompt(projection, "unconfirmed")
        val fenced = AuthorityLossPolicy.displayOnly(state.copy(pendingWorkspace = unknown))
        assertEquals(state.host, fenced.host)
        assertEquals(projection.turns, fenced.projection?.turns)
        assertEquals("8", fenced.projection?.eventSequence)
        assertEquals(unknown.envelope, fenced.pendingWorkspace?.envelope)
        assertTrue(WorkspaceDispatchPolicy.allows(fenced, true, true, WorkspaceCommand.restore(unknown.envelope)))
        val display = fenced.copy(pendingWorkspace = null)
        for (command in listOf(WorkspaceCommand.cancelPrompt(projection, "running"),
            WorkspaceCommand.steer(projection, "running", "change"), WorkspaceCommand.approve(projection, approval, "deny"))) {
            assertFalse(WorkspaceDispatchPolicy.allows(display, true, true, command))
        }
    }
}
