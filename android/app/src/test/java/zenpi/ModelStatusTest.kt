package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class ModelStatusTest {
    @Test fun sameSelectionAndCatalogRefreshMetadataWithoutResettingTheDisplayedPage() {
        val original = ModelStatusCodec.decode(value(), projection).copy(reasoningChoices = listOf(null, "low"),
            catalog = RemoteModelCatalog("a".repeat(64), 16, null, listOf(RemoteModelChoice("model-16", "active", true))))
        val incoming = original.copy(maxTokens = "32000", reservedOutputTokens = "2048",
            capabilities = original.capabilities?.mapValues { false }, reasoningChoices = listOf("low"),
            catalog = RemoteModelCatalog("a".repeat(64), 0, 16, (0..15).map {
                RemoteModelChoice("model-%02d".format(it), "active", true) }))
        val merged = requireNotNull(ModelStatusCodec.newest(original, incoming, projection))
        assertEquals(incoming.copy(catalog = original.catalog), merged)
        assertEquals(original.catalog, merged.catalog)
        assertTrue(ModelStatusCodec.retainsPage(original, merged))
        assertFalse(ModelStatusCodec.retainsPage(original, merged.copy(selectionRevision = "8")))
        assertFalse(ModelStatusCodec.retainsPage(original, incoming))
        assertFalse(ModelStatusCodec.retainsPage(original, merged.copy(sessionId = "foreign")))
        assertFalse(ModelStatusCodec.retainsPage(null, merged))
        val back = requireNotNull(ModelStatusCodec.newest(incoming.copy(maxTokens = original.maxTokens), merged, projection))
        assertEquals(incoming.catalog, back.catalog)
        assertEquals("32000", back.maxTokens); assertEquals("2048", back.reservedOutputTokens)
        assertEquals(incoming, ModelStatusCodec.newest(original, incoming.copy(selectionRevision = "8"), projection)
            ?.copy(selectionRevision = "7"))
        assertEquals(original, ModelStatusCodec.newest(original, incoming.copy(selectionRevision = "6"), projection))
        assertEquals(incoming.copy(catalog = incoming.catalog?.copy(snapshotId = "b".repeat(64))),
            ModelStatusCodec.newest(original, incoming.copy(catalog = incoming.catalog?.copy(snapshotId = "b".repeat(64))), projection))
    }
    @Test fun retainedStatusAndCatalogIntegrityFenceSelectionPagingAndMerge() {
        val status = ModelStatusCodec.decode(value(), projection).copy(reasoningChoices = listOf(null, "low"),
            catalog = RemoteModelCatalog("a".repeat(64), 0, 16, (0..15).map {
                RemoteModelChoice("model-%02d".format(it), "active", true) }))
        val live = projection.copy(projects = listOf(projection.projects.single().copy(modelControl = true, reasoningControl = true)),
            workspaceOwner = "b".repeat(64), workspaceRevision = "3", modelStatus = status)
        val commands = listOf(WorkspaceCommand.selectModel(live, status, "manual"), WorkspaceCommand.selectReasoning(live, status, null))
        val catalog = requireNotNull(status.catalog)
        val badCatalogs = listOf(catalog.copy(snapshotId = "bad"), catalog.copy(offset = 1), catalog.copy(nextOffset = 32),
            catalog.copy(entries = catalog.entries.dropLast(1)), catalog.copy(entries = catalog.entries.reversed()),
            catalog.copy(entries = catalog.entries.map { it.copy(status = "unknown") }),
            catalog.copy(entries = catalog.entries.map { it.copy(id = "https://private.invalid") }))
        val malformed = listOf(status.copy(selectionRevision = "01"), status.copy(maxTokens = "-1"),
            status.copy(reservedOutputTokens = "18446744073709551616"), status.copy(model = "/private/model"),
            status.copy(reasoningEffort = "unknown"), status.copy(reasoningChoices = listOf("low", "low")),
            status.copy(capabilities = mapOf("text" to true))) + badCatalogs.map { status.copy(catalog = it) }
        for (bad in malformed) {
            assertFalse(ModelStatusCodec.sameScope(bad, live))
            assertFalse(ModelStatusCodec.selectionCurrent(bad, live))
            assertTrue(runCatching { WorkspaceCommand.selectModel(live, bad, "manual") }.isFailure)
            assertTrue(runCatching { ModelStatusCodec.request(live, bad, 16) }.isFailure)
            assertEquals(status, ModelStatusCodec.newest(status, bad, live))
            val state = zenpi.pairing.PairingState(phase = zenpi.pairing.PairingPhase.CONNECTED,
                projection = live.copy(modelStatus = bad), modelStatus = bad, modelOpen = true)
            assertFalse(zenpi.pairing.ModelReadPolicy.navigation(state))
            for (command in commands) {
                assertFalse(zenpi.pairing.WorkspaceDispatchPolicy.allows(state, true, true, command))
                assertTrue(zenpi.pairing.WorkspaceDispatchPolicy.allows(state.copy(pendingWorkspace = command), true, true, command))
            }
        }
        val tail = status.copy(catalog = catalog.copy(offset = 16, nextOffset = null,
            entries = listOf(RemoteModelChoice("model-16", "active", true))))
        val ready = zenpi.pairing.PairingState(phase = zenpi.pairing.PairingPhase.CONNECTED,
            projection = live, modelStatus = tail, modelPrevious = listOf(status), modelOpen = true)
        assertTrue(zenpi.pairing.ModelReadPolicy.navigation(ready))
        for (history in listOf(listOf(status.copy(selectionRevision = "8")), listOf(status.copy(catalog = catalog.copy(snapshotId = "b".repeat(64)))),
            listOf(status.copy(catalog = catalog.copy(entries = catalog.entries.dropLast(1) + tail.catalog!!.entries))), List(8) { status })) {
            val state = ready.copy(modelPrevious = history)
            assertFalse(zenpi.pairing.ModelReadPolicy.navigation(state))
            for (command in commands) assertFalse(zenpi.pairing.WorkspaceDispatchPolicy.allows(state, true, true, command))
        }
        assertTrue(zenpi.pairing.ModelReadPolicy.browse(ready.copy(modelStatus = status.copy(selectionRevision = "bad"))))
    }

    @Test fun selectionConstructionUsesUniqueIdentityAndCurrentOwnerSelectionBeforeCapabilities() {
        val project = projection.projects.single().copy(modelControl = true, reasoningControl = true)
        val status = ModelStatusCodec.decode(value(), projection).copy(reasoningChoices = listOf(null, "none", "low"))
        val live = projection.copy(projects = listOf(project), workspaceOwner = "a".repeat(64), workspaceRevision = "3", modelStatus = status)
        val commands = listOf(WorkspaceCommand.selectModel(live, status, "manual-model"), WorkspaceCommand.selectReasoning(live, status, null))
        for (invalid in listOf(live.copy(projects = listOf(project, project.copy(sessionId = "foreign", modelControl = false, reasoningControl = false))),
            live.copy(sessionId = "foreign"), live.copy(modelStatus = status.copy(selectionRevision = "8")),
            live.copy(modelStatus = status.copy(registryBound = false)), live.copy(modelStatus = status.copy(projectId = "foreign")))) {
            assertTrue(runCatching { WorkspaceCommand.selectModel(invalid, status, "manual-model") }.isFailure)
            assertTrue(runCatching { WorkspaceCommand.selectReasoning(invalid, status, null) }.isFailure)
            val state = zenpi.pairing.PairingState(phase = zenpi.pairing.PairingPhase.CONNECTED, projection = invalid, modelStatus = status)
            for (command in commands) {
                assertFalse(zenpi.pairing.WorkspaceDispatchPolicy.allows(state, true, true, command))
                assertTrue(zenpi.pairing.WorkspaceDispatchPolicy.allows(state.copy(pendingWorkspace = command), true, true, command))
            }
        }
        val choicesChanged = live.copy(modelStatus = status.copy(reasoningChoices = listOf("low")))
        assertTrue(runCatching { WorkspaceCommand.selectReasoning(choicesChanged, status, null) }.isFailure)
        assertNotNull(WorkspaceCommand.selectReasoning(choicesChanged, status, "low"))
        assertNotNull(WorkspaceCommand.selectModel(live.copy(modelStatus = null), status, "manual-model"))
    }
    @Test fun catalogPagesAreStrictScopedAndSnapshotBound() {
        fun page(offset:Int,next:Int?,count:Int)=value().apply {
            add("reasoning_choices",JsonParser.parseString("[null,\"low\"]"))
            add("catalog",JsonObject().apply{addProperty("snapshot_id","a".repeat(64));addProperty("offset",offset);addProperty("next_offset",next)
                add("entries",com.google.gson.JsonArray().apply{repeat(count){index->add(JsonObject().apply{
                    addProperty("id","fixture-"+(index+offset).toString().padStart(2,'0'));addProperty("status","active");addProperty("selectable",index!=1)
                })}})})
        }
        val first=ModelStatusCodec.decode(page(0,16,16),projection)
        assertEquals(16,first.catalog?.nextOffset);assertFalse(requireNotNull(first.catalog).entries[1].selectable)
        val request=ModelStatusCodec.request(projection,first,16)
        FrameCodec.decode(request.toString().toByteArray());assertFalse(request.has("required_owner"))
        val tail=ModelStatusCodec.decode(page(16,null,3),projection,first,16);assertNull(tail.catalog?.nextOffset)
        assertEquals(first,ModelStatusCodec.decode(page(0,16,16),projection,tail,0))
        for(change in listOf<(JsonObject)->Unit>(
            {it["catalog"].asJsonObject.addProperty("snapshot_id","b".repeat(64))},
            {it.addProperty("selection_revision",8)},
            {it["catalog"].asJsonObject.addProperty("offset",0)},
            {it["catalog"].asJsonObject.addProperty("next_offset",32)},
            {it["catalog"].asJsonObject["entries"].asJsonArray[0].asJsonObject.addProperty("source","private")},
            {it["catalog"].asJsonObject["entries"].asJsonArray[0].asJsonObject.addProperty("id","https://private.invalid")},
            {it["catalog"].asJsonObject["entries"].asJsonArray[0].asJsonObject.addProperty("status","unknown")},
            {it["catalog"].asJsonObject["entries"].asJsonArray[0].asJsonObject.addProperty("selectable","true")}))
            assertTrue(runCatching{ModelStatusCodec.decode(page(16,null,3).apply(change),projection,first,16)}.isFailure)
        assertTrue(runCatching{ModelStatusCodec.request(projection,null,16)}.isFailure)
        assertTrue(runCatching{ModelStatusCodec.request(projection, first, 32)}.isFailure)
        assertTrue(runCatching{ModelStatusCodec.request(projection.copy(sessionId="foreign"),first,16)}.isFailure)
        val duplicate=page(0,null,2);duplicate["catalog"].asJsonObject["entries"].asJsonArray[1].asJsonObject.addProperty("id","fixture-00")
        assertTrue(runCatching{ModelStatusCodec.decode(duplicate,projection)}.isFailure)
        val unicode=page(0,null,2)
        unicode["catalog"].asJsonObject["entries"].asJsonArray[0].asJsonObject.addProperty("id","\uE000")
        unicode["catalog"].asJsonObject["entries"].asJsonArray[1].asJsonObject.addProperty("id","\uD800\uDC00")
        assertEquals(2,ModelStatusCodec.decode(unicode,projection).catalog?.entries?.size)
    }
    @Test fun catalogPreviousPageScopeRequiresAdjacentSnapshotPage() {
        fun page(offset:Int,next:Int?,count:Int)=value().apply {
            add("reasoning_choices",JsonParser.parseString("[null,\"low\"]"))
            add("catalog",JsonObject().apply{addProperty("snapshot_id","a".repeat(64));addProperty("offset",offset);addProperty("next_offset",next)
                add("entries",com.google.gson.JsonArray().apply{repeat(count){index->add(JsonObject().apply{
                    addProperty("id", "page-%03d".format(index + offset));addProperty("status","active");addProperty("selectable",true)
                })}})})
        }
        val first = ModelStatusCodec.decode(page(0, 16, 16), projection)
        val second = ModelStatusCodec.decode(page(16, null, 3), projection, first, 16)
        assertEquals(first.catalog?.snapshotId, second.catalog?.snapshotId)
        assertEquals(0, first.catalog?.offset)
        assertTrue(runCatching { ModelStatusCodec.decode(page(16, null, 3), projection, first.copy(catalog = first.catalog?.copy(snapshotId = "b".repeat(64))), 16) }.isFailure)
        assertTrue(runCatching { ModelStatusCodec.decode(page(32, null, 3), projection, first, 32) }.isFailure)
        val overlap = page(16, null, 3)
        overlap["catalog"].asJsonObject["entries"].asJsonArray[0].asJsonObject.addProperty("id", "page-015")
        assertTrue(runCatching { ModelStatusCodec.decode(overlap, projection, first, 16) }.isFailure)
        assertTrue(runCatching { ModelStatusCodec.decode(page(0, 16, 16), projection, second.copy(catalog = second.catalog?.copy(
            entries = listOf(RemoteModelChoice("page-015", "active", true)))), 0) }.isFailure)
    }
    @Test fun retainedModelPagesRequireOneAdjacentScopedRevisionAndBoundedHistory() {
        val original = ModelStatusCodec.decode(value(), projection)
        fun page(offset: Int) = original.copy(catalog = RemoteModelCatalog("a".repeat(64), offset, offset + 16,
            (offset until offset + 16).map { RemoteModelChoice("page-%03d".format(it), "active", true) }))
        var history = emptyList<RemoteModelStatus>(); var current = page(0)
        for (offset in 16..144 step 16) {
            val next = page(offset); history = ModelStatusCodec.remember(history, current, next); current = next
        }
        assertEquals((32..128 step 16).toList(), history.map { it.catalog?.offset })
        for (next in listOf(page(176), page(160).copy(sessionId = "foreign"), page(160).copy(selectionRevision = "9"),
            page(160).copy(catalog = page(160).catalog?.copy(snapshotId = "b".repeat(64))),
            page(160).copy(catalog = page(160).catalog?.copy(entries = requireNotNull(history.first().catalog).entries))))
            assertTrue(runCatching { ModelStatusCodec.remember(history, current, next) }.isFailure)
        assertTrue(runCatching { ModelStatusCodec.remember(history.reversed(), current, page(160)) }.isFailure)
        assertEquals("0", projection.eventSequence)
    }
    @Test fun modelSelectionScopeRequiresCurrentProjectAndSessionMembership() {
        val status = ModelStatusCodec.decode(value(), projection)
        assertTrue(ModelStatusCodec.sameScope(status, projection))
        assertFalse(ModelStatusCodec.sameScope(status, null))
        assertFalse(ModelStatusCodec.sameScope(status, projection.copy(sessionId = "foreign")))
        assertFalse(ModelStatusCodec.sameScope(status, projection.copy(projects = emptyList())))
        assertFalse(ModelStatusCodec.sameScope(status.copy(projectId = "foreign"), projection))
    }
    @Test fun reasoningChoicesAreHostBoundAndDefaultIsNotExplicitNone() {
        val live = projection.copy(projects = listOf(projection.projects.single().copy(reasoningControl = true)), workspaceOwner = "a".repeat(64), workspaceRevision = "3")
        val supplied = value().apply { add("reasoning_choices", JsonParser.parseString("[null,\"none\",\"low\",\"high\"]")) }
        val status = ModelStatusCodec.decode(supplied, live)
        assertEquals(listOf(null, "none", "low", "high"), status.reasoningChoices)
        for (effort in listOf<String?>(null,"none","low")) {
            val command = WorkspaceCommand.selectReasoning(live, status, effort)
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            val result = JsonObject().apply {
                addProperty("request_id",command.envelope["id"].asString); addProperty("project_id","project");addProperty("session_id","session")
                addProperty("reasoning_effort",effort);addProperty("selection_revision",9)
            }
            val response = JsonObject().apply {
                addProperty("id",command.envelope["id"].asString);addProperty("channel","control");addProperty("execution_state","succeeded")
                add("payload",JsonObject().apply {addProperty("type","response");add("result",result)})
            }
            assertEquals(result,WorkspaceReceiptCodec.validate(command,response))
            val wrong = response.deepCopy();wrong["payload"].asJsonObject["result"].asJsonObject.addProperty("reasoning_effort",if(effort==null) "none" else null)
            assertTrue(runCatching {WorkspaceReceiptCodec.validate(command,wrong)}.isFailure)
            val malformed = command.envelope;malformed["payload"].asJsonObject["arguments"].asJsonObject.addProperty("reasoning_effort", "default")
            assertTrue(runCatching {WorkspaceCommand.restore(malformed)}.isFailure)
        }
        assertTrue(runCatching {WorkspaceCommand.selectReasoning(live,status,"ultra")}.isFailure)
        assertTrue(runCatching {WorkspaceCommand.selectReasoning(live,status.copy(sessionId="foreign"),null)}.isFailure)
        assertTrue(runCatching {WorkspaceCommand.selectReasoning(live,status.copy(registryBound=false),null)}.isFailure)
        assertTrue(runCatching {WorkspaceCommand.selectReasoning(live.copy(projects=listOf(live.projects.single().copy(reasoningControl=false))),status,null)}.isFailure)
        assertTrue(ModelStatusCodec.decode(value(), live).reasoningChoices.isEmpty())
        assertEquals(listOf("ultra"),ModelStatusCodec.decode(value().apply {add("reasoning_choices",JsonParser.parseString("[\"ultra\"]"))},live).reasoningChoices)
        for(raw in listOf("[null,null]","[\"low\",\"low\"]","[\"default\"]","[false]","{}","[0]"))
            assertTrue(runCatching {ModelStatusCodec.decode(value().apply {add("reasoning_choices",JsonParser.parseString(raw))},live)}.isFailure)
    }
    @Test fun modelSelectionBindsScopeAndRevisionAndValidatesDurableReceipt() {
        val live = projection.copy(projects = listOf(projection.projects.single().copy(promptControl = true, modelControl = true)),
            workspaceOwner = "a".repeat(64), workspaceRevision = "3")
        val status = ModelStatusCodec.decode(value(), live)
        val command = WorkspaceCommand.selectModel(live, status, "gpt-next")
        assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        val args = command.envelope["payload"].asJsonObject["arguments"].asJsonObject
        assertEquals("7", args["selection_revision"].toString())
        val result = JsonObject().apply {
            addProperty("request_id", command.envelope["id"].asString); addProperty("project_id", "project")
            addProperty("session_id", "session"); addProperty("model", "gpt-next"); addProperty("selection_revision", 9)
        }
        val response = JsonObject().apply {
            addProperty("id", command.envelope["id"].asString); addProperty("channel", "control"); addProperty("execution_state", "succeeded")
            add("payload", JsonObject().apply { addProperty("type", "response"); add("result", result) })
        }
        assertEquals(result, WorkspaceReceiptCodec.validate(command, response))
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("model", "other") }, { it.addProperty("project_id", "other") },
            { it.addProperty("request_id", "other") }, { it.addProperty("session_id", "other") },
            { it.addProperty("selection_revision", 7) }, { it.addProperty("selection_revision", "9") },
            { it.addProperty("extra", true) })) {
            val invalid = response.deepCopy(); change(invalid["payload"].asJsonObject["result"].asJsonObject)
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, invalid) }.isFailure)
        }
        for (model in listOf("", "/host/path", "https://private.invalid/model", "x".repeat(129), "model\u0000name"))
            assertTrue(runCatching { WorkspaceCommand.selectModel(live, status, model) }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.selectModel(live, status.copy(sessionId = "foreign"), "next") }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.selectModel(live, status.copy(registryBound = false), "next") }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.selectModel(live.copy(projects = listOf(live.projects.single().copy(modelControl = false))), status, "next") }.isFailure)
    }
    private val projection = HostProjection(listOf(RemoteProject("project", "Project", "session")), "project", "session", "idle", emptyList(), "1", false, "0")
    private fun value() = JsonParser.parseString("""{"project_id":"project","session_id":"session","model":"gpt-fixture","reasoning_effort":"high","registry_bound":true,"selection_revision":7,"capabilities":{"text":true,"images":true,"files":false,"tools":true,"structured_output":true,"streaming":true,"reasoning":true},"budget":{"max_tokens":128000,"reserved_output_tokens":8192}}""").asJsonObject
    @Test fun statusReadCarriesNoWriteAuthorityAndStrictMetadataDoesNotMutateCursors() {
        val request = ModelStatusCodec.request(projection)
        FrameCodec.decode(request.toString().toByteArray())
        assertFalse(request.has("required_owner")); assertFalse(request.has("expected_revision"))
        val status = ModelStatusCodec.decode(value(), projection)
        assertEquals("gpt-fixture", status.model); assertEquals("high", status.reasoningEffort)
        assertEquals("128000", status.maxTokens); assertFalse(requireNotNull(status.capabilities).getValue("files"))
        assertEquals("0", projection.eventSequence)
        val unknown = value().apply { add("model", com.google.gson.JsonNull.INSTANCE); add("reasoning_effort", com.google.gson.JsonNull.INSTANCE); addProperty("registry_bound", false) }
        assertNull(ModelStatusCodec.decode(unknown, projection).model)
        unknown.add("capabilities", com.google.gson.JsonNull.INSTANCE)
        assertNull(ModelStatusCodec.decode(unknown, projection).capabilities)
        unknown.addProperty("reasoning_effort", "ultra")
        assertEquals("ultra", ModelStatusCodec.decode(unknown, projection).reasoningEffort)
    }
    @Test fun foreignScopeConnectionFieldsFlagsLabelsAndUnsignedNumbersFailClosed() {
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("session_id", "other") }, { it.addProperty("project_id", "other") },
            { it.addProperty("base_url", "secret") }, { it.addProperty("model", "x".repeat(129)) },
            { it.addProperty("reasoning_effort", "automatic") }, { it.addProperty("registry_bound", "true") },
            { it.addProperty("selection_revision", "7") }, { it.addProperty("selection_revision", -1) },
            { it["capabilities"].asJsonObject.addProperty("files", 1) }, { it["capabilities"].asJsonObject.addProperty("secret", true) },
            { it["budget"].asJsonObject.addProperty("max_tokens", 0.5) },
            { it["budget"].asJsonObject.add("max_tokens", JsonParser.parseString("18446744073709551616")) }))
            assertTrue(runCatching { ModelStatusCodec.decode(value().apply(change), projection) }.isFailure)
        assertTrue(runCatching { ModelStatusCodec.request(projection.copy(selectedId = null)) }.isFailure)
    }
}
