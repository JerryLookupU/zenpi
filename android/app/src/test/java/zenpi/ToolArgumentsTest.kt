package zenpi

import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.pairing.*

class ToolArgumentsTest {
    private val tab = "b".repeat(64)
    private val call = RemoteToolCall("call-a", "read_file", "preview", true)
    private val turn = RemoteTurn("invocation", "assistant", "", toolCalls = listOf(call))
    private val projection = HostProjection(listOf(RemoteProject("project", "Project", "session", workerStatuses = listOf(
        RemoteWorkerStatus("task", "project", "session", tab, "succeeded")))), "project", "session", "idle", emptyList(), "1", true, "1")
    private fun page(offset: Int = 0) = JsonObject().apply {
        addProperty("project_id", "project"); addProperty("session_id", "session"); addProperty("request_id", "task"); addProperty("subtab_id", tab)
        addProperty("turn_id", turn.id); addProperty("role", "assistant"); addProperty("tool_call_id", call.id); addProperty("tool_name", call.name)
        addProperty("digest", "a".repeat(64)); addProperty("offset", offset); addProperty("next_offset", offset + 3)
        addProperty("total_bytes", 6); addProperty("has_more", offset == 0); addProperty("text", if (offset == 0) "abc" else "def")
    }
    @Test fun argumentsRequestAndContinuationRetainTaskMessageAndCall() {
        val first = TranscriptPageCodec.decode(page(), projection, "task", null, true, turn, call)
        val request = TranscriptPageCodec.request(projection, "task", first, true, turn, call)
        FrameCodec.decode(request.toString().toByteArray())
        val args = request["payload"].asJsonObject["arguments"].asJsonObject
        assertEquals("task", args["request_id"].asString); assertEquals(turn.id, args["turn_id"].asString); assertEquals(call.id, args["tool_call_id"].asString)
        assertFalse(request.has("expected_revision")); assertFalse(request.has("required_owner"))
        val second = TranscriptPageCodec.decode(page(3), projection, "task", first, true, turn, call)
        assertEquals(call.id, second.toolCallId); assertFalse(second.hasMore)
        assertEquals(listOf(first), TranscriptNavigation.remember(emptyList(), first, second))
        assertEquals(first, TranscriptNavigation.previous(listOf(first), second))
        assertTrue(runCatching { TranscriptPageCodec.request(projection, "task", first, true, turn) }.isFailure)
        assertTrue(runCatching { TranscriptPageCodec.decode(page(3), projection, "task", first, true, turn) }.isFailure)
        assertTrue(runCatching { TranscriptNavigation.previous(listOf(first.copy(toolCallId = "foreign")), second) }.isFailure)
    }
    @Test fun malformedOrForeignToolPagesFailClosed() {
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("tool_call_id", "foreign") }, { it.addProperty("tool_name", "write_file") }, { it.remove("tool_call_id") },
            { it.addProperty("request_id", "foreign") }, { it.addProperty("turn_id", "foreign") }, { it.addProperty("subtab_id", "foreign") },
            { it.addProperty("total_bytes", 65537) }, { it.addProperty("text", "a\u0000c") }, { it.addProperty("execution", "private") })) {
            assertTrue(runCatching { TranscriptPageCodec.decode(page().apply(change), projection, "task", null, true, turn, call) }.isFailure)
        }
        for (changed in listOf(turn.copy(role = "tool"), turn.copy(toolCalls = emptyList()), turn.copy(toolCalls = listOf(call, call)))) {
            assertTrue(runCatching { TranscriptPageCodec.request(projection, "task", null, true, changed, call) }.isFailure)
            assertTrue(runCatching { TranscriptPageCodec.decode(page(), projection, "task", null, true, changed, call) }.isFailure)
        }
        assertTrue(runCatching { TranscriptPageCodec.request(projection, "task", null, false, turn, call) }.isFailure)
    }
    @Test fun dismissingToolArgumentsReturnsToValidatedHistoryAndAuthorityLossClearsIt() {
        val history = WorkerHistoryPage("project", "session", "task", tab, "c".repeat(64), 0, 1, 1, false, listOf(turn))
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, workerHistoryRequest = "task", workerHistoryPage = history,
            transcriptTurn = "task", transcriptWorker = true, transcriptHistoryTurn = turn, transcriptToolCall = call,
            transcriptPage = TranscriptPageCodec.decode(page(), projection, "task", null, true, turn, call))
        val restored = WorkerHistoryNavigation.dismissMessage(state)
        assertEquals(history, restored.workerHistoryPage); assertNull(restored.transcriptToolCall); assertNull(restored.transcriptPage)
        assertNull(WorkerHistoryNavigation.dismissMessage(state.copy(transcriptToolCall = call.copy(id = "foreign"))).workerHistoryPage)
        val fenced = AuthorityLossPolicy.displayOnly(state)
        assertNull(fenced.transcriptToolCall); assertNull(fenced.workerHistoryPage); assertNull(fenced.transcriptPage)
    }
}
