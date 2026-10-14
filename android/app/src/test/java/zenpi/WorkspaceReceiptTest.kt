package zenpi

import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class WorkspaceReceiptTest {
    @Test fun steeringReceiptConfirmsOnlyDurableOriginalTaskAdmission() {
        val projection = live.copy(projects = listOf(live.projects.single().copy(promptStatuses = listOf(
            RemotePromptStatus("target", project, "session", "running")))))
        val command = WorkspaceCommand.steer(projection, "target", "hello")
        val result = JsonObject().apply {
            addProperty("request_id", command.envelope["id"].asString); addProperty("target_request_id", "target")
            addProperty("project_id", project); addProperty("session_id", "session")
        }
        assertEquals(result, WorkspaceReceiptCodec.validate(command, response(command, result, "accepted")))
        for (state in listOf("succeeded", "unknown_outcome", "running"))
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, response(command, result, state)) }.isFailure)
        for (field in result.keySet()) {
            val invalid = result.deepCopy(); invalid.addProperty(field, "foreign")
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, response(command, invalid, "accepted")) }.isFailure)
        }
    }
    private val project = "b".repeat(64)
    private val tab = "c".repeat(64)
    private val live = HostProjection(listOf(RemoteProject(project, "Workspace", "session", true, true)), project, "session", "idle",
        emptyList(), "1", false, workspaceOwner = "a".repeat(64), workspaceRevision = "4",
        subtabs = RemoteSubtabs(project, "2", tab, listOf(RemoteSubtab(tab, "main", "main", 1)), true))
    private fun response(command: WorkspaceCommand, result: JsonObject, state: String = "succeeded") = JsonObject().apply {
        addProperty("id", command.envelope["id"].asString); addProperty("channel", "control"); addProperty("execution_state", state)
        add("payload", JsonObject().apply { addProperty("type", "response"); add("result", result) })
    }
    private fun workspaceResult() = JsonObject().apply {
        addProperty("workspace_revision", 5); addProperty("active_project_id", project); addProperty("required_owner", "a".repeat(64))
    }
    @Test fun mutationReceiptRequiresExactOwnerIdentityRevisionAndShape() {
        val command = WorkspaceCommand.create(live, project, "workspace_select")
        val valid = response(command, workspaceResult())
        assertEquals(workspaceResult(), WorkspaceReceiptCodec.validate(command, valid))
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("id", "foreign") }, { it.addProperty("channel", "event") },
            { it.addProperty("execution_state", "accepted") }, { it.addProperty("execution_state", "unknown_outcome") },
            { it["payload"].asJsonObject["result"].asJsonObject.addProperty("required_owner", "d".repeat(64)) },
            { it["payload"].asJsonObject["result"].asJsonObject.addProperty("workspace_revision", 6) },
            { it["payload"].asJsonObject["result"].asJsonObject.addProperty("workspace_revision", "5") },
            { it["payload"].asJsonObject["result"].asJsonObject.addProperty("extra", true) })) {
            val invalid = valid.deepCopy(); change(invalid)
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, invalid) }.isFailure)
        }
        val subtab = WorkspaceCommand.subtab(live, "subtab_rename", tab, "renamed")
        val result = workspaceResult().apply { addProperty("project_id", project); addProperty("active_subtab_id", tab); addProperty("subtab_revision", 3) }
        assertEquals(result, WorkspaceReceiptCodec.validate(subtab, response(subtab, result)))
        for (field in listOf("project_id", "active_subtab_id", "subtab_revision")) {
            val invalid = result.deepCopy(); invalid.addProperty(field, "foreign")
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(subtab, response(subtab, invalid)) }.isFailure)
        }
    }
    @Test fun admissionReceiptRequiresOriginalPromptAndApprovalTarget() {
        val command = WorkspaceCommand.prompt(live, "hello")
        val result = JsonObject().apply { addProperty("request_id", command.envelope["id"].asString); addProperty("project_id", project)
            addProperty("session_id", "session"); addProperty("job_id", "job") }
        assertEquals(result, WorkspaceReceiptCodec.validate(command, response(command, result, "accepted")))
        for (field in listOf("request_id", "project_id", "session_id", "job_id")) {
            val invalid = result.deepCopy(); invalid.addProperty(field, if (field == "job_id") "" else "foreign")
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, response(command, invalid, "accepted")) }.isFailure)
        }
        val target = RemoteApproval("target", project, "session", "turn", "call", "write_file", "workspace_write", "agent_tool", "d".repeat(64))
        val approval = WorkspaceCommand.approve(live.copy(projects = listOf(live.projects.single().copy(approvalControl = true, pendingApprovals = listOf(target)))), target, "deny")
        val receipt = result.deepCopy().apply { remove("job_id"); addProperty("request_id", approval.envelope["id"].asString)
            addProperty("target_request_id", "target"); addProperty("digest", target.digest); addProperty("decision", "deny") }
        assertEquals(receipt, WorkspaceReceiptCodec.validate(approval, response(approval, receipt, "accepted")))
        for (field in listOf("target_request_id", "digest", "decision")) {
            val invalid = receipt.deepCopy(); invalid.addProperty(field, "foreign")
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(approval, response(approval, invalid, "accepted")) }.isFailure)
        }
    }
    @Test fun selectReceiptMustActivateTheRequestedTabAndCloseMustNotRetainIt() {
        val secondary = "d".repeat(64)
        val projection = live.copy(subtabs = live.subtabs!!.copy(directoryControl = true,
            tabs = live.subtabs!!.tabs + RemoteSubtab(secondary, "Directory", "in_place", 1)))
        fun result(active: String) = workspaceResult().apply {
            addProperty("project_id", project); addProperty("active_subtab_id", active); addProperty("subtab_revision", 3)
        }
        val select = WorkspaceCommand.subtab(projection, "subtab_select", secondary)
        val close = WorkspaceCommand.subtab(projection, "subtab_close", secondary)
        assertEquals(result(secondary), WorkspaceReceiptCodec.validate(select, response(select, result(secondary))))
        assertEquals(result(tab), WorkspaceReceiptCodec.validate(close, response(close, result(tab))))
        for (active in listOf(tab, "e".repeat(64)))
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(select, response(select, result(active))) }.isFailure)
        assertTrue(runCatching { WorkspaceReceiptCodec.validate(close, response(close, result(secondary))) }.isFailure)
        // Renaming a nonselected tab legitimately retains the old selection.
        val rename = WorkspaceCommand.subtab(projection, "subtab_rename", secondary, "Renamed")
        assertEquals(result(tab), WorkspaceReceiptCodec.validate(rename, response(rename, result(tab))))
    }
    @Test fun fragmentedWrongTabReceiptNeverAcknowledgesThePendingSelection() = kotlinx.coroutines.runBlocking {
        val command = WorkspaceCommand.subtab(live, "subtab_select", tab)
        val wrong = workspaceResult().apply {
            addProperty("project_id", project); addProperty("active_subtab_id", "e".repeat(64)); addProperty("subtab_revision", 3)
        }
        val frame = response(command, wrong).apply { addProperty("schema_version", 1) }
        val wire = java.io.ByteArrayOutputStream().also { FrameCodec.write(it, frame) }.toByteArray()
        val input = object : java.io.ByteArrayInputStream(wire) {
            override fun read(target: ByteArray, offset: Int, length: Int): Int = super.read(target, offset, minOf(length, 3))
        }
        var acknowledged = false
        val error = runCatching {
            ResponseReader.read(input, command.envelope, { WorkspaceReceiptCodec.validate(command, it); acknowledged = true })
        }.exceptionOrNull()
        assertTrue(error is WireException); assertFalse(acknowledged)
        assertEquals(tab, command.envelope["payload"].asJsonObject["arguments"].asJsonObject["subtab_id"].asString)
    }
    @Test fun workerApprovalKeepsCorrelatedMasterReceiptShapeForExactRetry() {
        val target = RemoteApproval("target", project, "session", "turn", "call", "write_file", "workspace_write", "agent_tool", "d".repeat(64),
            workerSessionId = "worker-one", subtabId = tab)
        val projection = live.copy(projects = listOf(live.projects.single().copy(approvalControl = true, pendingApprovals = listOf(target))))
        val command = WorkspaceCommand.approve(projection, target, "deny")
        val receipt = JsonObject().apply {
            addProperty("request_id", command.envelope["id"].asString); addProperty("target_request_id", target.requestId)
            addProperty("project_id", project); addProperty("session_id", "session")
            addProperty("digest", target.digest); addProperty("decision", "deny")
        }
        assertEquals(receipt, WorkspaceReceiptCodec.validate(WorkspaceCommand.restore(command.envelope), response(command, receipt, "accepted")))
        val foreign = receipt.deepCopy().apply { addProperty("session_id", target.workerSessionId) }
        assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, response(command, foreign, "accepted")) }.isFailure)
    }
}
