package zenpi

import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class TranscriptPageTest {
    private val projection = HostProjection(listOf(RemoteProject("project", "Project", "session")), "project", "session", "idle",
        listOf(RemoteTurn("turn", "assistant", "prefix", true)), "1", true, "1")
    private fun page(offset: Int = 0, text: String = "abc", total: Int = 6) = JsonObject().apply {
        addProperty("project_id", "project"); addProperty("session_id", "session"); addProperty("turn_id", "turn"); addProperty("role", "assistant")
        addProperty("digest", "a".repeat(64)); addProperty("offset", offset); addProperty("next_offset", offset + text.toByteArray().size)
        addProperty("total_bytes", total); addProperty("has_more", offset + text.toByteArray().size < total); addProperty("text", text)
    }
    @Test fun contiguousPagesHaveExactScopeDigestByteOffsetsAndReadOnlyRequest() {
        val first = TranscriptPageCodec.decode(page(), projection, "turn", null)
        val second = TranscriptPageCodec.decode(page(3, "def"), projection, "turn", first)
        assertFalse(second.hasMore); assertEquals(6, second.nextOffset)
        val request = TranscriptPageCodec.request(projection, "turn", first)
        FrameCodec.decode(request.toString().toByteArray())
        assertFalse(request.has("required_owner")); assertFalse(request.has("expected_revision"))
        assertEquals(3, request["payload"].asJsonObject["arguments"].asJsonObject["offset"].asInt)
        assertEquals("1", projection.eventSequence)
        assertTrue(runCatching { TranscriptPageCodec.request(projection, "turn", second) }.isFailure)
    }
    @Test fun badScopeDigestOffsetsSizesAndFlagsNeverBecomeAPage() {
        val first = TranscriptPageCodec.decode(page(), projection, "turn", null)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("project_id", "foreign") }, { it.addProperty("session_id", "foreign") }, { it.addProperty("turn_id", "foreign") }, { it.addProperty("role", "user") },
            { it.addProperty("digest", "b".repeat(64)) }, { it.addProperty("total_bytes", 7) }, { it.addProperty("offset", 2) },
            { it.addProperty("next_offset", 5) }, { it.addProperty("has_more", true) }, { it.addProperty("has_more", "false") },
            { it.addProperty("offset", "3") }, { it.addProperty("text", "x".repeat(8193)) }, { it.addProperty("extra", true) })) {
            assertTrue(runCatching { TranscriptPageCodec.decode(page(3, "def").apply(change), projection, "turn", first) }.isFailure)
        }
    }
    @Test fun utf8BytesNotCharactersAndEmptyFinalContentAreValid() {
        val value = TranscriptPageCodec.decode(page(text = "\u4e2d", total = 3), projection, "turn", null)
        assertEquals(3, value.nextOffset); assertFalse(value.hasMore)
        assertFalse(TranscriptPageCodec.decode(page(text = "", total = 0), projection, "turn", null).hasMore)
        assertTrue(runCatching { TranscriptPageCodec.decode(page(text = "", total = 1), projection, "turn", null) }.isFailure)
    }
    @Test fun exactPageAndContentCapsAreAcceptedButOneMoreIsRefused() {
        val maximum = 4 * 1024 * 1024
        val value = page(text = "x".repeat(8192), total = maximum)
        val first = TranscriptPageCodec.decode(value, projection, "turn", null)
        assertEquals(8192, first.nextOffset); assertTrue(first.hasMore); assertEquals(maximum, first.totalBytes)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("total_bytes", maximum + 1) },
            { it.addProperty("text", "x".repeat(8193)); it.addProperty("next_offset", 8193) },
            { it.addProperty("offset", -1) }, { it.addProperty("next_offset", 8192.0) })) {
            assertTrue(runCatching { TranscriptPageCodec.decode(value.deepCopy().apply(change), projection, "turn", null) }.isFailure)
        }
        val prior = first.copy(offset = maximum - 8192, nextOffset = maximum, hasMore = false)
        assertTrue(runCatching { TranscriptPageCodec.request(projection, "turn", prior) }.isFailure)
    }
    @Test fun pageIsInvalidatedWhenSnapshotReplacesItsTurnOrScope() {
        for (next in listOf(null, projection.copy(selectedId = "other"), projection.copy(sessionId = "other"),
            projection.copy(turns = emptyList()), projection.copy(turns = listOf(RemoteTurn("other", "assistant", "prefix", true))),
            projection.copy(turns = listOf(RemoteTurn("turn", "user", "prefix", true))),
            projection.copy(turns = listOf(RemoteTurn("turn", "assistant", "changed", true))),
            projection.copy(turns = listOf(RemoteTurn("turn", "assistant", "prefix", false))),
            projection.copy(turns = projection.turns + projection.turns))) {
            assertFalse(TranscriptPageCodec.retainsTurn(projection, next, "turn"))
        }
        assertFalse(TranscriptPageCodec.retainsTurn(projection.copy(turns = emptyList()), projection, "turn"))
    }
    @Test fun appendAndStatusRefreshRetainAnUnchangedPageWithoutRetainingRemovedMessages() {
        val next = projection.copy(turns = projection.turns + RemoteTurn("new", "assistant", "new answer"),
            phase = "running", journalSequence = "5", eventSequence = "2", replayedEventCount = 1)
        assertTrue(TranscriptPageCodec.retainsTurn(projection, next, "turn"))
        assertTrue(TranscriptPageCodec.retainsTurn(next, projection, "turn"))
        assertFalse(TranscriptPageCodec.retainsTurn(next, projection, "new"))
    }
    @Test fun malformedPriorPageCannotCreateOrValidateAContinuation() {
        val first = TranscriptPageCodec.decode(page(), projection, "turn", null)
        for (bad in listOf(first.copy(subtabId = "foreign"), first.copy(role = "user"), first.copy(digest = "bad"),
            first.copy(offset = -1), first.copy(text = "different"), first.copy(nextOffset = 2))) {
            assertTrue(runCatching { TranscriptPageCodec.request(projection, "turn", bad) }.isFailure)
            assertTrue(runCatching { TranscriptPageCodec.decode(page(3, "def"), projection, "turn", bad) }.isFailure)
        }
    }
}
