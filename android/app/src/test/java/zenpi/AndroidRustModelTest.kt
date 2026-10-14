package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Assume.assumeNotNull
import org.junit.Test
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.pairing.WorkspaceDispatchPolicy
import zenpi.pairing.ModelReadPolicy
import zenpi.transport.*
import java.io.ByteArrayInputStream
import java.io.File
import java.util.Base64

/** Current Android model commands and receipts, not a provider/network fixture. */
class AndroidRustModelTest {
    private fun projection(seed: JsonObject): HostProjection {
        val listing = seed.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing)
        val selected = requireNotNull(HostProjectionCodec.select(listing, projects, null))
        return HostProjectionCodec.workspace(
            HostProjectionCodec.snapshot(projects, selected, seed.getAsJsonObject("snapshot")), listing)
    }

    @Test fun modelAndReasoningCommandsRoundtripThroughDesktop() = runBlocking {
        val phase = System.getProperty("zenpi.model.phase")
        assumeNotNull(phase)
        fun file(name: String) = File(requireNotNull(System.getProperty("zenpi.model.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val before = projection(seed)
        val status = ModelStatusCodec.decode(seed.getAsJsonObject("model"), before)
        val oldBudget = ModelStatusCodec.decode(seed.getAsJsonObject("budget_before"), before)
        val newBudget = ModelStatusCodec.decode(seed.getAsJsonObject("budget_after"), before)
        assertEquals(oldBudget.selectionRevision, newBudget.selectionRevision)
        assertEquals(oldBudget.catalog, newBudget.catalog)
        assertNotEquals(oldBudget.maxTokens, newBudget.maxTokens)
        val merged = requireNotNull(ModelStatusCodec.newest(oldBudget, newBudget, before))
        assertEquals("32000", merged.maxTokens); assertEquals("2048", merged.reservedOutputTokens)
        assertEquals(newBudget, merged)
        for (bad in listOf(status.copy(selectionRevision = "01"), status.copy(maxTokens = "-1"),
            status.copy(reasoningChoices = listOf("unknown")))) {
            assertFalse(ModelStatusCodec.sameScope(bad, before))
            assertTrue(runCatching { WorkspaceCommand.selectModel(before, bad, "manual-model") }.isFailure)
            assertEquals(status, ModelStatusCodec.newest(status, bad, before))
        }
        val readable = PairingState(phase = PairingPhase.CONNECTED, projection = before, modelOpen = true, modelStatus = status)
        assertTrue(ModelReadPolicy.browse(readable))
        assertFalse(ModelReadPolicy.browse(readable.copy(scanInProgress = true)))
        assertTrue(ModelReadPolicy.browse(readable.copy(projection = before.copy(projects = before.projects.map {
            it.copy(modelControl = false, reasoningControl = false)
        }))))
        if (phase == "generate") {
            val selected = seed["selected"].takeUnless { it.isJsonNull }?.asString
            val command = if (seed["operation"].asString == "model_select")
                WorkspaceCommand.selectModel(before, status, requireNotNull(selected))
            else WorkspaceCommand.selectReasoning(before, status, selected)
            val connected = PairingState(phase = PairingPhase.CONNECTED, projection = before, modelStatus = status)
            for (invalid in listOf(before.copy(projects = before.projects + before.projects.single().copy(sessionId = "foreign", modelControl = false, reasoningControl = false)),
                before.copy(modelStatus = status.copy(selectionRevision = (java.math.BigInteger(status.selectionRevision) + java.math.BigInteger.ONE).toString())))) {
                assertTrue(runCatching { WorkspaceCommand.selectModel(invalid, status, "manual-model") }.isFailure)
                assertTrue(runCatching { WorkspaceCommand.selectReasoning(invalid, status, null) }.isFailure)
                assertFalse(WorkspaceDispatchPolicy.allows(connected.copy(projection = invalid), true, true, command))
                assertTrue(WorkspaceDispatchPolicy.allows(connected.copy(projection = invalid, pendingWorkspace = command), true, true, command))
            }
            assertTrue(WorkspaceDispatchPolicy.allows(connected, true, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(connected, false, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(connected, true, false, command))
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            file("command").outputStream().use { FrameCodec.write(it, command.envelope) }
        } else {
            assertEquals("validate", phase)
            val results = JsonParser.parseString(file("results").readText()).asJsonObject
            for (raw in results.getAsJsonArray("exchanges")) {
                val exchange = raw.asJsonObject
                val request = exchange.getAsJsonObject("request")
                val command = WorkspaceCommand.restore(request)
                val bytes = Base64.getDecoder().decode(exchange["wire_base64"].asString)
                val input = object : ByteArrayInputStream(bytes) {
                    override fun read(target: ByteArray, offset: Int, length: Int): Int =
                        super.read(target, offset, minOf(length, 3))
                }
                val checked = runCatching {
                    ResponseReader.read(input, request, {
                        if (FrameCodec.string(it, "execution_state") == "failed")
                            throw HostRefusedException(FrameCodec.string(it, "code"))
                        WorkspaceReceiptCodec.validate(command, it)
                    })
                }
                val code = exchange["code"].takeUnless { it.isJsonNull }?.asString
                if (code != null) {
                    assertTrue(checked.exceptionOrNull().toString(), checked.exceptionOrNull() is HostRefusedException)
                    assertEquals(code, (checked.exceptionOrNull() as HostRefusedException).code)
                } else {
                    assertTrue(checked.exceptionOrNull().toString(), checked.isSuccess)
                    val response = requireNotNull(FrameCodec.read(ByteArrayInputStream(bytes)))
                    val field = if (request.getAsJsonObject("payload")["operation"].asString == "model_select") "model" else "reasoning_effort"
                    for (changed in listOf("request_id", "project_id", "session_id", "selection_revision", field)) {
                        val altered = response.deepCopy()
                        val result = altered.getAsJsonObject("payload").getAsJsonObject("result")
                        if (changed == "selection_revision") result.addProperty(changed, 0)
                        else result.addProperty(changed, "foreign")
                        assertTrue("Altered $changed cannot confirm", runCatching {
                            WorkspaceReceiptCodec.validate(command, altered)
                        }.isFailure)
                    }
                }
            }
            val after = results.getAsJsonObject("after")
            val current = ModelStatusCodec.decode(after.getAsJsonObject("model"), projection(after))
            assertEquals(results["expected_model"].asString, current.model)
            assertEquals(results["expected_reasoning"].takeUnless { it.isJsonNull }?.asString, current.reasoningEffort)
            assertEquals(results["expected_revision"].toString(), current.selectionRevision)
            assertEquals(before.workspaceRevision, projection(after).workspaceRevision)
            assertEquals(before.sessionId, current.sessionId)
            val advanced = PairingState(phase = PairingPhase.CONNECTED, projection = projection(after), modelStatus = current)
            // Actual host selection changes leave workspace CAS unchanged.
            // New Android admission must still reject each earlier choice.
            for (raw in results.getAsJsonArray("exchanges")) {
                val command = WorkspaceCommand.restore(raw.asJsonObject.getAsJsonObject("request"))
                assertFalse(WorkspaceDispatchPolicy.allows(advanced, true, true, command))
                assertTrue(WorkspaceDispatchPolicy.allows(advanced.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = command),
                    true, true, WorkspaceCommand.restore(command.envelope)))
            }
        }
    }
}
