package zenpi

import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Assume.assumeNotNull
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*
import java.io.ByteArrayInputStream
import java.io.File
import java.util.Base64

/** Current Kotlin cancellation through a real host runner, not device/LAN evidence. */
class AndroidRustWorkerCancelTest {
    @Test fun explicitCancelStopsOnlyItsWorkerAndRestoresExactReceipt() = runBlocking {
        val phase = System.getProperty("zenpi.workercancel.phase"); assumeNotNull(phase)
        fun file(name: String) = File(requireNotNull(System.getProperty("zenpi.workercancel.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val listing = seed.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing)
        val selected = projects.single()
        val projection = HostProjectionCodec.workspace(HostProjectionCodec.snapshot(projects, selected,
            seed.getAsJsonObject("snapshot")), listing)
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection)
        val target = seed["target"].asString
        if (phase == "submit") {
            val tabs = SubtabProjectionCodec.decode(seed.getAsJsonObject("subtabs"), selected, listing)
            val active = tabs.copy(activeId = seed["submit_tab"].asString)
            val submitting = projection.copy(subtabs = active)
            val submitState = state.copy(projection = submitting)
            assertTrue(PromptControlPolicy.workerSend(submitState))
            val generated = WorkspaceCommand.workerPrompt(submitting, "wait").envelope
            generated.addProperty("id", target)
            val command = WorkspaceCommand.restore(generated)
            assertTrue(WorkspaceDispatchPolicy.allows(submitState, true, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(submitState.copy(scanInProgress = true), true, true, command))
            file("command").outputStream().use { FrameCodec.write(it, command.envelope) }
            return@runBlocking
        }
        assertNull(projection.subtabs)
        assertFalse(PromptControlPolicy.workerSend(state))
        if (phase == "generate") {
            val task = requireNotNull(WorkerTargetPolicy.cancellable(state, target))
            val command = WorkspaceCommand.cancelWorker(projection, task)
            assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
            for (bad in listOf(task.copy(subtabId = "bad"), task.copy(failureCode = "internal_error"),
                task.copy(outputWithheld = true), task.copy(outputText = "invalid running output"),
                task.copy(streamText = "x".repeat(4097)))) {
                val malformed = projection.copy(projects = listOf(selected.copy(workerStatuses =
                    selected.workerStatuses.map { if (it.requestId == target) bad else it })))
                assertNull(WorkerTargetPolicy.cancellable(state.copy(projection = malformed), target))
                assertTrue(runCatching { WorkspaceCommand.cancelWorker(malformed, bad) }.isFailure)
                assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = malformed), true, true, command))
                assertTrue(WorkspaceDispatchPolicy.allows(state.copy(projection = malformed, pendingWorkspace = command), true, true, command))
            }
            assertEquals(setOf("request_id"), command.envelope.getAsJsonObject("payload").getAsJsonObject("arguments").keySet())
            val ambiguous = projection.copy(projects = listOf(selected.copy(workerStatuses = selected.workerStatuses + task.copy(state = "succeeded"))))
            assertTrue(runCatching { WorkspaceCommand.cancelWorker(ambiguous, target) }.isFailure)
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = ambiguous), true, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(scanInProgress = true), true, true, command))
            file("command").outputStream().use { FrameCodec.write(it, command.envelope) }
            return@runBlocking
        }
        assertEquals("validate", phase)
        val results = JsonParser.parseString(file("results").readText()).asJsonObject
        val command = WorkspaceCommand.restore(results.getAsJsonObject("request"))
        for (raw in results.getAsJsonArray("exchanges")) {
            val exchange = raw.asJsonObject
            val request = exchange.getAsJsonObject("request")
            val sent = WorkspaceCommand.restore(request)
            val bytes = Base64.getDecoder().decode(exchange["wire_base64"].asString)
            val response = runCatching { ResponseReader.read(object : ByteArrayInputStream(bytes) {
                override fun read(buffer: ByteArray, offset: Int, length: Int) = super.read(buffer, offset, minOf(length, 3))
            }, request, {
                if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
                WorkspaceReceiptCodec.validate(sent, it)
            }) }
            val code = exchange["code"].takeUnless { it.isJsonNull }?.asString
            if (code == null) assertTrue(response.exceptionOrNull().toString(), response.isSuccess)
            else {
                assertTrue(response.exceptionOrNull().toString(), response.exceptionOrNull() is HostRefusedException)
                assertEquals(code, (response.exceptionOrNull() as HostRefusedException).code)
            }
        }
        val after = HostProjectionCodec.projects(results.getAsJsonObject("after")).single()
        assertEquals("cancelled", after.workerStatuses.single { it.requestId == target }.state)
        assertEquals("running", after.workerStatuses.single { it.requestId == "sibling-task" }.state)
        assertEquals(1, results["cancel_observations"].asInt)
        assertEquals(2, results["starts"].asInt)
        assertTrue(results["actual_agent_execution"].asBoolean)
        assertTrue(results["submission_restart_equal"].asBoolean)
        val submissions = results.getAsJsonArray("kotlin_submissions")
        assertEquals(2, submissions.size())
        for (raw in submissions) {
            val sent = raw.asJsonObject
            val submitted = WorkspaceCommand.restore(sent.getAsJsonObject("request"))
            val response = requireNotNull(FrameCodec.read(ByteArrayInputStream(Base64.getDecoder().decode(sent["wire_base64"].asString))))
            WorkspaceReceiptCodec.validate(submitted, response)
        }
        val security = results.getAsJsonArray("security_exchanges")
        assertEquals(6, security.size())
        assertEquals(listOf("request_id_conflict", "request_id_conflict", "request_id_conflict", "unauthorized", "unauthorized", "unauthorized"),
            security.map { it.asJsonObject["code"].asString })
        for (raw in security) {
            val exchange = raw.asJsonObject
            val request = exchange.getAsJsonObject("request")
            val sent = WorkspaceCommand.restore(request)
            val input = ByteArrayInputStream(Base64.getDecoder().decode(exchange["wire_base64"].asString))
            val response = runCatching { ResponseReader.read(input, request, {
                if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
                WorkspaceReceiptCodec.validate(sent, it)
            }) }
            assertTrue(response.exceptionOrNull() is HostRefusedException)
            assertEquals(exchange["code"].asString, (response.exceptionOrNull() as HostRefusedException).code)
        }
        assertTrue(results["post_security_journal_unchanged"].asBoolean)
        assertTrue(results["post_security_workers_absent"].asBoolean)
        assertEquals(1, results["target_user_turns"].asInt)
        assertEquals(0, results["target_assistant_turns"].asInt)
        assertEquals(1, results["sibling_assistant_turns"].asInt)
        val finished = WorkerStatusCodec.decode(results["finished"], after.id, after.sessionId)
        assertEquals("succeeded", finished.single { it.requestId == "sibling-task" }.state)
        assertEquals("cancelled", finished.single { it.requestId == target }.state)
        val receipts = results.getAsJsonArray("receipts").map { it.asString }
        assertEquals(3, receipts.size); assertEquals(1, receipts.distinct().size)
        assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = projection.copy(projects = listOf(after))), true, true, command))
        assertTrue(WorkspaceDispatchPolicy.allows(state.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = command), true, true, command))
    }
}
