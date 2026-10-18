package zenpi

import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class OrdinaryToolArgumentsTest {
    private val call = RemoteToolCall("call", "read_file", "preview", true)
    private val turn = RemoteTurn("turn", "assistant", "", toolCalls = listOf(call))
    private val projection = HostProjection(listOf(RemoteProject("project", "Project", "session")),
        "project", "session", "idle", listOf(turn), "1", false, "1")
    private fun page(offset: Int = 0, text: String = "abc", total: Int = 6) = JsonObject().apply {
        addProperty("project_id", "project"); addProperty("session_id", "session"); addProperty("turn_id", "turn"); addProperty("role", "assistant")
        addProperty("tool_call_id", "call"); addProperty("tool_name", "read_file"); addProperty("digest", "a".repeat(64))
        addProperty("offset", offset); addProperty("next_offset", offset + text.toByteArray(Charsets.UTF_8).size)
        addProperty("total_bytes", total); addProperty("has_more", offset + text.toByteArray(Charsets.UTF_8).size < total); addProperty("text", text)
    }
    @Test fun ordinaryArgumentsPageWithoutWorkerBindingsAndNavigateWithinTheSameCall() {
        val first = TranscriptPageCodec.decode(page(), projection, "turn", null, toolCall = call)
        val request = TranscriptPageCodec.request(projection, "turn", first, toolCall = call)
        FrameCodec.decode(request.toString().toByteArray())
        assertFalse(request.has("required_owner")); assertFalse(request.has("expected_revision"))
        val args = request["payload"].asJsonObject["arguments"].asJsonObject
        assertEquals(setOf("turn_id", "tool_call_id", "offset", "digest"), args.keySet())
        assertEquals("call", args["tool_call_id"].asString); assertEquals(3, args["offset"].asInt)
        val second = TranscriptPageCodec.decode(page(3, "def"), projection, "turn", first, toolCall = call)
        assertFalse(second.hasMore); assertNull(second.workerRequestId); assertNull(second.subtabId)
        assertEquals(first, TranscriptNavigation.previous(TranscriptNavigation.remember(emptyList(), first, second), second))
        assertTrue(runCatching { TranscriptPageCodec.request(projection, "turn", first) }.isFailure)
        assertTrue(runCatching { TranscriptPageCodec.decode(page(), projection, "turn", null) }.isFailure)
    }
    @Test fun foreignCallsNamesRolesWorkerFieldsAndOversizedArgumentsAreRejected() {
        val first = TranscriptPageCodec.decode(page(), projection, "turn", null, toolCall = call)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("tool_call_id", "foreign") }, { it.addProperty("tool_name", "write_file") },
            { it.remove("tool_call_id") }, { it.addProperty("turn_id", "foreign") }, { it.addProperty("role", "tool") },
            { it.addProperty("project_id", "foreign") }, { it.addProperty("session_id", "foreign") },
            { it.addProperty("request_id", "worker") }, { it.addProperty("subtab_id", "b".repeat(64)) },
            { it.addProperty("digest", "b".repeat(64)) }, { it.addProperty("total_bytes", 65537) })) {
            assertTrue(runCatching { TranscriptPageCodec.decode(page(3, "def").apply(change), projection, "turn", first, toolCall = call) }.isFailure)
        }
        for (changed in listOf(turn.copy(role = "user"), turn.copy(toolCalls = emptyList()),
            turn.copy(toolCalls = listOf(call.copy(name = "write_file"))), turn.copy(toolCalls = listOf(call, call)))) {
            val next = projection.copy(turns = listOf(changed))
            assertFalse(TranscriptPageCodec.retainsTurn(projection, next, "turn"))
            assertTrue(runCatching { TranscriptPageCodec.request(next, "turn", null, toolCall = call) }.isFailure)
            assertTrue(runCatching { TranscriptPageCodec.decode(page(), next, "turn", null, toolCall = call) }.isFailure)
        }
        assertEquals(65536, TranscriptPageCodec.decode(page(total = 65536), projection, "turn", null, toolCall = call).totalBytes)
    }
    @Test fun authorityLossDiscardsTransientArgumentPagesAndSelection() {
        val page = TranscriptPageCodec.decode(page(), projection, "turn", null, toolCall = call)
        val fenced = zenpi.pairing.AuthorityLossPolicy.displayOnly(zenpi.pairing.PairingState(
            projection = projection, transcriptTurn = "turn", transcriptToolCall = call, transcriptPage = page))
        assertNull(fenced.transcriptTurn); assertNull(fenced.transcriptToolCall); assertNull(fenced.transcriptPage)
        assertTrue(fenced.transcriptPreviousPages.isEmpty())
    }
}
