package zenpi

import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class DiffProjectionTest {
    @Test fun invalidInactiveTabsAndCapacityCannotAuthorizeAnyDiffReadOrCurrentPage() {
        val diff = DiffProjectionCodec.decode(value(), projection, "file.txt")
        val first = diff.copy(diff = "abc", page = DiffPage("a".repeat(64), 0, 3, 6, true))
        val tabs = requireNotNull(projection.subtabs)
        val inactive = RemoteSubtab("e".repeat(64), "Other", "in_place", 1)
        val healthy = projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + inactive))
        val state = zenpi.pairing.PairingState(phase = zenpi.pairing.PairingPhase.CONNECTED,
            projection = healthy, diffOpen = true, diff = first)
        assertTrue(zenpi.pairing.DiffReadPolicy.navigation(state))
        for (invalid in listOf(tabs.copy(tabs = tabs.tabs + listOf(inactive, inactive)),
            tabs.copy(tabs = tabs.tabs + (1..32).map { inactive.copy(id = "%064x".format(it)) }))) {
            val changed = projection.copy(subtabs = invalid)
            assertFalse(DiffProjectionCodec.readable(changed))
            assertFalse(DiffProjectionCodec.sameSelection(diff, changed))
            assertFalse(DiffProjectionCodec.sameSelection(first, changed))
            for (paged in listOf(false, true)) assertTrue(runCatching {
                DiffProjectionCodec.request(changed, "file.txt", paged = paged)
            }.isFailure)
            assertTrue(runCatching { DiffProjectionCodec.request(changed, "file.txt", first, true) }.isFailure)
            assertTrue(runCatching { DiffProjectionCodec.decode(value(), changed, "file.txt") }.isFailure)
            assertFalse(zenpi.pairing.DiffReadPolicy.browse(state.copy(projection = changed)))
            assertFalse(zenpi.pairing.DiffReadPolicy.navigation(state.copy(projection = changed)))
        }
        assertTrue(DiffProjectionCodec.sameSelection(first, healthy.copy(eventSequence = "2")))
        assertFalse(DiffProjectionCodec.request(healthy, "file.txt", first, true).has("required_owner"))
    }

    private val tab = "b".repeat(64)
    private val projection = HostProjection(listOf(RemoteProject("project", "Project", "session", diffAvailable = true)),
        "project", "session", "idle", emptyList(), "1", false, subtabs = RemoteSubtabs("project", "7", tab,
            listOf(RemoteSubtab(tab, "main", "main", 1)), false))
    private fun value() = JsonParser.parseString("""{"project_id":"project","session_id":"session","subtab_id":"$tab","subtab_revision":7,"path":"file.txt","changed":true,"binary":false,"truncated":false,"diff":"+changed","status":" M file.txt"}""").asJsonObject
    @Test fun mixedDuplicateProjectsRefuseLegacyPagedAndContinuationReads() {
        val diff = DiffProjectionCodec.decode(value(), projection, "file.txt")
        val ambiguous = projection.copy(projects = projection.projects + projection.projects.single().copy(sessionId = "foreign"))
        assertTrue(DiffProjectionCodec.sameSelection(diff, projection))
        assertFalse(DiffProjectionCodec.sameSelection(diff, ambiguous))
        for (paged in listOf(false, true)) assertTrue(runCatching { DiffProjectionCodec.request(ambiguous, "file.txt", paged = paged) }.isFailure)
        assertTrue(runCatching { DiffProjectionCodec.decode(value(), ambiguous, "file.txt") }.isFailure)
        val first = diff.copy(diff = "abc", page = DiffPage("a".repeat(64), 0, 3, 6, true))
        assertTrue(runCatching { DiffProjectionCodec.request(ambiguous, "file.txt", first, true) }.isFailure)
    }
    @Test fun malformedHistoricalBytesCannotAuthorizeNavigationOrWireContinuations() {
        val diff = DiffProjectionCodec.decode(value(), projection, "file.txt")
        val first = diff.copy(diff = "abc", page = DiffPage("a".repeat(64), 0, 3, 9, true))
        val second = first.copy(diff = "def", page = first.page?.copy(offset = 3, nextOffset = 6))
        val third = first.copy(diff = "ghi", page = first.page?.copy(offset = 6, nextOffset = 9, hasMore = false))
        assertTrue(DiffProjectionCodec.validChain(listOf(first), second))
        val state = zenpi.pairing.PairingState(phase = zenpi.pairing.PairingPhase.CONNECTED, projection = projection,
            diffOpen = true, diff = second, diffPrevious = listOf(first))
        assertTrue(zenpi.pairing.DiffReadPolicy.navigation(state))
        assertFalse(zenpi.pairing.DiffReadPolicy.navigation(state.copy(diffOpen = false)))
        for (bad in listOf(first.copy(diff = "a"), first.copy(page = first.page?.copy(digest = "invalid")),
            first.copy(page = first.page?.copy(offset = -1)), first.copy(page = first.page?.copy(nextOffset = 4)),
            first.copy(page = first.page?.copy(totalBytes = 1048577)), first.copy(page = first.page?.copy(hasMore = false)),
            first.copy(diff = "x".repeat(8193)), first.copy(status = "x".repeat(32769)), first.copy(path = "credentials.json"))) {
            assertFalse(DiffProjectionCodec.sameSelection(bad, projection))
            assertTrue(runCatching { DiffProjectionCodec.request(projection, bad.path, bad, true) }.isFailure)
            assertFalse(DiffProjectionCodec.validChain(listOf(bad), second))
            assertFalse(zenpi.pairing.DiffReadPolicy.navigation(state.copy(diffPrevious = listOf(bad))))
            assertTrue(runCatching { DiffProjectionCodec.previous(listOf(bad), second) }.isFailure)
            assertTrue(runCatching { DiffProjectionCodec.remember(listOf(bad), second, third) }.isFailure)
            assertTrue(runCatching { DiffProjectionCodec.remember(emptyList(), null, bad) }.isFailure)
        }
        assertTrue(runCatching { DiffProjectionCodec.remember(emptyList(), null, second) }.isFailure)
        assertTrue(runCatching { DiffProjectionCodec.previous(List(8) { first }, second) }.isFailure)
        assertEquals(listOf(first, second), DiffProjectionCodec.remember(listOf(first), second, third))
        assertFalse(DiffProjectionCodec.validChain(listOf(first.copy(status = "different")), second))
        assertFalse(DiffProjectionCodec.validChain(listOf(first, first), second))
    }
    @Test fun readRequestHasExactRouteAndNoMutationAuthority() {
        val request = DiffProjectionCodec.request(projection, "file.txt")
        assertEquals(request, FrameCodec.decode(request.toString().toByteArray()))
        assertFalse(request.has("required_owner")); assertFalse(request.has("expected_revision"))
        assertEquals("diff", request["payload"].asJsonObject["operation"].asString)
        assertEquals("+changed", DiffProjectionCodec.decode(value(), projection, "./file.txt").diff)
    }
    @Test fun unsafePathOrMissingCapabilityFails() {
        for (path in listOf("", "../private", "/private", "a\n", "a".repeat(4097)))
            assertTrue(runCatching { DiffProjectionCodec.request(projection, path) }.isFailure)
        assertTrue(runCatching { DiffProjectionCodec.request(projection.copy(projects = listOf(projection.projects.single().copy(diffAvailable = false))), ".") }.isFailure)
    }
    @Test fun privatePathsCannotBeRequestedOrDecodedButOrdinaryDotAndLiteralNamesRemainValid() {
        for (name in listOf("credentials.json", "AUTH.JSON", "id_rsa", "private.PEM", "PRIVATE.KEY", "cert.p12",
            ".env", ".env.local", ".git/config", "credentials/opaque.txt", ".ssh/note.txt", ".zenpi/note.txt")) {
            for (path in listOf(name, "nested/$name", "./nested/$name")) {
                assertTrue(path, runCatching { DiffProjectionCodec.request(projection, path) }.isFailure)
                val response = value().apply { addProperty("path", path) }
                assertTrue(path, runCatching { DiffProjectionCodec.decode(response, projection, path) }.isFailure)
            }
        }
        for (path in listOf(".", ".editorconfig", "nested/.ignore", "credentials.json.txt", "[literal]*.txt", ":literal.txt")) {
            DiffProjectionCodec.request(projection, path)
            assertEquals(path, DiffProjectionCodec.decode(value().apply { addProperty("path", path) }, projection, path).path)
        }
    }
    @Test fun wrongIdentityRevisionPathFlagsAndBoundsFail() {
        for (field in listOf("project_id", "session_id", "subtab_id", "path", "subtab_revision", "changed", "binary", "truncated")) {
            val bad = value().apply { addProperty(field, "wrong") }
            assertTrue(field, runCatching { DiffProjectionCodec.decode(bad, projection, "file.txt") }.isFailure)
        }
        for ((field, limit) in listOf("diff" to 65536, "status" to 32768)) {
            assertTrue(runCatching { DiffProjectionCodec.decode(value().apply { addProperty(field, "x".repeat(limit + 1)) }, projection, "file.txt") }.isFailure)
        }
        assertTrue(runCatching { DiffProjectionCodec.decode(value().apply { addProperty("extra", true) }, projection, "file.txt") }.isFailure)
    }
    @Test fun pagesBindUtf8OffsetsScopeDigestAndMetadataWithoutWriteAuthority() {
        fun page(offset: Int, text: String, more: Boolean) = value().apply {
            addProperty("diff", text)
            add("page", JsonParser.parseString("""{"digest":"${"a".repeat(64)}","offset":$offset,"next_offset":${offset + text.toByteArray().size},"total_bytes":6,"has_more":$more}"""))
        }
        val first = DiffProjectionCodec.decode(page(0, "\u4e2d", true), projection, "file.txt", paged = true)
        val request = DiffProjectionCodec.request(projection, "file.txt", first, true)
        FrameCodec.decode(request.toString().toByteArray()); assertFalse(request.has("required_owner"))
        assertEquals(3, request["payload"].asJsonObject["arguments"].asJsonObject["offset"].asInt)
        val tail = DiffProjectionCodec.decode(page(3, "end", false), projection, "file.txt", first, true)
        assertEquals(listOf(first), DiffProjectionCodec.remember(emptyList(), first, tail))
        assertEquals(first, DiffProjectionCodec.previous(listOf(first), tail))
        assertTrue(runCatching { DiffProjectionCodec.request(projection, "file.txt", tail, true) }.isFailure)
        for (change in listOf<(com.google.gson.JsonObject) -> Unit>(
            { it["page"].asJsonObject.addProperty("digest", "b".repeat(64)) }, { it["page"].asJsonObject.addProperty("offset", 2) },
            { it["page"].asJsonObject.addProperty("next_offset", 5) }, { it["page"].asJsonObject.addProperty("total_bytes", 7) },
            { it["page"].asJsonObject.addProperty("has_more", true) }, { it.addProperty("status", "changed") }, { it.addProperty("truncated", true) }))
            assertTrue(runCatching { DiffProjectionCodec.decode(page(3, "end", false).apply(change), projection, "file.txt", first, true) }.isFailure)
        assertFalse(DiffProjectionCodec.sameSelection(first, projection.copy(sessionId = "other")))
        assertTrue(runCatching { DiffProjectionCodec.request(projection.copy(subtabs = projection.subtabs?.copy(projectId = "other")), ".") }.isFailure)
        assertTrue(runCatching { DiffProjectionCodec.previous(listOf(first.copy(status = "different")), tail) }.isFailure)
    }
}
