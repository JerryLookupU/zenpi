package zenpi

import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Assume.assumeNotNull
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

/** Current owner runner, durable steering and cancellation, not physical transport evidence. */
class AndroidRustPromptTest {
    @Test fun generatedPromptAndCancellationControlOneRealOwnerAndRetainRestartReceipts() = runBlocking {
        val phase = System.getProperty("zenpi.prompt.phase")
        assumeNotNull(phase)
        fun file(name: String) = java.io.File(requireNotNull(System.getProperty("zenpi.prompt.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val listing = seed.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing)
        val project = projects.single()
        val projection = HostProjectionCodec.workspace(HostProjectionCodec.snapshot(projects, project, seed.getAsJsonObject("snapshot")), listing)
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection)
        val ambiguous = projection.copy(projects = projects + project.copy(sessionId = "foreign"))
        assertTrue(runCatching { WorkspaceCommand.prompt(ambiguous, "ANDROID_OWNER_PROMPT") }.isFailure)
        if (phase == "prompt") {
            assertTrue(project.workerControl)
            val tabs = SubtabProjectionCodec.decode(seed.getAsJsonObject("subtabs"), project, listing)
            val workerProjection = projection.copy(subtabs = tabs)
            val workerState = state.copy(projection = workerProjection)
            assertTrue(PromptControlPolicy.workerSend(workerState))
            val worker = WorkspaceCommand.workerPrompt(workerProjection, "review active tab")
            assertTrue(WorkspaceDispatchPolicy.allows(workerState, true, true, worker))
            val extra = RemoteSubtab("e".repeat(64), "Extra", "in_place", 1)
            val invalid = workerProjection.copy(subtabs = tabs.copy(tabs = tabs.tabs + extra + extra))
            assertFalse(PromptControlPolicy.workerSend(workerState.copy(projection = invalid)))
            assertTrue(runCatching { WorkspaceCommand.workerPrompt(invalid, "review") }.isFailure)
            assertFalse(WorkspaceDispatchPolicy.allows(workerState.copy(projection = invalid), true, true, worker))
            assertTrue(WorkspaceDispatchPolicy.allows(workerState.copy(projection = invalid, pendingWorkspace = worker), true, true, worker))
            assertTrue(PromptControlPolicy.send(state))
            val command = WorkspaceCommand.prompt(projection, "ANDROID_OWNER_PROMPT")
            assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
            file("command").outputStream().use { FrameCodec.write(it, command.envelope) }
            return@runBlocking
        }
        if (phase == "approve") {
            assertTrue(project.approvalControl); assertFalse(project.promptControl)
            val target = project.pendingApprovals.single()
            assertEquals("write_file", target.tool)
            assertEquals(seed["approval_path"].asString, requireNotNull(target.preview).path)
            assertTrue(ApprovalDecisionPolicy.allows(state, target))
            assertFalse(ApprovalDecisionPolicy.allows(state.copy(scanInProgress = true), target))
            val command = WorkspaceCommand.approve(projection, target, seed["approval_decision"].asString)
            assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
            file("command").outputStream().use { FrameCodec.write(it, command.envelope) }
            return@runBlocking
        }
        if (phase == "cancel" || phase == "steer") {
            val target = seed["target"].asString
            val active = requireNotNull(PromptStatusCodec.active(projection, target))
            assertEquals("running", active.state)
            val expectedStream = "STAGE2_PARTIAL_BEFORE_COMPLETION\n".repeat(if (phase == "cancel") 2 else 1)
            assertEquals(expectedStream, active.streamText)
            assertTrue(PromptControlPolicy.active(state.copy(phase = PairingPhase.ERROR), target))
            assertFalse(PromptControlPolicy.active(state.copy(scanInProgress = true), target))
            assertTrue(runCatching { WorkspaceCommand.cancelPrompt(ambiguous, target) }.isFailure)
            val command = if (phase == "steer") WorkspaceCommand.steer(projection, target, "ANDROID_OWNER_STEERING")
                else WorkspaceCommand.cancelPrompt(projection, target)
            for (bad in listOf(active.copy(failureCode = "internal_error"), active.copy(streamText = "x".repeat(4097)),
                active.copy(streamText = "bad\u0000text"), active.copy(streamText = null, streamTruncated = true))) {
                val malformed = projection.copy(projects = listOf(project.copy(promptStatuses =
                    project.promptStatuses.map { if (it.requestId == target) bad else it })))
                assertNull(PromptStatusCodec.active(malformed, target))
                assertTrue(runCatching { WorkspaceCommand.steer(malformed, target, "ANDROID_OWNER_STEERING") }.isFailure)
                assertTrue(runCatching { WorkspaceCommand.cancelPrompt(malformed, target) }.isFailure)
                assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = malformed), true, true, command))
                assertTrue(WorkspaceDispatchPolicy.allows(state.copy(projection = malformed, pendingWorkspace = command), true, true, command))
            }
            if (phase == "steer") {
                assertTrue(runCatching { WorkspaceCommand.steer(ambiguous, target, "ANDROID_OWNER_STEERING") }.isFailure)
                assertTrue(runCatching { WorkspaceCommand.steer(projection, target, "/skill:research") }.isFailure)
            }
            assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
            file("command").outputStream().use { FrameCodec.write(it, command.envelope) }
            return@runBlocking
        }
        assertEquals("validate", phase)
        val results = JsonParser.parseString(file("results").readText()).asJsonObject
        for (raw in results.getAsJsonArray("exchanges")) {
            val exchange = raw.asJsonObject
            val request = exchange.getAsJsonObject("request")
            val command = WorkspaceCommand.restore(request)
            val wire = java.util.Base64.getDecoder().decode(exchange["wire_base64"].asString)
            val response = runCatching {
                ResponseReader.read(object : java.io.ByteArrayInputStream(wire) {
                    override fun read(bytes: ByteArray, offset: Int, length: Int) = super.read(bytes, offset, minOf(length, 3))
                }, request, {
                    if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
                    if (FrameCodec.string(it, "execution_state") == "unknown_outcome") {
                        assertTrue(it.getAsJsonObject("payload")["result"].isJsonNull)
                        assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, it) }.isFailure)
                    } else WorkspaceReceiptCodec.validate(command, it)
                })
            }
            val code = exchange["code"].takeUnless { it.isJsonNull }?.asString
            if (exchange["execution_state"].asString == "unknown_outcome") {
                assertTrue(response.exceptionOrNull().toString(), response.exceptionOrNull() is WireException)
                assertEquals("Host response result missing", response.exceptionOrNull()?.message)
            } else if (code == null) assertTrue(response.exceptionOrNull().toString(), response.isSuccess)
            else {
                assertTrue(response.exceptionOrNull().toString(), response.exceptionOrNull() is HostRefusedException)
                assertEquals(code, (response.exceptionOrNull() as HostRefusedException).code)
            }
        }
        val finalProject = HostProjectionCodec.projects(results.getAsJsonObject("listing")).single()
        val status = finalProject.promptStatuses.single { it.requestId == seed["target"].asString }
        assertEquals("cancelled", status.state); assertNull(status.streamText)
        val finalProjection = projection.copy(projects = listOf(finalProject))
        assertFalse(PromptControlPolicy.active(state.copy(projection = finalProjection), status.requestId))
        assertTrue(runCatching { WorkspaceCommand.cancelPrompt(finalProjection, status.requestId) }.isFailure)
        val pending = WorkspaceCommand.restore(results.getAsJsonObject("cancel"))
        assertTrue(WorkspaceDispatchPolicy.allows(state.copy(projection = finalProjection, phase = PairingPhase.OFFLINE,
            pendingWorkspace = pending), true, true, WorkspaceCommand.restore(pending.envelope)))
        assertEquals(1, results["steering_inputs"].asInt)
        assertEquals("ANDROID_OWNER_STEERING", results["steering_text"].asString)
        assertEquals("applied", results["steering_status"].asString)
        assertEquals(1, results["provider_steering_occurrences"].asInt)
        assertEquals(2, results["backend_calls"].asInt)
        assertTrue(results["fresh_agent_recovery"].asBoolean)
        assertEquals(0, results["recovery_backend_calls"].asInt)
        assertEquals(1, results["tool_turns"].asInt)
        assertEquals("ANDROID_TOOL_EFFECT_ONCE\n", results["tool_effect"].asString)
        val tool = results.getAsJsonObject("tool_result")
        assertEquals("android-effect.txt", tool["path"].asString)
        assertEquals("ANDROID_TOOL_EFFECT_ONCE\n".toByteArray().size, tool["bytes"].asInt)
        assertTrue(tool["diff"].asString.contains("+ANDROID_TOOL_EFFECT_ONCE"))
        val recovered = results.getAsJsonArray("recovered_inputs")
        assertEquals(1, recovered.size())
        assertEquals("ANDROID_OWNER_STEERING", recovered.single().asJsonObject["text"].asString)
        assertEquals("applied", recovered.single().asJsonObject["status"].asString)
        val revoked = results.getAsJsonArray("exchanges").toList().takeLast(3)
        assertEquals(listOf("prompt", "steer", "cancel"), revoked.map {
            it.asJsonObject.getAsJsonObject("request").getAsJsonObject("payload")["operation"].asString })
        assertTrue(revoked.all { it.asJsonObject["code"].asString == "unauthorized" })
        assertEquals("always", results["approval_mode"].asString)
        assertTrue(results["denied_file_absent"].asBoolean)
        assertTrue(results.getAsJsonObject("denied_result").has("error"))
        val approvalReceipts = results.getAsJsonArray("approval_receipts")
        assertEquals(2, approvalReceipts.size())
        for (raw in approvalReceipts) {
            val values = raw.asJsonArray.map { it.asString }
            assertEquals(3, values.size); assertEquals(1, values.distinct().size)
        }
        val audit = results.getAsJsonArray("approval_audit")
        assertEquals(listOf("allow", "deny"), audit.map { it.asJsonObject["decision"].asString })
        assertTrue(audit.all { !it.asJsonObject["remember"].asBoolean && it.asJsonObject["source"].asString == "host_answer" })
        assertEquals(1, results["approval_consumed"].asInt)
        assertTrue(results["cancelled_file_absent"].asBoolean)
        assertTrue(finalProject.pendingApprovals.isEmpty())
        val late = results.getAsJsonObject("late_approval")
        val lateResponses = results.getAsJsonArray("exchanges").filter { it.asJsonObject.getAsJsonObject("request") == late }
        assertEquals(2, lateResponses.size)
        assertEquals(listOf("invalid_request", "owner_mismatch"), lateResponses.map { it.asJsonObject["code"].asString })
        val currentLate = results.getAsJsonObject("current_owner_late_approval")
        val currentResponses = results.getAsJsonArray("exchanges").filter { it.asJsonObject.getAsJsonObject("request") == currentLate }
        assertEquals(1, currentResponses.size)
        assertEquals("invalid_request", currentResponses.single().asJsonObject["code"].asString)
        for (name in listOf("prompt_receipts", "cancel_receipts", "steer_receipts")) {
            val receipts = results.getAsJsonArray(name).map { it.asString }
            assertEquals(3, receipts.size); assertEquals(1, receipts.distinct().size)
        }
    }
}
