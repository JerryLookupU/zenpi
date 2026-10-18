package zenpi

import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Assume.assumeNotNull
import org.junit.Test
import zenpi.transport.*
import zenpi.pairing.*

/** Current host approval rendezvous; not camera, TLS or Keystore acceptance. */
class AndroidRustApprovalTest {
    @Test fun projectedApprovalCommandsAndActualReceiptsRetainDecisionIdentity() = runBlocking {
        val phase = System.getProperty("zenpi.approval.phase")
        assumeNotNull(phase)
        fun file(name: String) = java.io.File(requireNotNull(System.getProperty("zenpi.approval.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val decision = seed["decision"].asString
        val listing = seed.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing)
        val selected = projects.single()
        val projection = HostProjectionCodec.workspace(
            HostProjectionCodec.snapshot(projects, selected, seed.getAsJsonObject("snapshot")), listing)
        val approval = selected.pendingApprovals.single()
        assertTrue(selected.approvalControl)
        val live = PairingState(phase = PairingPhase.CONNECTED, projection = projection)
        assertTrue(ApprovalDecisionPolicy.allows(live, approval))
        for (bad in listOf(approval.copy(tool = "bad\n"), approval.copy(origin = "unknown"),
            approval.copy(preview = RemoteApprovalPreview("note.txt", "bad\u001b", true, false, false, false)))) {
            val changed = projection.copy(projects = listOf(selected.copy(pendingApprovals = listOf(bad))))
            assertFalse(ApprovalDecisionPolicy.allows(live.copy(projection = changed), bad))
            assertTrue(runCatching { WorkspaceCommand.approve(changed, bad, decision) }.isFailure)
        }
        assertFalse(ApprovalDecisionPolicy.allows(live.copy(scanInProgress = true), approval))
        val ambiguous = projection.copy(projects = projects + selected.copy(sessionId = "foreign"))
        assertFalse(ApprovalProjectionCodec.current(ambiguous, approval))
        assertTrue(runCatching { WorkspaceCommand.approve(ambiguous, approval, decision) }.isFailure)
        val duplicate = projection.copy(projects = listOf(selected.copy(pendingApprovals = listOf(approval, approval.copy(digest = "0".repeat(64))))))
        assertTrue(runCatching { WorkspaceCommand.approve(duplicate, approval, decision) }.isFailure)
        assertEquals("write_file", approval.tool)
        assertEquals("workspace_write", approval.sideEffect)
        assertFalse(seed.toString().contains("INTEROP_APPROVAL_PRIVATE"))
        if (phase == "generate") {
            val command = WorkspaceCommand.approve(projection, approval, decision)
            file("command").outputStream().use { FrameCodec.write(it, command.envelope) }
            return@runBlocking
        }
        assertEquals("validate", phase)
        val evidence = JsonParser.parseString(file("results").readText()).asJsonObject
        val exchanges = evidence["exchanges"].asJsonArray
        val codes = listOf("invalid_request", "owner_mismatch", "revision_conflict", "unauthorized",
            null, null, "request_id_conflict", null)
        assertEquals(codes.size, exchanges.size())
        for ((index, raw) in exchanges.withIndex()) {
            val exchange = raw.asJsonObject
            val request = exchange.getAsJsonObject("request")
            val command = WorkspaceCommand.restore(request)
            val wire = java.util.Base64.getDecoder().decode(exchange["wire_base64"].asString)
            var confirmed = false
            val error = runCatching {
                ResponseReader.read(object : java.io.ByteArrayInputStream(wire) {
                    override fun read(bytes: ByteArray, offset: Int, length: Int) = super.read(bytes, offset, minOf(length, 2))
                }, request, { response ->
                    if (FrameCodec.string(response, "execution_state") == "failed")
                        throw HostRefusedException(FrameCodec.string(response, "code"))
                    val receipt = WorkspaceReceiptCodec.validate(command, response)
                    assertEquals(approval.requestId, receipt["target_request_id"].asString)
                    assertEquals(approval.digest, receipt["digest"].asString)
                    assertEquals(decision, receipt["decision"].asString)
                    confirmed = true
                })
            }.exceptionOrNull()
            if (codes[index] == null) { assertNull(error); assertTrue(confirmed) }
            else {
                assertFalse(confirmed); assertTrue(error is HostRefusedException)
                assertEquals(codes[index], (error as HostRefusedException).code)
            }
        }
        assertEquals(exchanges[4].asJsonObject["wire_base64"], exchanges[5].asJsonObject["wire_base64"])
        assertEquals(exchanges[4].asJsonObject["wire_base64"], exchanges[7].asJsonObject["wire_base64"])
        val accepted = exchanges[4].asJsonObject
        val request = accepted.getAsJsonObject("request")
        val bytes = java.util.Base64.getDecoder().decode(accepted["wire_base64"].asString)
        val response = requireNotNull(FrameCodec.read(java.io.ByteArrayInputStream(bytes)))
        for (field in listOf("request_id", "target_request_id", "project_id", "session_id", "digest", "decision")) {
            val altered = response.deepCopy().apply { getAsJsonObject("payload").getAsJsonObject("result").addProperty(field, "foreign") }
            assertTrue(field, runCatching { WorkspaceReceiptCodec.validate(WorkspaceCommand.restore(request), altered) }.isFailure)
        }
        val refreshed = HostProjectionCodec.projects(evidence.getAsJsonObject("listing")).single()
        assertEquals(selected.id, refreshed.id); assertEquals(selected.sessionId, refreshed.sessionId)
        assertTrue(refreshed.pendingApprovals.isEmpty())
    }
}
