package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.pairing.*

class ResourceReloadTest {
    @Test fun malformedCatalogAndHistoryCannotAuthorizeFreshReloadButExactIntentStillRetries() {
        val command = WorkspaceCommand.reloadResources(projection, catalog)
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, resources = catalog)
        for (bad in listOf(catalog.copy(generation = "04"), catalog.copy(generation = "18446744073709551616"),
            catalog.copy(snapshotId = "invalid"), catalog.copy(offset = 1), catalog.copy(offset = 32),
            catalog.copy(truncated = true), catalog.copy(truncated = true, nextOffset = 32))) {
            assertFalse(ResourceCatalogCodec.sameScope(bad, projection))
            assertTrue(runCatching { WorkspaceCommand.reloadResources(projection, bad) }.isFailure)
            assertTrue(runCatching { ResourceCatalogCodec.request(projection, bad, 32) }.isFailure)
            val blocked = state.copy(resources = bad)
            assertFalse(ResourceReadPolicy.navigation(blocked)); assertFalse(ResourceReadPolicy.insertion(blocked))
            assertTrue(ResourceReadPolicy.browse(blocked))
            assertFalse(WorkspaceDispatchPolicy.allows(blocked, true, true, command))
            assertTrue(WorkspaceDispatchPolicy.allows(blocked.copy(pendingWorkspace = command), true, true, command))
        }
        val first = catalog.copy(entries = (0 until 32).map { RemoteResource("entry-$it", "template", "/entry-$it", "", true) },
            truncated = true, nextOffset = 32)
        val tail = catalog.copy(offset = 32, entries = listOf(RemoteResource("tail", "template", "/tail", "", true)))
        val reload = WorkspaceCommand.reloadResources(projection, tail)
        val healthy = state.copy(resources = tail, resourcesPrevious = listOf(first))
        assertTrue(ResourceReadPolicy.navigation(healthy)); assertTrue(WorkspaceDispatchPolicy.allows(healthy, true, true, reload))
        for (history in listOf(listOf(first.copy(generation = "5")), listOf(first.copy(snapshotId = "c".repeat(64))),
            listOf(first.copy(entries = first.entries.dropLast(1) + tail.entries.single())), List(8) { first })) {
            val blocked = healthy.copy(resourcesPrevious = history)
            assertFalse(ResourceReadPolicy.navigation(blocked)); assertFalse(ResourceReadPolicy.insertion(blocked))
            assertFalse(WorkspaceDispatchPolicy.allows(blocked, true, true, reload))
        }
    }

    @Test fun reloadConstructionRejectsDuplicateProjectBeforeFilteringScopeAndCapability() {
        val command = WorkspaceCommand.reloadResources(projection, catalog)
        for (foreign in listOf(project.copy(sessionId = "foreign", resourcesReloadControl = false),
            project.copy(resourcesReloadControl = false), project.copy(sessionId = "foreign"))) {
            val ambiguous = projection.copy(projects = listOf(project, foreign))
            assertTrue(runCatching { WorkspaceCommand.reloadResources(ambiguous, catalog) }.isFailure)
            val state = PairingState(phase = PairingPhase.CONNECTED, projection = ambiguous, resources = catalog)
            assertFalse(WorkspaceDispatchPolicy.allows(state, true, true, command))
            assertTrue(WorkspaceDispatchPolicy.allows(state.copy(pendingWorkspace = command), true, true, command))
        }
        assertTrue(runCatching { WorkspaceCommand.reloadResources(projection.copy(sessionId = "foreign"), catalog) }.isFailure)
        val paged = catalog.copy(offset = 32, entries = listOf(RemoteResource("review", "template", "/review", "", true)))
        assertEquals("4", WorkspaceCommand.reloadResources(projection, paged).envelope.getAsJsonObject("payload")
            .getAsJsonObject("arguments")["generation"].toString())
    }
    private val project = RemoteProject("project", "Workspace", "session", promptControl = true, resourcesReloadControl = true)
    private val projection = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false, "0",
        workspaceOwner = "a".repeat(64), workspaceRevision = "3")
    private val catalog = ResourceCatalog("project", "session", "4", emptyList(), false, "b".repeat(64))
    private fun response(command: WorkspaceCommand) = JsonObject().apply {
        addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", command.envelope["id"].asString)
        addProperty("project_id", "project"); addProperty("session_id", "session"); addProperty("execution_state", "succeeded")
        add("payload", JsonObject().apply {
            addProperty("type", "response")
            add("result", JsonObject().apply {
                addProperty("request_id", command.envelope["id"].asString); addProperty("project_id", "project")
                addProperty("session_id", "session"); addProperty("generation", 5)
            })
        })
    }
    @Test fun reloadBindsHostScopeGenerationAndExactSavedRequest() {
        val command = WorkspaceCommand.reloadResources(projection, catalog)
        val envelope = command.envelope
        assertEquals("resources_reload", envelope["payload"].asJsonObject["operation"].asString)
        assertEquals("4", envelope["payload"].asJsonObject["arguments"].asJsonObject["generation"].toString())
        assertEquals(catalog.snapshotId, envelope["payload"].asJsonObject["arguments"].asJsonObject["snapshot_id"].asString)
        assertEquals("a".repeat(64), envelope["required_owner"].asString)
        assertEquals("3", envelope["expected_revision"].toString())
        assertEquals(envelope, WorkspaceCommand.restore(envelope).envelope)
        WorkspaceReceiptCodec.validate(command, response(command))
        for (args in listOf("""{}""", """{"generation":"4"}""", """{"generation":-1}""",
            """{"generation":18446744073709551616}""", """{"generation":4,"cwd":"/private"}""")) {
            val invalid = envelope.deepCopy().apply { getAsJsonObject("payload").add("arguments", JsonParser.parseString(args)) }
            assertTrue(runCatching { WorkspaceCommand.restore(invalid) }.isFailure)
        }
        assertTrue(runCatching { WorkspaceCommand.reloadResources(projection.copy(
            projects = listOf(project.copy(resourcesReloadControl = false))), catalog) }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.reloadResources(projection, catalog.copy(sessionId = "foreign")) }.isFailure)
        val badDigest = envelope.deepCopy().apply { getAsJsonObject("payload").getAsJsonObject("arguments")
            .addProperty("snapshot_id", "foreign") }
        assertTrue(runCatching { WorkspaceCommand.restore(badDigest) }.isFailure)
    }
    @Test fun receiptRequiresIncreasedUnsignedGenerationAndExactIdentity() {
        val command = WorkspaceCommand.reloadResources(projection, catalog)
        for ((field, value) in listOf("generation" to 4, "generation" to -1, "generation" to "5",
            "project_id" to "foreign", "session_id" to "foreign", "request_id" to "foreign", "extra" to true)) {
            val invalid = response(command)
            val result = invalid["payload"].asJsonObject["result"].asJsonObject
            when (value) {
                is String -> result.addProperty(field, value)
                is Number -> result.addProperty(field, value)
                is Boolean -> result.addProperty(field, value)
            }
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, invalid) }.isFailure)
        }
        for (state in listOf("accepted", "unknown_outcome", "running")) {
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(command,
                response(command).apply { addProperty("execution_state", state) }) }.isFailure)
        }
    }
    @Test fun freshReloadIsForegroundConnectedOnlyAndCacheDropsCapability() {
        val command = WorkspaceCommand.reloadResources(projection, catalog)
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, resources = catalog)
        assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state, false, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state, true, false, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state.copy(phase = PairingPhase.ERROR), true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state.copy(phase = PairingPhase.OFFLINE), true, true, command))
        val cached = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(projection))
        assertFalse(cached.projects.single().resourcesReloadControl)
        assertTrue(runCatching { WorkspaceCommand.reloadResources(cached, catalog) }.isFailure)
        val pending = state.copy(phase = PairingPhase.ERROR, pendingWorkspace = command)
        assertTrue(WorkspaceDispatchPolicy.allows(pending, true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.reloadResources(projection, catalog)))
    }
    @Test fun newReloadCannotPersistAfterCatalogScopeGenerationSnapshotOrReadStateChanged() {
        val command = WorkspaceCommand.reloadResources(projection, catalog)
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, resources = catalog)
        for (changed in listOf(state.copy(resources = null), state.copy(resourcesLoading = true), state.copy(resourcesError = "failed"),
            state.copy(resources = catalog.copy(projectId = "foreign")), state.copy(resources = catalog.copy(sessionId = "other")),
            state.copy(resources = catalog.copy(generation = "5")), state.copy(resources = catalog.copy(snapshotId = "c".repeat(64))))) {
            assertFalse(WorkspaceDispatchPolicy.allows(changed, true, true, command))
            assertTrue(WorkspaceDispatchPolicy.allows(changed.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = command),
                true, true, WorkspaceCommand.restore(command.envelope)))
        }
        val updated = catalog.copy(generation = "5", snapshotId = "c".repeat(64))
        assertTrue(WorkspaceDispatchPolicy.allows(state.copy(resources = updated), true, true,
            WorkspaceCommand.reloadResources(projection, updated)))
    }
}
