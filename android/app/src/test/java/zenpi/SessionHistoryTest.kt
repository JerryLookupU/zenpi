package zenpi

import com.google.gson.*
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.pairing.*

class SessionHistoryTest {
    @Test fun historyTurnIdentityMatchesHostUtf8BoundsWithoutAsciiRewriting() {
        for (id in listOf("history:0/path", "history:\u4e2d", "\u754c".repeat(42) + "ab", " leading trailing ")) {
            val raw = value(count = 1).apply { getAsJsonArray("turns")[0].asJsonObject.addProperty("id", id) }
            val page = SessionHistoryCodec.decode(raw, projection, null)
            assertEquals(id, page.turns.single().id)
            assertTrue(SessionHistoryCodec.validChain(emptyList(), page))
            val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, sessionHistoryOpen = true,
                sessionHistory = page, transcriptTurn = id, transcriptHistoryTurn = page.turns.single())
            val read = requireNotNull(SessionHistoryReadPolicy.projection(state, projection))
            val request = TranscriptPageCodec.request(read, id, null, journalRevision = page.journalRevision)
            val decoded = FrameCodec.decode(FrameCodec.encode(request))
            assertEquals(id, decoded.getAsJsonObject("payload").getAsJsonObject("arguments")["turn_id"].asString)
            assertEquals(projection, state.projection)
        }
        for (id in listOf("", " ", "\t", "bad\nturn", "bad\u007fturn", "a".repeat(129), "\u754c".repeat(43))) {
            val raw = value(count = 1).apply { getAsJsonArray("turns")[0].asJsonObject.addProperty("id", id) }
            assertTrue(runCatching { SessionHistoryCodec.decode(raw, projection, null) }.isFailure)
            val valid = SessionHistoryCodec.decode(value(count = 1), projection, null)
            assertFalse(SessionHistoryCodec.validChain(emptyList(), valid.copy(turns = valid.turns.map { it.copy(id = id) })))
        }
    }
    @Test fun retainedToolSummariesCannotAuthorizeHistoricalReadsAfterMutation() {
        val base = SessionHistoryCodec.decode(value(count = 1), projection, null)
        val call = RemoteToolCall("call", "read_file", "{}", true)
        val turn = base.turns.single().copy(role = "assistant", toolCalls = listOf(call))
        val healthy = base.copy(turns = listOf(turn))
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, sessionHistoryOpen = true,
            sessionHistory = healthy)
        assertTrue(SessionHistoryReadPolicy.navigation(state))
        for (bad in listOf(turn.copy(role = "tool"), turn.copy(toolCallsTruncated = true),
            turn.copy(toolCalls = listOf(call, call)), turn.copy(toolCalls = listOf(call.copy(name = "bad\nname"))),
            turn.copy(toolCalls = listOf(call.copy(argumentsPreview = "x".repeat(513)))))) {
            val page = healthy.copy(turns = listOf(bad))
            val changed = state.copy(sessionHistory = page, transcriptTurn = bad.id, transcriptHistoryTurn = bad)
            assertFalse(SessionHistoryCodec.validChain(emptyList(), page))
            assertFalse(SessionHistoryReadPolicy.navigation(state.copy(sessionHistory = page)))
            assertNull(SessionHistoryReadPolicy.projection(changed, projection))
            assertTrue(runCatching { SessionHistoryCodec.request(projection, page) }.isFailure)
        }
        val bounded = turn.copy(toolCalls = (0..7).map { call.copy(id = "call-$it") }, toolCallsTruncated = true)
        assertTrue(SessionHistoryCodec.validChain(emptyList(), healthy.copy(turns = listOf(bounded))))
    }

    private val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session")), "project", "session", "idle",
        listOf(RemoteTurn("recent", "assistant", "recent")), "80", true, "10")
    private fun value(offset: Int = 0, count: Int = 16, total: Int = 70) = JsonObject().apply {
        addProperty("project_id", "project"); addProperty("session_id", "session"); addProperty("journal_revision", 80)
        addProperty("digest", "a".repeat(64)); addProperty("offset", offset); addProperty("next_offset", offset + count)
        addProperty("total_turns", total); addProperty("has_more", offset + count < total)
        add("turns", JsonArray().apply { repeat(count) { i -> add(JsonObject().apply {
            addProperty("id", "old-${offset + i}"); addProperty("role", "user"); addProperty("content", "historical text")
            addProperty("content_truncated", false)
        }) } })
    }
    @Test fun historyPagesBindJournalAndReadOldMessagesWithoutChangingLiveProjection() {
        val first = SessionHistoryCodec.decode(value(), projection, null)
        val request = SessionHistoryCodec.request(projection, first)
        FrameCodec.decode(request.toString().toByteArray())
        assertFalse(request.has("required_owner")); assertFalse(request.has("expected_revision"))
        assertEquals("16", request.getAsJsonObject("payload").getAsJsonObject("arguments")["offset"].toString())
        val second = SessionHistoryCodec.decode(value(16), projection, first)
        assertEquals(listOf(first), SessionHistoryCodec.remember(emptyList(), first, second))
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, sessionHistoryOpen = true, sessionHistory = first)
        assertTrue(SessionHistoryReadPolicy.navigation(state))
        val selected = state.copy(transcriptTurn = "old-0", transcriptHistoryTurn = first.turns.first())
        val readProjection = requireNotNull(SessionHistoryReadPolicy.projection(selected, projection))
        val turnRequest = TranscriptPageCodec.request(readProjection, "old-0", null)
        assertFalse(turnRequest.getAsJsonObject("payload").getAsJsonObject("arguments").has("view"))
        val message = TranscriptPage("project", "session", "old-0", "user", "b".repeat(64), 0, 5, 5, false, "older", journalRevision = "80")
        assertTrue(TranscriptReadPolicy.matches(selected, message))
        assertFalse(TranscriptReadPolicy.matches(selected.copy(projection = projection.copy(journalSequence = "81")), message))
        assertFalse(TranscriptReadPolicy.matches(selected, message.copy(journalRevision = null)))
        assertEquals(listOf("recent"), projection.turns.map { it.id })
        assertNull(SessionHistoryReadPolicy.projection(selected, projection.copy(journalSequence = "81")))
        assertNull(SessionHistoryReadPolicy.projection(selected.copy(sessionHistoryOpen = false), projection))
        assertNull(SessionHistoryReadPolicy.projection(selected.copy(transcriptHistoryTurn = RemoteTurn("foreign", "user", "x")), projection))
        val cleared = AuthorityLossPolicy.displayOnly(selected)
        assertFalse(cleared.sessionHistoryOpen); assertNull(cleared.sessionHistory); assertNull(cleared.transcriptTurn)
        assertFalse(SessionHistoryReadPolicy.navigation(state.copy(phase = PairingPhase.OFFLINE)))
        assertFalse(SessionHistoryReadPolicy.navigation(state.copy(scanInProgress = true)))
    }
    @Test fun retainedHistoryIsBoundedAndCannotMixScopeDigestRevisionOrRepeatedTurns() {
        fun page(offset: Int) = SessionHistoryPage("project", "session", "80", "a".repeat(64), offset, offset + 1,
            12, offset + 1 < 12, listOf(RemoteTurn("old-$offset", "user", "text")))
        var history = emptyList<SessionHistoryPage>(); var current = page(0)
        for (offset in 1..9) {
            val next = page(offset); history = SessionHistoryCodec.remember(history, current, next); current = next
        }
        assertEquals((2..8).toList(), history.map { it.offset })
        for (next in listOf(page(11), page(10).copy(sessionId = "foreign"), page(10).copy(journalRevision = "81"),
            page(10).copy(digest = "b".repeat(64)), page(10).copy(turns = history.first().turns))) {
            assertTrue(runCatching { SessionHistoryCodec.remember(history, current, next) }.isFailure)
        }
        assertTrue(runCatching { SessionHistoryCodec.remember(history.reversed(), current, page(10)) }.isFailure)
    }
    @Test fun malformedScopeSizesCountsAndChangedJournalNeverBecomeHistory() {
        val first = SessionHistoryCodec.decode(value(), projection, null)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("project_id", "foreign") }, { it.addProperty("session_id", "foreign") },
            { it.addProperty("journal_revision", 81) }, { it.addProperty("journal_revision", "80") },
            { it.addProperty("digest", "b".repeat(64)) }, { it.addProperty("offset", 0) },
            { it.addProperty("total_turns", 71) }, { it.addProperty("has_more", false) },
            { it.addProperty("path", "private") },
            { it.getAsJsonArray("turns")[0].asJsonObject.addProperty("id", "old-0") },
            { it.getAsJsonArray("turns")[0].asJsonObject.addProperty("content", "x".repeat(2049)) })) {
            assertTrue(runCatching { SessionHistoryCodec.decode(value(16).apply(change), projection, first) }.isFailure)
        }
        assertTrue(runCatching { SessionHistoryCodec.decode(value(count = 17), projection, null) }.isFailure)
        assertTrue(runCatching { SessionHistoryCodec.request(projection.copy(journalSequence = "81"), first) }.isFailure)
        assertTrue(SessionHistoryCodec.decode(value(count = 0, total = 0), projection, null).turns.isEmpty())
        val emptyNext = value(16, 0)
        assertTrue(runCatching { SessionHistoryCodec.decode(emptyNext, projection, first) }.isFailure)
        assertTrue(runCatching { SessionHistoryCodec.remember(emptyList(), first, first) }.isFailure)
    }
    @Test fun corruptedRetainedPagesCannotAuthorizeBackNavigationOrHistoricalMessageReads() {
        val first = SessionHistoryCodec.decode(value(), projection, null)
        val second = SessionHistoryCodec.decode(value(16), projection, first)
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, sessionHistoryOpen = true,
            sessionHistory = second, sessionHistoryPrevious = listOf(first))
        assertTrue(SessionHistoryReadPolicy.navigation(state))
        for (bad in listOf(first.copy(digest = "bad"), first.copy(journalRevision = "18446744073709551616"),
            first.copy(nextOffset = 15), first.copy(totalTurns = 16385), first.copy(hasMore = false),
            first.copy(turns = first.turns + first.turns.first()), first.copy(turns = first.turns.map { it.copy(content = "x".repeat(2049)) }),
            first.copy(sessionId = "foreign"), first.copy(turns = second.turns))) {
            val corrupt = state.copy(sessionHistoryPrevious = listOf(bad))
            assertFalse(SessionHistoryReadPolicy.navigation(corrupt))
            assertNull(SessionHistoryReadPolicy.projection(corrupt.copy(transcriptHistoryTurn = second.turns.first()), projection))
            assertTrue(runCatching { SessionHistoryCodec.remember(listOf(bad), second,
                SessionHistoryCodec.decode(value(32), projection, second)) }.isFailure)
        }
        val emptyTerminal = value(16, 0, 16)
        assertTrue(runCatching { SessionHistoryCodec.decode(emptyTerminal, projection, first) }.isFailure)
    }
    @Test fun historicalMessagePagesRequireExactNumericRevisionOnEveryRequestAndReceipt() {
        val live = projection.copy(turns = listOf(RemoteTurn("old", "user", "preview", true)))
        fun receipt(offset: Int = 0, text: String = "first", more: Boolean = true) = JsonObject().apply {
            addProperty("project_id", "project"); addProperty("session_id", "session"); addProperty("turn_id", "old")
            addProperty("role", "user"); addProperty("digest", "b".repeat(64)); addProperty("offset", offset)
            addProperty("next_offset", offset + text.length); addProperty("total_bytes", 10); addProperty("has_more", more)
            addProperty("text", text); addProperty("journal_revision", 80)
        }
        val request = TranscriptPageCodec.request(live, "old", null, journalRevision = "80")
        FrameCodec.decode(request.toString().toByteArray())
        assertEquals("80", request.getAsJsonObject("payload").getAsJsonObject("arguments")["journal_revision"].toString())
        val first = TranscriptPageCodec.decode(receipt(), live, "old", null, journalRevision = "80")
        val last = TranscriptPageCodec.decode(receipt(5, "later", false), live, "old", first, journalRevision = "80")
        assertEquals("80", last.journalRevision)
        assertEquals(listOf(first), TranscriptNavigation.remember(emptyList(), first, last))
        for (change in listOf<(JsonObject) -> Unit>({ it.remove("journal_revision") }, { it.addProperty("journal_revision", "80") },
            { it.addProperty("journal_revision", 81) }, { it.addProperty("journal_revision", -1) })) {
            assertTrue(runCatching { TranscriptPageCodec.decode(receipt().apply(change), live, "old", null, journalRevision = "80") }.isFailure)
        }
        assertTrue(runCatching { TranscriptPageCodec.decode(receipt(), live, "old", null) }.isFailure)
        assertTrue(runCatching { TranscriptPageCodec.request(live, "old", first) }.isFailure)
        assertTrue(runCatching { TranscriptPageCodec.request(live.copy(journalSequence = "81"), "old", first, journalRevision = "80") }.isFailure)
        assertTrue(runCatching { TranscriptNavigation.remember(emptyList(), first, last.copy(journalRevision = null)) }.isFailure)
        val call = RemoteToolCall("call", "read_file", "{}", true)
        val assistant = live.copy(turns = listOf(RemoteTurn("old", "assistant", "preview", true, listOf(call))))
        val toolRequest = TranscriptPageCodec.request(assistant, "old", null, toolCall = call, journalRevision = "80")
        assertEquals("80", toolRequest.getAsJsonObject("payload").getAsJsonObject("arguments")["journal_revision"].toString())
        val toolReceipt = receipt().apply {
            addProperty("role", "assistant"); addProperty("tool_call_id", call.id); addProperty("tool_name", call.name)
        }
        assertEquals("80", TranscriptPageCodec.decode(toolReceipt, assistant, "old", null, toolCall = call, journalRevision = "80").journalRevision)
        assertTrue(runCatching { TranscriptPageCodec.decode(toolReceipt.deepCopy().apply { remove("journal_revision") },
            assistant, "old", null, toolCall = call, journalRevision = "80") }.isFailure)
    }
    @Test fun dismissingHistoricalMessageReturnsOnlyToItsValidConversationPage() {
        val page = SessionHistoryCodec.decode(value(), projection, null)
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, sessionHistoryOpen = true,
            sessionHistory = page, transcriptTurn = page.turns.first().id, transcriptHistoryTurn = page.turns.first(),
            transcriptError = "Read failed")
        val returned = SessionHistoryReadPolicy.dismissMessage(state)
        assertEquals(page, returned.sessionHistory); assertTrue(returned.sessionHistoryOpen)
        assertNull(returned.transcriptTurn); assertNull(returned.transcriptError)
        assertTrue(SessionHistoryReadPolicy.navigation(returned))
        for (bad in listOf(state.copy(projection = projection.copy(journalSequence = "81")), state.copy(transcriptTurn = "foreign"),
            state.copy(transcriptHistoryTurn = page.turns.first().copy(content = "replaced")), state.copy(sessionHistoryError = "changed"),
            state.copy(transcriptToolCall = RemoteToolCall("foreign", "read_file", "{}", false)))) {
            val cleared = SessionHistoryReadPolicy.dismissMessage(bad)
            assertFalse(cleared.sessionHistoryOpen); assertNull(cleared.sessionHistory); assertNull(cleared.transcriptTurn)
        }
    }
}
