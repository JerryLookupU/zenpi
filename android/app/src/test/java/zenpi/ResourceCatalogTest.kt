package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class ResourceCatalogTest {
    @Test fun malformedRetainedCommandsCannotAuthorizeInsertionOrCatalogSelection() {
        val catalog = ResourceCatalogCodec.decode(value(), projection)
        val resource = catalog.entries.first()
        val state = zenpi.pairing.PairingState(phase = zenpi.pairing.PairingPhase.CONNECTED,
            projection = projection.copy(projects = projection.projects.map { it.copy(promptControl = true) }), resources = catalog)
        assertTrue(zenpi.pairing.ResourceReadPolicy.insertion(state))
        for (bad in listOf(resource.copy(name = "../outside"), resource.copy(kind = "shell"),
            resource.copy(command = "!touch marker"), resource.copy(command = "/help"),
            resource.copy(command = "/skill:research\n!touch marker"), resource.copy(description = "bad\u0000"),
            resource.copy(description = "x".repeat(1025)))) {
            val invalid = catalog.copy(entries = listOf(bad))
            assertFalse(ResourceCatalogCodec.sameScope(invalid, projection))
            assertFalse(zenpi.pairing.ResourceReadPolicy.navigation(state.copy(resources = invalid)))
            assertFalse(zenpi.pairing.ResourceReadPolicy.insertion(state.copy(resources = invalid)))
            assertTrue(zenpi.pairing.ResourceReadPolicy.browse(state.copy(resources = invalid)))
            assertTrue(runCatching { ResourceCatalogCodec.insert(bad, "draft") }.isFailure)
        }
        for (bad in listOf(catalog.copy(entries = listOf(resource, resource)), catalog.copy(entries = List(33) { resource })))
            assertFalse(ResourceCatalogCodec.sameScope(bad, projection))
        assertTrue(runCatching { ResourceCatalogCodec.insert(resource, "draft\u0000text") }.isFailure)
        assertEquals("/skill:research first\nsecond", ResourceCatalogCodec.insert(resource, "first\nsecond"))
    }

    private val projection = HostProjection(listOf(RemoteProject("project", "Project", "session")), "project", "session", "idle", emptyList(), "1", false, "0")
    private fun value() = JsonParser.parseString("""{"project_id":"project","session_id":"session","generation":2,"snapshot_id":"${"a".repeat(64)}","offset":0,"next_offset":null,"entries":[{"name":"research","kind":"skill","command":"/skill:research","description":"Find sources","invocable":true},{"name":"review","kind":"template","command":"/review","description":"Review change","invocable":false}],"truncated":false}""").asJsonObject
    @Test fun authenticatedReadMetadataAndDraftArgumentsDoNotAdvanceReplayCursor() {
        val request = ResourceCatalogCodec.request(projection)
        FrameCodec.decode(request.toString().toByteArray())
        assertFalse(request.has("required_owner")); assertFalse(request.has("expected_revision"))
        assertEquals(0, request["payload"].asJsonObject["arguments"].asJsonObject.size())
        val catalog = ResourceCatalogCodec.decode(value(), projection)
        assertEquals("2", catalog.generation); assertFalse(catalog.truncated)
        assertEquals("/skill:research literal $1", ResourceCatalogCodec.insert(catalog.entries[0], "literal $1"))
        assertTrue(runCatching { ResourceCatalogCodec.insert(catalog.entries[1], "draft") }.isFailure)
        assertEquals("0", projection.eventSequence)
    }
    @Test fun foreignScopeBodiesCommandsTypesNumbersAndDuplicateEntriesFailClosed() {
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("project_id", "other") }, { it.addProperty("session_id", "other") },
            { it.addProperty("generation", "2") }, { it.addProperty("generation", -1) },
            { it.add("generation", JsonParser.parseString("18446744073709551616")) },
            { it.addProperty("generation", 2.5) }, { it.remove("generation") },
            { it.addProperty("truncated", "false") }, { it.addProperty("source", "/host/secret") },
            { it["entries"].asJsonArray[0].asJsonObject.addProperty("body", "secret") },
            { it["entries"].asJsonArray[0].asJsonObject.addProperty("command", "/help") },
            { it["entries"].asJsonArray[0].asJsonObject.addProperty("kind", "shell") },
            { it["entries"].asJsonArray[0].asJsonObject.addProperty("name", "../secret") },
            { it["entries"].asJsonArray[0].asJsonObject.addProperty("invocable", "true") },
            { it["entries"].asJsonArray[0].asJsonObject.addProperty("description", "a\u0000b") },
            { it["entries"].asJsonArray[0].asJsonObject.addProperty("description", "x".repeat(1025)) },
            { it["entries"].asJsonArray.add(it["entries"].asJsonArray[0].deepCopy()) })) {
            assertTrue(runCatching { ResourceCatalogCodec.decode(value().apply(change), projection) }.isFailure)
        }
        val many = value(); repeat(31) { many["entries"].asJsonArray.add(many["entries"].asJsonArray[0].deepCopy()) }
        assertTrue(runCatching { ResourceCatalogCodec.decode(many, projection) }.isFailure)
    }
    @Test fun utf8LimitUnsignedMaximumAndDraftCommandReplacementAreExplicit() {
        val valid = value().apply { add("generation", JsonParser.parseString("18446744073709551615"))
            get("entries").asJsonArray[0].asJsonObject.addProperty("description", "\u6c49".repeat(341)) }
        val resource = ResourceCatalogCodec.decode(valid, projection).entries[0]
        for (draft in listOf("/other", "  /skill:other", "!command", "x".repeat(65536)))
            assertTrue(runCatching { ResourceCatalogCodec.insert(resource, draft) }.isFailure)
        assertEquals("/skill:research ", ResourceCatalogCodec.insert(resource, ""))
        assertTrue(runCatching { ResourceCatalogCodec.request(projection.copy(sessionId = null)) }.isFailure)
        val dotted = value(); dotted["entries"].asJsonArray[1].asJsonObject.apply {
            addProperty("name", "review.code"); addProperty("command", "/review.code")
        }
        assertEquals("/review.code", ResourceCatalogCodec.decode(dotted, projection).entries[1].command)
    }
    @Test fun pageOffsetsSnapshotIdentityAndContinuationRequireExactBoundaries() {
        val first = value().apply {
            val items = get("entries").asJsonArray; items.remove(1)
            repeat(31) { index -> items.add(items[0].deepCopy().asJsonObject.apply {
                addProperty("name", "research-${index}"); addProperty("command", "/skill:research-${index}")
            }) }
            addProperty("truncated", true); addProperty("next_offset", 32)
        }
        val page = ResourceCatalogCodec.decode(first, projection)
        val request = ResourceCatalogCodec.request(projection, page, 32)
        FrameCodec.decode(request.toString().toByteArray())
        assertEquals(32, request["payload"].asJsonObject["arguments"].asJsonObject["offset"].asInt)
        val last = value().apply { addProperty("offset", 32); getAsJsonArray("entries")[0].asJsonObject.apply {
            addProperty("name", "tail"); addProperty("command", "/skill:tail")
        } }
        val tail = ResourceCatalogCodec.decode(last, projection, page, 32)
        assertEquals(32, tail.offset)
        assertEquals(page.snapshotId, ResourceCatalogCodec.request(projection, tail, 0)["payload"].asJsonObject["arguments"].asJsonObject["snapshot_id"].asString)
        assertEquals(0, ResourceCatalogCodec.decode(first, projection, tail, 0).offset)
        assertTrue(runCatching { ResourceCatalogCodec.decode(first.deepCopy().apply { addProperty("snapshot_id", "b".repeat(64)) }, projection, tail, 0) }.isFailure)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("snapshot_id", "b".repeat(64)) }, { it.addProperty("generation", 3) },
            { it.addProperty("offset", 64) }, { it.addProperty("offset", "32") },
            { it.addProperty("next_offset", 33) }, { it.addProperty("truncated", true) },
            { it.add("entries", com.google.gson.JsonArray()) }))
            assertTrue(runCatching { ResourceCatalogCodec.decode(last.deepCopy().apply(change), projection, page, 32) }.isFailure)
        assertTrue(runCatching { ResourceCatalogCodec.decode(first.deepCopy().apply { addProperty("next_offset", 64) }, projection) }.isFailure)
        assertTrue(runCatching { ResourceCatalogCodec.decode(first.deepCopy().apply { get("entries").asJsonArray.remove(0) }, projection) }.isFailure)
        for (offset in listOf(-1, 1, 64, 5152)) assertTrue(runCatching { ResourceCatalogCodec.request(projection, page, offset) }.isFailure)
        assertTrue(runCatching { ResourceCatalogCodec.request(projection.copy(sessionId = "other"), page, 32) }.isFailure)
    }
    @Test fun previousCatalogPageCarriesSnapshotAndRejectsGenerationChange() {
        val first = ResourceCatalogCodec.decode(value(), projection)
        val later = first.copy(offset = 32, nextOffset = null, generation = first.generation)
        assertEquals(first.snapshotId, ResourceCatalogCodec.request(projection, later, 0)["payload"].asJsonObject["arguments"].asJsonObject["snapshot_id"].asString)
        assertTrue(runCatching { ResourceCatalogCodec.decode(value().apply { addProperty("offset", 32) }, projection, later.copy(generation = "3"), 32) }.isFailure)
    }
    @Test fun resourceContinuationRejectsDuplicateAndSkippedPagesWithoutChangingCursor() {
        val decoded = ResourceCatalogCodec.decode(value(), projection)
        val first = decoded.copy(truncated = true, nextOffset = 32, entries = decoded.entries + (2 until 32).map {
            RemoteResource("entry-$it", "template", "/entry-$it", "", true)
        })
        val duplicate = value().apply { addProperty("offset", 32) }
        assertTrue(runCatching { ResourceCatalogCodec.decode(duplicate, projection, first, 32) }.isFailure)
        assertTrue(runCatching { ResourceCatalogCodec.decode(duplicate.deepCopy().apply { addProperty("offset", 64) }, projection, first, 64) }.isFailure)
        assertEquals("0", projection.eventSequence)
        assertTrue(ResourceCatalogCodec.sameScope(first, projection))
        assertFalse(ResourceCatalogCodec.sameScope(first, projection.copy(sessionId = "other")))
        assertFalse(ResourceCatalogCodec.sameScope(first.copy(projectId = "other"), projection))
        assertFalse(ResourceCatalogCodec.sameScope(first, null))
    }
    @Test fun retainedCatalogChainRejectsNonAdjacentDuplicatesAndGenerationChanges() {
        fun page(offset: Int) = ResourceCatalog("project", "session", "2", (offset until offset + 32).map {
            RemoteResource("entry-$it", "template", "/entry-$it", "", true) }, true, "a".repeat(64), offset, offset + 32)
        var pages = emptyList<ResourceCatalog>(); var current = page(0)
        for (offset in 32..288 step 32) {
            val next = page(offset); pages = ResourceCatalogCodec.remember(pages, current, next); current = next
        }
        assertEquals((64..256 step 32).toList(), pages.map { it.offset })
        for (next in listOf(page(320).copy(entries = pages.first().entries), page(320).copy(generation = "3"),
            page(320).copy(snapshotId = "b".repeat(64)), page(352))) {
            assertTrue(runCatching { ResourceCatalogCodec.remember(pages, current, next) }.isFailure)
        }
        assertTrue(runCatching { ResourceCatalogCodec.remember(pages.reversed(), current, page(320)) }.isFailure)
    }
}
