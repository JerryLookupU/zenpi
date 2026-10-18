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
import zenpi.transport.*
import java.io.ByteArrayInputStream
import java.io.File
import java.util.Base64

/** Current Android primary-tab commands against the desktop's actual owner/view. */
class AndroidRustWorkspaceTest {
    private fun projection(seed: JsonObject): HostProjection {
        val listing = seed.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing)
        val selected = requireNotNull(HostProjectionCodec.select(listing, projects, null))
        return HostProjectionCodec.workspace(
            HostProjectionCodec.snapshot(projects, selected, seed.getAsJsonObject("snapshot")), listing)
    }

    @Test fun primaryTabsReceiptsAndDesktopRacesRoundtrip() = runBlocking {
        val phase = System.getProperty("zenpi.workspace.phase")
        assumeNotNull(phase)
        fun file(name: String) = File(requireNotNull(System.getProperty("zenpi.workspace.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val before = projection(seed)
        val step = seed["step"].asString
        val target = seed["target"].asString
        if (phase == "generate") {
            val operation = when (step) {
                "open", "reopen" -> "workspace_open"
                "close", "close_again" -> "workspace_close"
                else -> "workspace_select"
            }
            val command = WorkspaceCommand.create(before, target, operation,
                if (operation == "workspace_open") "child" else null)
            val connected = PairingState(phase = PairingPhase.CONNECTED, projection = before)
            assertTrue(WorkspaceDispatchPolicy.allows(connected, true, true, command))
            val targetProject = before.projects.single { it.id == target }
            val ambiguous = before.copy(projects = before.projects + targetProject.copy(sessionId = "foreign", workspaceControl = false))
            assertTrue(runCatching { WorkspaceCommand.create(ambiguous, target, operation,
                if (operation == "workspace_open") "child" else null) }.isFailure)
            assertFalse(WorkspaceDispatchPolicy.allows(connected.copy(projection = ambiguous), true, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(connected, false, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(connected, true, false, command))
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            file("command").outputStream().use { FrameCodec.write(it, command.envelope) }
        } else {
            assertEquals("validate", phase)
            val result = JsonParser.parseString(file("results").readText()).asJsonObject
            val command = WorkspaceCommand.restore(result.getAsJsonObject("request"))
            for (entry in result.getAsJsonArray("exchanges")) {
                val exchange = entry.asJsonObject
                val sent = exchange.getAsJsonObject("request")
                val sentCommand = WorkspaceCommand.restore(sent)
                val code = exchange["code"]?.takeUnless { it.isJsonNull }?.asString
                val bytes = Base64.getDecoder().decode(exchange["wire_base64"].asString)
                val rawResponse = requireNotNull(FrameCodec.read(ByteArrayInputStream(bytes)))
                val unknown = FrameCodec.string(rawResponse, "execution_state") == "unknown_outcome"
                val input = object : ByteArrayInputStream(bytes) {
                    override fun read(target: ByteArray, offset: Int, length: Int): Int =
                        super.read(target, offset, minOf(length, 3))
                }
                val checked = runCatching {
                    ResponseReader.read(input, sent, {
                        WorkspaceOutcomeCodec.requireKnown(sent, it)
                        if (FrameCodec.string(it, "execution_state") == "failed")
                            throw HostRefusedException(FrameCodec.string(it, "code"))
                        WorkspaceReceiptCodec.validate(sentCommand, it)
                    })
                }
                if (unknown) {
                    assertTrue(checked.exceptionOrNull() is WorkspaceUnknownOutcomeException)
                    assertEquals(code, (checked.exceptionOrNull() as WorkspaceUnknownOutcomeException).code)
                    assertEquals(code in setOf("unauthorized", "revoked_device", "owner_mismatch"),
                        zenpi.pairing.AuthorityLossPolicy.refused(checked.exceptionOrNull() as Exception))
                    val pending = zenpi.pairing.AuthorityLossPolicy.displayOnly(
                        PairingState(phase = PairingPhase.ERROR, projection = before, pendingWorkspace = sentCommand))
                    assertEquals(sent, pending.pendingWorkspace?.envelope)
                    assertNull(pending.projection?.workspaceOwner)
                    assertTrue(WorkspaceDispatchPolicy.allows(pending, true, true, sentCommand))
                } else if (code != null) {
                    assertTrue(checked.exceptionOrNull().toString(), checked.exceptionOrNull() is HostRefusedException)
                    assertEquals(code, (checked.exceptionOrNull() as HostRefusedException).code)
                } else {
                    assertTrue(checked.exceptionOrNull().toString(), checked.isSuccess)
                    val response = requireNotNull(FrameCodec.read(ByteArrayInputStream(bytes)))
                    for (field in listOf("required_owner", "workspace_revision", "active_project_id")) {
                        val altered = response.deepCopy()
                        val receipt = altered.getAsJsonObject("payload").getAsJsonObject("result")
                        when (field) {
                            "required_owner" -> receipt.addProperty(field, "f".repeat(64))
                            "workspace_revision" -> receipt.addProperty(field, 0)
                            else -> receipt.addProperty(field, "not-an-identity")
                        }
                        assertTrue("Altered $field must not confirm", runCatching {
                            WorkspaceReceiptCodec.validate(sentCommand, altered)
                        }.isFailure)
                    }
                }
            }
            val after = projection(result.getAsJsonObject("after"))
            assertEquals(seed["expected_active"].asString, after.selectedId)
            assertEquals(seed["expected_count"].asInt, after.projects.size)
            assertEquals(java.math.BigInteger(requireNotNull(before.workspaceRevision)) + java.math.BigInteger.ONE,
                java.math.BigInteger(requireNotNull(after.workspaceRevision)))
            assertFalse(WorkspaceDispatchPolicy.allows(
                PairingState(phase = PairingPhase.CONNECTED, projection = after), true, true, command))
            // Unknown writes retain exact identity even after a local winner changes the view.
            val pending = PairingState(phase = PairingPhase.OFFLINE, projection = after, pendingWorkspace = command)
            assertTrue(WorkspaceDispatchPolicy.allows(pending, true, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(pending.copy(scanInProgress = true), true, true, command))
        }
    }
}
