package zenpi

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.pairing.*

class SessionCatalogTest {
    @Test fun opaqueSessionIdentitySurvivesCatalogDurableResumeAndMatchingReceipt() {
        for (id in listOf("history:session/\u4e2d", "\u754c".repeat(42) + "ab", " leading trailing ")) {
            val value = page(16, 1).apply { addProperty("offset", 0)
                getAsJsonArray("entries")[0].asJsonObject.addProperty("session_id", id) }
            val catalog = SessionCatalogCodec.decode(value, projection, null, 0)
            val command = SessionCommandCodec.resume(projection, catalog, id)
            assertEquals(id, command.envelope.getAsJsonObject("payload").getAsJsonObject("arguments")["target_session_id"].asString)
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            val receipt = JsonObject().apply {
                add("request_id", command.envelope["id"]); addProperty("project_id", project.id)
                addProperty("old_session_id", "current"); addProperty("new_session_id", id)
                addProperty("required_owner", "c".repeat(64)); addProperty("workspace_revision", 4)
                addProperty("active_project_id", project.id)
            }
            SessionCommandCodec.receipt(command.envelope, receipt)
            assertTrue(SessionCatalogCodec.validChain(emptyList(), catalog))
            assertTrue(runCatching { SessionCommandCodec.receipt(command.envelope,
                receipt.deepCopy().apply { addProperty("new_session_id", "foreign") }) }.isFailure)
        }
        val valid = SessionCatalogCodec.decode(page(), projection, null, 0)
        val command = SessionCommandCodec.resume(projection, valid, "history-0")
        for (id in listOf("", " ", "bad\nidentity", "bad\u007fidentity", "x".repeat(129), "\u754c".repeat(43))) {
            val value = page().apply { getAsJsonArray("entries")[0].asJsonObject.addProperty("session_id", id) }
            assertTrue(runCatching { SessionCatalogCodec.decode(value, projection, null, 0) }.isFailure)
            val retained = valid.copy(entries = valid.entries.mapIndexed { index, entry -> if (index == 0) entry.copy(id = id) else entry })
            assertFalse(SessionCatalogCodec.sameScope(retained, projection))
            assertTrue(runCatching { WorkspaceCommand.restore(command.envelope.apply {
                getAsJsonObject("payload").getAsJsonObject("arguments").addProperty("target_session_id", id)
            }) }.isFailure)
        }
    }
    @Test fun malformedRetainedCatalogCannotAuthorizeResumeOrContinuationButExactIntentCanRetry() {
        val catalog = SessionCatalogCodec.decode(page(), projection, null, 0)
        val command = SessionCommandCodec.resume(projection, catalog, "history-0")
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, sessionsOpen = true, sessions = catalog)
        for (bad in listOf(catalog.copy(snapshotId = "invalid"), catalog.copy(offset = 1), catalog.copy(nextOffset = 32),
            catalog.copy(entries = catalog.entries.take(1)), catalog.copy(entries = catalog.entries + catalog.entries.first()),
            catalog.copy(query = " padded "), catalog.copy(entries = catalog.entries.mapIndexed { index, entry ->
                if (index == 0) entry.copy(current = true) else entry }),
            catalog.copy(entries = catalog.entries.mapIndexed { index, entry -> if (index == 0) entry.copy(createdAtMs = "01") else entry }),
            catalog.copy(entries = catalog.entries.mapIndexed { index, entry -> if (index == 0) entry.copy(turnCount = -1) else entry }),
            catalog.copy(entries = catalog.entries.mapIndexed { index, entry -> if (index == 0) entry.copy(name = "bad/name") else entry }),
            catalog.copy(entries = catalog.entries.mapIndexed { index, entry -> if (index == 0) entry.copy(matches = 1) else entry }))) {
            assertFalse(SessionCatalogCodec.sameScope(bad, projection))
            assertFalse(SessionCatalogCodec.resumable(bad, projection, "history-0"))
            assertTrue(runCatching { SessionCommandCodec.resume(projection, bad, "history-0") }.isFailure)
            assertTrue(runCatching { SessionCatalogCodec.request(projection, bad, 16) }.isFailure)
            val blocked = state.copy(sessions = bad)
            assertFalse(SessionCatalogReadPolicy.navigation(blocked))
            assertTrue(SessionCatalogReadPolicy.browse(blocked))
            assertFalse(WorkspaceDispatchPolicy.allows(blocked, true, true, command))
            assertTrue(WorkspaceDispatchPolicy.allows(blocked.copy(pendingWorkspace = command), true, true, command))
        }
        val tail = SessionCatalogCodec.decode(page(16, 1), projection, catalog, 16)
        val resume = SessionCommandCodec.resume(projection, tail, "history-16")
        val healthy = state.copy(sessions = tail, sessionsPrevious = listOf(catalog))
        assertTrue(SessionCatalogReadPolicy.navigation(healthy))
        assertTrue(WorkspaceDispatchPolicy.allows(healthy, true, true, resume))
        for (history in listOf(listOf(catalog.copy(snapshotId = "e".repeat(64))),
            listOf(catalog.copy(entries = catalog.entries.dropLast(1) + tail.entries.single())), List(8) { catalog })) {
            val blocked = healthy.copy(sessionsPrevious = history)
            assertFalse(SessionCatalogReadPolicy.navigation(blocked))
            assertFalse(WorkspaceDispatchPolicy.allows(blocked, true, true, resume))
            assertTrue(runCatching { SessionCatalogCodec.remember(history, tail, catalog) }.isFailure)
        }
    }

    private val project = RemoteProject("a".repeat(64), "Workspace", "current", sessionControl = true)
    private val tab = "b".repeat(64)
    private val projection = HostProjection(listOf(project), project.id, "current", "idle", emptyList(), "1", false, "0",
        workspaceOwner = "c".repeat(64), workspaceRevision = "3", subtabs = RemoteSubtabs(project.id, "1", tab,
            listOf(RemoteSubtab(tab, "Main", "main", 1)), true))
    private fun page(offset: Int = 0, count: Int = 16) = JsonObject().apply {
        addProperty("project_id", project.id); addProperty("session_id", "current"); addProperty("snapshot_id", "d".repeat(64))
        addProperty("offset", offset); if (offset == 0) addProperty("next_offset", 16) else add("next_offset", com.google.gson.JsonNull.INSTANCE)
        add("entries", JsonArray().apply { repeat(count) { index -> add(JsonObject().apply {
            addProperty("session_id", "history-${offset + index}"); addProperty("created_at_ms", 10); addProperty("turn_count", 2); addProperty("current", false)
        }) } })
    }
    @Test fun pagedReadAndResumeBindCatalogScopeIdentityAndDurableRequest() {
        val first = SessionCatalogCodec.decode(page(), projection, null, 0)
        val request = SessionCatalogCodec.request(projection, first, 16)
        FrameCodec.decode(request.toString().toByteArray()); assertFalse(request.has("required_owner"))
        val second = SessionCatalogCodec.decode(page(16, 1), projection, first, 16)
        assertNull(second.nextOffset)
        val command = SessionCommandCodec.resume(projection, second, "history-16")
        assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        val args = command.envelope["payload"].asJsonObject["arguments"].asJsonObject
        assertEquals(setOf("target_session_id", "snapshot_id"), args.keySet())
        assertEquals("history-16", args["target_session_id"].asString)
        assertEquals(first.snapshotId, args["snapshot_id"].asString)
        assertTrue(runCatching { SessionCommandCodec.resume(projection, second, "history-0") }.isFailure)
        assertTrue(runCatching { SessionCommandCodec.resume(projection.copy(sessionId = "foreign"), second, "history-16") }.isFailure)
    }
    @Test fun malformedMetadataPagingAndChangedSnapshotNeverBecomeCatalog() {
        val first = SessionCatalogCodec.decode(page(), projection, null, 0)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("project_id", "foreign") }, { it.addProperty("session_id", "foreign") },
            { it.addProperty("snapshot_id", "e".repeat(64)) }, { it.addProperty("offset", "16") },
            { it.addProperty("next_offset", 32) }, { it.addProperty("path", "/private") },
            { it.getAsJsonArray("entries")[0].asJsonObject.addProperty("current", true) },
            { it.getAsJsonArray("entries")[0].asJsonObject.addProperty("turn_count", -1) },
            { it.getAsJsonArray("entries")[0].asJsonObject.addProperty("created_at_ms", 10.0) },
            { it.getAsJsonArray("entries")[0].asJsonObject.addProperty("session_id", "bad\nidentity") })) {
            assertTrue(runCatching { SessionCatalogCodec.decode(page(16, 1).apply(change), projection, first, 16) }.isFailure)
        }
        assertTrue(runCatching { SessionCatalogCodec.decode(page(count = 17), projection, null, 0) }.isFailure)
        assertTrue(runCatching { SessionCatalogCodec.request(projection, null, 16) }.isFailure)
    }
    @Test fun authorityLossClearsCatalogAndResumeReceiptRequiresChosenTarget() {
        val catalog = SessionCatalogCodec.decode(page(), projection, null, 0)
        val cleared = AuthorityLossPolicy.displayOnly(PairingState(projection = projection, sessionsOpen = true, sessions = catalog, sessionsPrevious = listOf(catalog), sessionsQuery = "history"))
        assertFalse(cleared.sessionsOpen); assertNull(cleared.sessions); assertTrue(cleared.sessionsPrevious.isEmpty())
        assertNull(cleared.sessionsQuery)
        val command = SessionCommandCodec.resume(projection, catalog, "history-0")
        val result = JsonObject().apply {
            add("request_id", command.envelope["id"]); addProperty("project_id", project.id); addProperty("old_session_id", "current")
            addProperty("new_session_id", "history-0"); addProperty("required_owner", "c".repeat(64)); addProperty("workspace_revision", 4)
            addProperty("active_project_id", project.id)
        }
        SessionCommandCodec.receipt(command.envelope, result)
        assertTrue(runCatching { SessionCommandCodec.receipt(command.envelope, result.apply { addProperty("new_session_id", "history-1") }) }.isFailure)
    }
    @Test fun catalogReadsRequireAdjacentPagesDistinctIdsAndNoEmptyContinuation() {
        val first = SessionCatalogCodec.decode(page(), projection, null, 0)
        val last = SessionCatalogCodec.decode(page(16, 1), projection, first, 16)
        SessionCatalogCodec.request(projection, last, 0)
        assertEquals(first, SessionCatalogCodec.decode(page(), projection, last, 0))
        for ((previous, offset) in listOf(null to 16, first to 32, first to 0, last to 32, last to 16, last to 512)) {
            assertTrue(runCatching { SessionCatalogCodec.request(projection, previous, offset) }.isFailure)
            assertTrue(runCatching { SessionCatalogCodec.decode(page(offset, 1), projection, previous, offset) }.isFailure)
        }
        val duplicate = page(16, 1).apply { getAsJsonArray("entries")[0].asJsonObject.addProperty("session_id", "history-0") }
        assertTrue(runCatching { SessionCatalogCodec.decode(duplicate, projection, first, 16) }.isFailure)
        assertTrue(runCatching { SessionCatalogCodec.decode(page(16, 0), projection, first, 16) }.isFailure)
        val empty = page(count = 0).apply { add("next_offset", com.google.gson.JsonNull.INSTANCE) }
        assertTrue(SessionCatalogCodec.decode(empty, projection, null, 0).entries.isEmpty())
    }
    @Test fun catalogNavigationRequiresRetainedBackPageAndRejectsNonAdjacentRepeatedSession() {
        fun catalog(offset: Int) = SessionCatalog(project.id, "current", "d".repeat(64), offset, offset + 16,
            (offset until offset + 16).map { RemoteSession("history-$it", "10", 2, false) })
        var pages = emptyList<SessionCatalog>()
        var current = catalog(0)
        for (offset in 16..144 step 16) {
            val next = catalog(offset)
            pages = SessionCatalogCodec.remember(pages, current, next)
            current = next
        }
        assertEquals((32..128 step 16).toList(), pages.map { it.offset })
        val previous = requireNotNull(pages.lastOrNull())
        assertEquals(pages.dropLast(1), SessionCatalogCodec.remember(pages, current, previous))
        assertTrue(runCatching { SessionCatalogCodec.remember(emptyList(), current, previous) }.isFailure)
        assertTrue(runCatching { SessionCatalogCodec.remember(pages, current, previous.copy(entries = emptyList())) }.isFailure)
        assertTrue(runCatching { SessionCatalogCodec.remember(pages, current, catalog(160).copy(entries = pages.first().entries)) }.isFailure)
        assertTrue(runCatching { SessionCatalogCodec.remember(pages.reversed(), current, catalog(160)) }.isFailure)
        assertTrue(runCatching { SessionCatalogCodec.remember(pages, current, catalog(176)) }.isFailure)
        assertEquals(emptyList<SessionCatalog>(), SessionCatalogCodec.remember(pages, null, catalog(0)))
    }
    @Test fun searchPagesRetainExactQueryAndCannotMixWithUnfilteredCatalog() {
        val query = "History"
        val first = SessionCatalogCodec.decode(page().apply { addProperty("query", query) }, projection, null, 0, query)
        val request = SessionCatalogCodec.request(projection, first, 16)
        assertEquals(query, request["payload"].asJsonObject["arguments"].asJsonObject["query"].asString)
        val second = SessionCatalogCodec.decode(page(16, 1).apply { addProperty("query", query) }, projection, first, 16)
        assertEquals(query, second.query)
        assertEquals(listOf(first), SessionCatalogCodec.remember(emptyList(), first, second))
        assertEquals("history-16", SessionCommandCodec.resume(projection, second, "history-16")
            .envelope["payload"].asJsonObject["arguments"].asJsonObject["target_session_id"].asString)
        for (different in listOf(null, "Other", "history")) {
            assertTrue(runCatching { SessionCatalogCodec.request(projection, first, 16, different) }.isFailure)
            assertTrue(runCatching { SessionCatalogCodec.remember(emptyList(), first, second.copy(query = different)) }.isFailure)
        }
        assertTrue(runCatching { SessionCatalogCodec.decode(page(), projection, null, 0, query) }.isFailure)
        assertTrue(runCatching { SessionCatalogCodec.decode(page().apply { addProperty("query", "other") }, projection, null, 0, query) }.isFailure)
        for (bad in listOf(" padded ", "bad\nquery", "x".repeat(129))) {
            assertTrue(runCatching { SessionCatalogCodec.request(projection, null, 0, bad) }.isFailure)
        }
        val empty = page(count = 0).apply { addProperty("query", "missing"); add("next_offset", com.google.gson.JsonNull.INSTANCE) }
        assertTrue(SessionCatalogCodec.decode(empty, projection, null, 0, "missing").entries.isEmpty())
    }
    @Test fun searchSnippetsAreBoundedTypedAndNeverAcceptedInUnfilteredPages() {
        fun searched() = page(count = 1).apply {
            addProperty("query", "Mailbox"); add("next_offset", com.google.gson.JsonNull.INSTANCE)
            getAsJsonArray("entries")[0].asJsonObject.apply {
                addProperty("matches", 2)
                add("snippets", JsonArray().apply { add("Mailbox api_key=<redacted>"); add("\uD83D\uDE00".repeat(240)) })
            }
        }
        val decoded = SessionCatalogCodec.decode(searched(), projection, null, 0, "Mailbox")
        assertEquals(2, decoded.entries.single().matches)
        assertEquals(2, decoded.entries.single().snippets.size)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.remove("matches") }, { it.remove("snippets") }, { it.addProperty("matches", -1) },
            { it.addProperty("matches", "2") }, { it.addProperty("matches", 3) },
            { it.add("snippets", JsonArray().apply { add("only one") }) },
            { it.add("snippets", JsonArray().apply { add("x".repeat(241)); add("ok") }) },
            { it.add("snippets", JsonArray().apply { add("bad\u001btext"); add("ok") }) },
            { it.add("snippets", JsonArray().apply { add(42); add("ok") }) })) {
            val value = searched().apply { change(getAsJsonArray("entries")[0].asJsonObject) }
            assertTrue(runCatching { SessionCatalogCodec.decode(value, projection, null, 0, "Mailbox") }.isFailure)
        }
        assertTrue(runCatching { SessionCatalogCodec.decode(searched().apply { remove("query") }, projection, null, 0) }.isFailure)
    }
    @Test fun nameOnlySearchAndLegacyMetadataRemainReadableWithoutMixingQueryModes() {
        val value = page(count = 1).apply {
            addProperty("query", "Named session"); add("next_offset", com.google.gson.JsonNull.INSTANCE)
            getAsJsonArray("entries")[0].asJsonObject.apply {
                addProperty("name", "Named session"); addProperty("matches", 0); add("snippets", JsonArray())
            }
        }
        val decoded = SessionCatalogCodec.decode(value, projection, null, 0, "Named session")
        assertEquals(0, decoded.entries.single().matches)
        assertTrue(decoded.entries.single().snippets.isEmpty())
        assertTrue(SessionCatalogCodec.resumable(decoded, projection, "history-0"))
        val missing = value.deepCopy().apply {
            getAsJsonArray("entries")[0].asJsonObject.apply { remove("matches"); remove("snippets") }
        }
        assertEquals(decoded, SessionCatalogCodec.decode(missing, projection, null, 0, "Named session"))
        assertTrue(runCatching { SessionCatalogCodec.decode(value.deepCopy().apply { remove("query") }, projection, null, 0) }.isFailure)
    }
}
