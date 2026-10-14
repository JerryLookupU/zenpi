package zenpi

import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.state.ProjectionCacheCodec
import zenpi.pairing.*

class ConcurrencyControlTest {
    private val project = "a".repeat(64)
    private val tab = "b".repeat(64)
    private val live = HostProjection(listOf(RemoteProject(project, "Workspace", "session", workspaceControl = true)),
        project, "session", "idle", emptyList(), "1", false, "0", workspaceOwner = "c".repeat(64), workspaceRevision = "4",
        subtabs = RemoteSubtabs(project, "2", tab, listOf(RemoteSubtab(tab, "main", "main", 1)), true, concurrencyControl = true))
    @Test fun exactQuotaCommandIsBoundedSelectedAndExplicitlyRestorable() {
        for (count in listOf(1, 2, 64)) {
            val command = WorkspaceCommand.concurrency(live, tab, count)
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            val args = command.envelope["payload"].asJsonObject["arguments"].asJsonObject
            assertEquals(setOf("subtab_id", "subtab_revision", "concurrency"), args.keySet())
            assertEquals(tab, args["subtab_id"].asString); assertEquals("2", args["subtab_revision"].toString())
            assertEquals(count, args["concurrency"].asInt)
        }
        for (count in listOf(0, 65, -1)) assertTrue(runCatching { WorkspaceCommand.concurrency(live, tab, count) }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.concurrency(live, "foreign", 2) }.isFailure)
        val cached = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(live))
        assertFalse(cached.subtabs!!.concurrencyControl)
        assertTrue(runCatching { WorkspaceCommand.concurrency(cached, tab, 2) }.isFailure)
        for (value in listOf("0", "65", "-1", "1.0", "\"2\"", "true")) {
            val request = WorkspaceCommand.concurrency(live, tab, 2).envelope
            request["payload"].asJsonObject["arguments"].asJsonObject.add("concurrency", com.google.gson.JsonParser.parseString(value))
            assertTrue(runCatching { WorkspaceCommand.restore(request) }.isFailure)
        }
    }
    @Test fun concurrencyReceiptRequiresCommittedSubtabAndWorkspaceRevision() {
        val command = WorkspaceCommand.concurrency(live, tab, 2)
        val result = JsonObject().apply {
            addProperty("required_owner", "c".repeat(64)); addProperty("workspace_revision", 5); addProperty("active_project_id", project)
            addProperty("project_id", project); addProperty("active_subtab_id", tab); addProperty("subtab_revision", 3)
        }
        fun response(value: JsonObject) = JsonObject().apply {
            addProperty("id", command.envelope["id"].asString); addProperty("channel", "control"); addProperty("execution_state", "succeeded")
            add("payload", JsonObject().apply { addProperty("type", "response"); add("result", value) })
        }
        assertEquals(result, WorkspaceReceiptCodec.validate(command, response(result)))
        for (field in listOf("workspace_revision", "subtab_revision", "project_id", "required_owner")) {
            val invalid = result.deepCopy(); invalid.addProperty(field, "foreign")
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, response(invalid)) }.isFailure)
        }
    }
    @Test fun quotaWritesRequireForegroundLiveAuthorityOrExactExplicitRetry() {
        val command = WorkspaceCommand.concurrency(live, tab, 2)
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = live)
        fun allowed(value: PairingState, foreground: Boolean = true, network: Boolean = true) =
            WorkspaceDispatchPolicy.allows(value, foreground, network, command)
        assertTrue(allowed(state))
        for (phase in PairingPhase.values()) {
            assertEquals(phase == PairingPhase.CONNECTED, allowed(state.copy(phase = phase)))
            assertFalse(allowed(state.copy(phase = phase), foreground = false))
            assertFalse(allowed(state.copy(phase = phase), network = false))
        }
        val pending = state.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = WorkspaceCommand.restore(command.envelope))
        assertTrue(allowed(pending))
        assertFalse(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.concurrency(live, tab, 2)))
        for (value in listOf(pending.copy(resetting = true), pending.copy(resetRequired = true), pending.copy(pendingRevoke = "revoke"))) {
            assertFalse(allowed(value))
        }
    }
    @Test fun connectedQuotaRequestRejectsStaleTabAndRevokedControlBeforeDurableAdmission() {
        val command = WorkspaceCommand.concurrency(live, tab, 2)
        val tabs = requireNotNull(live.subtabs)
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = live)
        for (changed in listOf(live.copy(subtabs = null), live.copy(subtabs = tabs.copy(revision = "3")),
            live.copy(subtabs = tabs.copy(concurrencyControl = false)),
            live.copy(projects = live.projects.map { it.copy(workspaceControl = false) }))) {
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = changed), true, true, command))
        }
    }
    @Test fun fragmentedQuotaResponseCannotConfirmUnknownEvenWithAValidLookingResult() = kotlinx.coroutines.runBlocking {
        val command = WorkspaceCommand.concurrency(live, tab, 2)
        val result = JsonObject().apply {
            addProperty("required_owner", "c".repeat(64)); addProperty("workspace_revision", 5); addProperty("active_project_id", project)
            addProperty("project_id", project); addProperty("active_subtab_id", tab); addProperty("subtab_revision", 3)
        }
        for (state in listOf("unknown_outcome", "accepted", "running", "succeeded")) {
            val response = JsonObject().apply {
                addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", command.envelope["id"].asString)
                addProperty("execution_state", state)
                add("payload", JsonObject().apply { addProperty("type", "response"); add("result", result) })
            }
            val bytes = java.io.ByteArrayOutputStream().also { FrameCodec.write(it, response) }.toByteArray()
            val input = object : java.io.ByteArrayInputStream(bytes) {
                override fun read(target: ByteArray, offset: Int, length: Int): Int = super.read(target, offset, minOf(length, 3))
            }
            var confirmed = false
            val received = runCatching {
                ResponseReader.read(input, command.envelope, { WorkspaceReceiptCodec.validate(command, it) })
                confirmed = true
            }
            assertEquals(state == "succeeded", received.isSuccess)
            assertEquals(state == "succeeded", confirmed)
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        }
    }
}
