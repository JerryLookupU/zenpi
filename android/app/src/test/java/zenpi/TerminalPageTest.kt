package zenpi

import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.pairing.*

class TerminalPageTest {
    private val tab = "b".repeat(64)
    private val terminal = RemoteTerminal("project", "session", tab, "1", "live", false, "c".repeat(64))
    private val project = RemoteProject("project", "Workspace", "session", terminal = terminal)
    private val projection = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false, "0",
        subtabs = RemoteSubtabs("project", "1", tab, listOf(RemoteSubtab(tab, "Main", "main", 1)), false))
    private fun page(offset: Int = 0, text: String = "abc", total: Int = 6) = JsonObject().apply {
        addProperty("project_id", "project"); addProperty("session_id", "session"); addProperty("terminal_id", terminal.terminalId)
        addProperty("subtab_id", tab); addProperty("subtab_revision", 1); addProperty("digest", "d".repeat(64))
        addProperty("offset", offset); addProperty("next_offset", offset + text.toByteArray().size); addProperty("total_bytes", total)
        addProperty("has_more", offset + text.toByteArray().size < total); addProperty("text", text)
    }
    @Test fun readonlyPagesBindInstanceDigestAndContiguousUtf8WithoutControlAuthority() {
        val first = TerminalPageCodec.decode(page(text = "\u4e2d"), projection, null)
        val request = TerminalPageCodec.request(projection, first)
        FrameCodec.decode(request.toString().toByteArray())
        assertFalse(request.has("required_owner")); assertFalse(request.has("expected_revision"))
        assertEquals("terminal_read", request["payload"].asJsonObject["operation"].asString)
        val args = request["payload"].asJsonObject["arguments"].asJsonObject
        assertEquals(setOf("terminal_id", "subtab_id", "subtab_revision", "offset", "digest"), args.keySet())
        assertEquals(3, args["offset"].asInt)
        val second = TerminalPageCodec.decode(page(3, "def"), projection, first)
        assertFalse(second.hasMore)
        assertEquals(listOf(first), TerminalPageCodec.remember(emptyList(), first, second))
        assertEquals(first, TerminalPageCodec.previous(listOf(first), second))
        assertTrue(runCatching { TerminalPageCodec.previous(listOf(first.copy(terminalId = "e".repeat(64))), second) }.isFailure)
        assertTrue(runCatching { TerminalPageCodec.request(projection, second) }.isFailure)
        val updated = projection.copy(projects = listOf(project.copy(terminal = terminal.copy(text = "new output"))))
        assertTrue(TerminalPageCodec.sameScope(projection, updated))
    }
    @Test fun malformedForeignChangedAndOversizedPagesNeverApply() {
        val first = TerminalPageCodec.decode(page(), projection, null)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("terminal_id", "e".repeat(64)) }, { it.addProperty("session_id", "foreign") },
            { it.addProperty("project_id", "foreign") }, { it.addProperty("subtab_id", "foreign") },
            { it.addProperty("subtab_revision", "1") }, { it.addProperty("subtab_revision", 1.0) },
            { it.addProperty("digest", "e".repeat(64)) }, { it.addProperty("offset", 2) },
            { it.addProperty("total_bytes", 8388609) }, { it.addProperty("has_more", "false") },
            { it.addProperty("text", "\u001bxx") }, { it.addProperty("extra", true) })) {
            assertTrue(runCatching { TerminalPageCodec.decode(page(3, "def").apply(change), projection, first) }.isFailure)
        }
        assertTrue(runCatching { TerminalPageCodec.decode(page(text = "x".repeat(8193), total = 8193), projection, null) }.isFailure)
        assertEquals(8388608, TerminalPageCodec.decode(page(total = 8388608), projection, null).totalBytes)
        assertTrue(runCatching { TerminalPageCodec.remember(listOf(first), first, first.copy(offset = 3, nextOffset = 6, digest = "e".repeat(64))) }.isFailure)
    }
    @Test fun instanceSelectionAndAuthorityLossInvalidateButOrdinaryOutputUpdatesRetainPages() {
        for (changed in listOf(project.copy(terminal = null), project.copy(terminal = terminal.copy(terminalId = "e".repeat(64))),
            project.copy(terminal = terminal.copy(subtabRevision = "2"))))
            assertFalse(TerminalPageCodec.sameScope(projection, projection.copy(projects = listOf(changed))))
        assertFalse(TerminalPageCodec.sameScope(projection, projection.copy(sessionId = "other")))
        val page = TerminalPageCodec.decode(page(), projection, null)
        val state = AuthorityLossPolicy.displayOnly(PairingState(projection = projection, terminalOpen = true, terminalPage = page, terminalPrevious = listOf(page)))
        assertFalse(state.terminalOpen); assertNull(state.terminalPage); assertTrue(state.terminalPrevious.isEmpty())
    }
    @Test fun displayedPageRequiresCompleteCurrentTerminalScopeNotOnlyInstanceId() {
        val first = TerminalPageCodec.decode(page(), projection, null)
        assertTrue(TerminalPageCodec.matchesSelection(first, projection))
        assertTrue(TerminalPageCodec.matchesSelection(first, projection.copy(projects = listOf(project.copy(terminal = terminal.copy(text = "later"))))))
        assertFalse(TerminalPageCodec.matchesSelection(first, null))
        for (foreign in listOf(first.copy(projectId = "other"), first.copy(sessionId = "other"), first.copy(subtabId = "e".repeat(64)),
            first.copy(subtabRevision = "2"), first.copy(terminalId = "e".repeat(64)))) {
            assertFalse(TerminalPageCodec.matchesSelection(foreign, projection))
            assertTrue(runCatching { TerminalPageCodec.request(projection, foreign) }.isFailure)
        }
        assertNull(TerminalPageCodec.terminal(projection.copy(subtabs = projection.subtabs?.copy(projectId = "foreign"))))
        assertNull(TerminalPageCodec.terminal(projection.copy(subtabs = projection.subtabs?.copy(activeId = "e".repeat(64)))))
        assertNull(TerminalPageCodec.terminal(projection.copy(sessionId = "other")))
        assertEquals("0", projection.eventSequence)
    }
    @Test fun retainedTerminalHistoryRejectsBrokenChainsAndKeepsOnlySevenPrecedingPages() {
        fun segment(offset: Int) = TerminalPageCodec.decode(page(offset, "abc", 36), projection,
            if (offset == 0) null else TerminalPage("project", "session", requireNotNull(terminal.terminalId), tab, "1",
                "d".repeat(64), offset - 3, offset, 36, true, "abc"))
        var pages = emptyList<TerminalPage>(); var current = segment(0)
        for (offset in 3..27 step 3) {
            val next = segment(offset); pages = TerminalPageCodec.remember(pages, current, next); current = next
        }
        assertEquals((6..24 step 3).toList(), pages.map { it.offset })
        assertEquals(24, TerminalPageCodec.previous(pages, current)?.offset)
        for (broken in listOf(pages.reversed(), pages.drop(1).let { listOf(pages.first().copy(digest = "e".repeat(64))) + it },
            pages.drop(1).let { listOf(pages.first().copy(totalBytes = 37)) + it })) {
            assertTrue(runCatching { TerminalPageCodec.remember(broken, current, segment(30)) }.isFailure)
            assertTrue(runCatching { TerminalPageCodec.previous(broken, current) }.isFailure)
        }
        assertTrue(runCatching { TerminalPageCodec.remember(emptyList(), null, segment(3)) }.isFailure)
        assertTrue(runCatching { TerminalPageCodec.remember(pages, current, segment(33)) }.isFailure)
    }
    @Test fun malformedPriorPageNeverBecomesAContinuationRequestOrDisplayedSelection() {
        val first = TerminalPageCodec.decode(page(), projection, null)
        for (bad in listOf(first.copy(digest = "invalid"), first.copy(offset = -1), first.copy(nextOffset = 4),
            first.copy(totalBytes = 2), first.copy(totalBytes = 8388609), first.copy(hasMore = false),
            first.copy(text = ""), first.copy(text = "\u001bxx"), first.copy(text = "x".repeat(8193)))) {
            assertFalse(TerminalPageCodec.matchesSelection(bad, projection))
            assertTrue(runCatching { TerminalPageCodec.request(projection, bad) }.isFailure)
            assertTrue(runCatching { TerminalPageCodec.decode(page(3, "def"), projection, bad) }.isFailure)
        }
        assertEquals("0", projection.eventSequence)
    }
    @Test fun identityContinuousButInvalidHistoricalBytesCannotAuthorizeBackOrForwardNavigation() {
        val first = TerminalPageCodec.decode(page(), projection, null)
        val second = TerminalPageCodec.decode(page(3, "def"), projection, first)
        val broken = first.copy(text = "a")
        assertTrue(runCatching { TerminalPageCodec.previous(listOf(broken), second) }.isFailure)
        assertTrue(runCatching { TerminalPageCodec.remember(emptyList(), broken, second) }.isFailure)
        assertTrue(runCatching { TerminalPageCodec.remember(emptyList(), null, broken) }.isFailure)
        val oversizedChain = (0 until 8).map { index -> first.copy(offset = index * 3, nextOffset = index * 3 + 3, totalBytes = 30) }
        val current = first.copy(offset = 24, nextOffset = 27, totalBytes = 30)
        val next = first.copy(offset = 27, nextOffset = 30, totalBytes = 30, hasMore = false)
        assertTrue(runCatching { TerminalPageCodec.previous(oversizedChain, current) }.isFailure)
        assertTrue(runCatching { TerminalPageCodec.remember(oversizedChain, current, next) }.isFailure)
        assertTrue(runCatching { TerminalPageCodec.previous(listOf(first), second.copy(text = "d")) }.isFailure)
    }
}
