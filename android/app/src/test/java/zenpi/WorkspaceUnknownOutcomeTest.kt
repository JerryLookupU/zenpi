package zenpi

import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class WorkspaceUnknownOutcomeTest {
    @Test fun codedUnknownWritePreservesPendingIdentityAndFencesOnlyAuthorityCodes() = kotlinx.coroutines.runBlocking {
        val projection = HostProjection(listOf(RemoteProject("a".repeat(64), "Workspace", "session", promptControl = true)),
            "a".repeat(64), "session", "idle", emptyList(), "1", false, "0", workspaceOwner = "b".repeat(64), workspaceRevision = "1")
        val command = WorkspaceCommand.prompt(projection, "hello")
        for (code in listOf(null, "busy", "unauthorized", "revoked_device", "owner_mismatch")) {
            val request = command.envelope
            val response = JsonObject().apply {
                addProperty("schema_version", 1); addProperty("channel", "control"); add("id", request["id"])
                addProperty("execution_state", "unknown_outcome")
                code?.let { addProperty("code", it) }
                add("payload", JsonObject().apply { addProperty("type", "response"); add("result", com.google.gson.JsonNull.INSTANCE) })
            }
            var confirmed = false
            val bytes = java.io.ByteArrayOutputStream().also { FrameCodec.write(it, response) }.toByteArray()
            val error = runCatching {
                ResponseReader.read(java.io.ByteArrayInputStream(bytes), request, {
                    WorkspaceOutcomeCodec.requireKnown(request, it); confirmed = true
                })
            }.exceptionOrNull()
            assertTrue(error is WorkspaceUnknownOutcomeException); assertFalse(confirmed)
            assertEquals(code, (error as WorkspaceUnknownOutcomeException).code)
            assertEquals(code in setOf("unauthorized", "revoked_device", "owner_mismatch"), zenpi.pairing.AuthorityLossPolicy.refused(error))
            val state = zenpi.pairing.PairingState(phase = zenpi.pairing.PairingPhase.ERROR, projection = projection, pendingWorkspace = command)
            val fenced = zenpi.pairing.AuthorityLossPolicy.displayOnly(state)
            assertEquals(command.envelope, fenced.pendingWorkspace?.envelope)
            assertNull(fenced.projection?.workspaceOwner)
            assertTrue(zenpi.pairing.WorkspaceDispatchPolicy.allows(fenced, true, true, command))
        }
    }
    @Test fun unknownCodeCannotAuthorizeReadOrClaimAResultObject() = kotlinx.coroutines.runBlocking {
        for (write in listOf(true, false)) for (nullResult in listOf(true, false)) {
            if (write && nullResult) continue
            val request = JsonObject().apply {
                addProperty("id", "request")
                add("payload", JsonObject().apply { addProperty("type", "request"); addProperty("operation", if (write) "prompt" else "inspect") })
            }
            val response = JsonObject().apply {
                addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", "request")
                addProperty("execution_state", "unknown_outcome"); addProperty("code", "unauthorized")
                add("payload", JsonObject().apply { addProperty("type", "response"); add("result", if (nullResult) com.google.gson.JsonNull.INSTANCE else JsonObject()) })
            }
            var admitted = false
            val bytes = java.io.ByteArrayOutputStream().also { FrameCodec.write(it, response) }.toByteArray()
            assertTrue(runCatching { ResponseReader.read(java.io.ByteArrayInputStream(bytes), request, { admitted = true }) }.isFailure)
            assertFalse(admitted)
        }
    }
    @Test fun unknownOutcomeIsNotAcceptedAsAConfirmedMutationReceipt() {
        val projection = HostProjection(
            listOf(RemoteProject("a".repeat(64), "Workspace", "session", promptControl = true)),
            "a".repeat(64), "session", "idle", emptyList(), "1", false,
            workspaceOwner = "b".repeat(64), workspaceRevision = "1")
        val command = WorkspaceCommand.prompt(projection, "hello")
        val response = JsonObject().apply {
            addProperty("id", command.envelope["id"].asString)
            addProperty("execution_state", "unknown_outcome")
            add("payload", JsonObject().apply { addProperty("type", "response"); add("result", JsonObject()) })
        }
        assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, response) }.isFailure)
        assertEquals("Workspace request outcome unknown", WorkspaceUnknownOutcomeException().message)
    }
}
