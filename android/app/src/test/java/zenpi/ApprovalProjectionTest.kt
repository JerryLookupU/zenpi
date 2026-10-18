package zenpi

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.state.ProjectionCacheCodec

class ApprovalProjectionTest {
    @Test fun approvalConstructionCannotBorrowAuthorityFromAReplacementSession() {
        val target = ApprovalProjectionCodec.decode(JsonArray().apply { add(approval()) }, "project", "session").single()
        val project = RemoteProject("project", "Workspace", "session", pendingApprovals = listOf(target), approvalControl = true)
        val live = HostProjection(listOf(project), "project", "session", "running", emptyList(), "1", false, "0",
            workspaceOwner = "b".repeat(64), workspaceRevision = "0")
        for (changed in listOf(live.copy(sessionId = "replacement"), live.copy(selectedId = "foreign"),
            live.copy(projects = listOf(project.copy(sessionId = "replacement"))),
            live.copy(projects = listOf(project.copy(approvalControl = false))))) {
            for (decision in listOf("allow", "deny")) assertTrue(runCatching {
                WorkspaceCommand.approve(changed, target, decision)
            }.isFailure)
        }
        val command = WorkspaceCommand.approve(live, target, "deny")
        assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        assertEquals("session", command.envelope["session_id"].asString)
        val replacement = target.copy(sessionId = "replacement")
        val fresh = live.copy(sessionId = "replacement", projects = listOf(project.copy(sessionId = "replacement", pendingApprovals = listOf(replacement))))
        assertEquals("replacement", WorkspaceCommand.approve(fresh, replacement, "deny").envelope["session_id"].asString)
        assertTrue(runCatching { WorkspaceCommand.approve(fresh, target, "allow") }.isFailure)
    }
    @Test fun sameLiveDeviceCanComposePromptAndApprovalButNeitherCapabilityImpliesTheOther() {
        val target=ApprovalProjectionCodec.decode(JsonArray().apply{add(approval())},"project","session").single()
        val project=RemoteProject("project","Workspace","session",promptControl=true,pendingApprovals=listOf(target),approvalControl=true)
        val live=HostProjection(listOf(project),"project","session","idle",emptyList(),"1",false,"0",
            workspaceOwner="b".repeat(64),workspaceRevision="0")
        val prompt=WorkspaceCommand.prompt(live,"write the note").envelope
        val decision=WorkspaceCommand.approve(live,target,"allow").envelope
        assertEquals("prompt",prompt["payload"].asJsonObject["operation"].asString)
        assertEquals("approve",decision["payload"].asJsonObject["operation"].asString)
        assertNotEquals(prompt["id"],decision["id"])
        for(noPrompt in listOf(true,false)) {
            val restricted=live.copy(projects=listOf(project.copy(promptControl=!noPrompt,approvalControl=noPrompt)))
            assertEquals(noPrompt,runCatching{WorkspaceCommand.prompt(restricted,"write")}.isFailure)
            assertEquals(!noPrompt,runCatching{WorkspaceCommand.approve(restricted,target,"allow")}.isFailure)
        }
        val cached=ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(live))
        assertFalse(cached.projects.single().promptControl);assertFalse(cached.projects.single().approvalControl)
        assertTrue(runCatching{WorkspaceCommand.prompt(cached,"write")}.isFailure)
        assertTrue(runCatching{WorkspaceCommand.approve(cached,target,"allow")}.isFailure)
    }
    private fun approval() = JsonObject().apply {
        addProperty("request_id", "approval"); addProperty("project_id", "project"); addProperty("session_id", "session")
        addProperty("turn_id", "turn"); addProperty("call_id", "call"); addProperty("tool", "write_file")
        addProperty("side_effect", "workspace_write"); addProperty("origin", "agent_tool"); addProperty("digest", "a".repeat(64))
    }
    @Test fun workerIdentityScopesDuplicateIdsAndSurvivesExactPendingRequestRestore() {
        fun worker(session: String) = approval().apply {
            addProperty("worker_session_id", session); addProperty("subtab_id", "c".repeat(64))
        }
        val targets = ApprovalProjectionCodec.decode(JsonArray().apply {
            add(approval()); add(worker("worker-one")); add(worker("worker-two"))
        }, "project", "session")
        assertEquals(3, targets.size)
        val live = HostProjection(listOf(RemoteProject("project", "Workspace", "session", pendingApprovals = targets, approvalControl = true)),
            "project", "session", "running", emptyList(), "1", false, "0", workspaceOwner = "b".repeat(64), workspaceRevision = "0")
        for (target in targets) for (decision in listOf("allow", "deny")) {
            val command = WorkspaceCommand.approve(live, target, decision)
            val arguments = command.envelope["payload"].asJsonObject["arguments"].asJsonObject
            assertEquals(target.workerSessionId, arguments["worker_session_id"]?.asString)
            assertEquals(if (target.workerSessionId == null) 3 else 4, arguments.size())
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        }
        assertTrue(runCatching { WorkspaceCommand.approve(live, targets[1].copy(workerSessionId = "foreign"), "allow") }.isFailure)
        assertTrue(ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(live)).projects.single().pendingApprovals.isEmpty())
        assertTrue(runCatching { ApprovalProjectionCodec.decode(JsonArray().apply {
            add(worker("worker-one")); add(worker("worker-one"))
        }, "project", "session") }.isFailure)
    }
    @Test fun workerProjectionAndSavedCommandRejectPartialOrInvalidScope() {
        fun worker() = approval().apply {
            addProperty("worker_session_id", "worker-one"); addProperty("subtab_id", "c".repeat(64))
        }
        for (change in listOf<(JsonObject) -> Unit>(
            { it.remove("worker_session_id") }, { it.remove("subtab_id") },
            { it.addProperty("worker_session_id", "") }, { it.addProperty("worker_session_id", "bad\n") },
            { it.addProperty("worker_session_id", "x".repeat(129)) }, { it.addProperty("worker_session_id", 1) },
            { it.addProperty("subtab_id", "foreign") }, { it.addProperty("remember", true) })) {
            val item = worker(); change(item)
            assertTrue(runCatching { ApprovalProjectionCodec.decode(JsonArray().apply { add(item) }, "project", "session") }.isFailure)
        }
        val target = ApprovalProjectionCodec.decode(JsonArray().apply { add(worker()) }, "project", "session").single()
        val live = HostProjection(listOf(RemoteProject("project", "Workspace", "session", pendingApprovals = listOf(target), approvalControl = true)),
            "project", "session", "running", emptyList(), "1", false, "0", workspaceOwner = "b".repeat(64), workspaceRevision = "0")
        for (invalid in listOf("", "bad\n", "x".repeat(129))) {
            val envelope = WorkspaceCommand.approve(live, target, "deny").envelope
            envelope["payload"].asJsonObject["arguments"].asJsonObject.addProperty("worker_session_id", invalid)
            assertTrue(runCatching { WorkspaceCommand.restore(envelope) }.isFailure)
        }
    }
    @Test fun exactFieldsOwnerDigestAndCapacityAreRequired() {
        assertEquals("write_file", ApprovalProjectionCodec.decode(JsonArray().apply { add(approval()) }, "project", "session").single().tool)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("project_id", "foreign") }, { it.addProperty("session_id", "foreign") },
            { it.addProperty("side_effect", "shell") }, { it.addProperty("origin", "network") },
            { it.addProperty("digest", "bad") }, { it.addProperty("tool", "bad\n") },
            { it.addProperty("arguments", "secret") }, { it.addProperty("preview", "secret") })) {
            val item = approval(); change(item)
            assertTrue(runCatching { ApprovalProjectionCodec.decode(JsonArray().apply { add(item) }, "project", "session") }.isFailure)
        }
        assertTrue(runCatching { ApprovalProjectionCodec.decode(JsonArray().apply { add(approval()); add(approval()) }, "project", "session") }.isFailure)
        assertTrue(runCatching { ApprovalProjectionCodec.decode(JsonArray().apply { repeat(17) { add(approval().apply { addProperty("request_id", "r-$it") }) } }, "project", "session") }.isFailure)
    }
    @Test fun pendingApprovalIsNeverPersistedAsAnOfflineDecisionTarget() {
        val approvals = ApprovalProjectionCodec.decode(JsonArray().apply { add(approval()) }, "project", "session")
        val project = RemoteProject("project", "Workspace", "session", pendingApprovals = approvals)
        val live = HostProjection(listOf(project), "project", "session", "running", emptyList(), "1", false, "0")
        val encoded = ProjectionCacheCodec.encode(live)
        assertFalse(encoded.toString().contains("write_file"))
        assertTrue(ProjectionCacheCodec.decode(encoded).projects.single().pendingApprovals.isEmpty())
    }
    @Test fun decisionRequiresSeparateLiveAuthorityAndPersistsExactDigestAndChoice() {
        val target = ApprovalProjectionCodec.decode(JsonArray().apply { add(approval()) }, "project", "session").single()
        val live = HostProjection(listOf(RemoteProject("project", "Workspace", "session", pendingApprovals = listOf(target), approvalControl = true)),
            "project", "session", "running", emptyList(), "1", false, "0", workspaceOwner = "b".repeat(64), workspaceRevision = "0")
        for (decision in listOf("allow", "deny")) {
            val command = WorkspaceCommand.approve(live, target, decision)
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            assertEquals(target.digest, command.envelope["payload"].asJsonObject["arguments"].asJsonObject["digest"].asString)
            assertEquals(decision, command.envelope["payload"].asJsonObject["arguments"].asJsonObject["decision"].asString)
        }
        assertTrue(runCatching { WorkspaceCommand.approve(live, target.copy(digest = "c".repeat(64)), "allow") }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.approve(live, target, "remember") }.isFailure)
        val cached = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(live))
        assertFalse(cached.projects.single().approvalControl)
        assertTrue(runCatching { WorkspaceCommand.approve(cached, target, "allow") }.isFailure)
    }
    @Test fun boundedPreviewHasStrictFlagsAndCannotLeakThroughOfflineCache() {
        fun preview() = JsonObject().apply {
            addProperty("kind", "diff"); addProperty("path", "note.txt"); addProperty("patch", "+hello\n+API_KEY=<redacted>")
            addProperty("changed", true); addProperty("truncated", false); addProperty("redacted", true); addProperty("withheld", false)
        }
        val item = approval().apply { add("preview", preview()) }
        val target = ApprovalProjectionCodec.decode(JsonArray().apply { add(item) }, "project", "session").single()
        assertTrue(requireNotNull(target.preview).redacted)
        val live = HostProjection(listOf(RemoteProject("project", "Workspace", "session", pendingApprovals = listOf(target), approvalControl = true)),
            "project", "session", "running", emptyList(), "1", false, "0")
        val encoded = ProjectionCacheCodec.encode(live)
        assertFalse(encoded.toString().contains("note.txt"))
        assertFalse(encoded.toString().contains("API_KEY"))
        assertTrue(ProjectionCacheCodec.decode(encoded).projects.single().pendingApprovals.isEmpty())
        assertFalse(ProjectionCacheCodec.decode(encoded).projects.single().approvalControl)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("patch", "x".repeat(2049)) }, { it.addProperty("patch", "bad\u001b") },
            { it.addProperty("path", "bad\n") }, { it.addProperty("withheld", true) },
            { it.addProperty("changed", "true") }, { it.addProperty("arguments", "secret") })) {
            val view = preview(); change(view)
            assertTrue(runCatching { ApprovalProjectionCodec.decode(JsonArray().apply { add(approval().apply { add("preview", view) }) }, "project", "session") }.isFailure)
        }
    }
}
