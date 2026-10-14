package zenpi

import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class WorkerHistoryTest {
    @Test fun opaqueWorkerTurnIdsRemainReadableWithoutLosingTaskOrContinuationIdentity() {
        for (id in listOf("worker:input/\u4e2d", "\u754c".repeat(42) + "ab", " leading trailing ")) {
            val history = WorkerHistoryCodec.decode(page().apply {
                getAsJsonArray("turns")[0].asJsonObject.addProperty("id", id)
            }, projection, "task", null)
            val turn = history.turns.single()
            val request = TranscriptPageCodec.request(projection, "task", null, true, turn)
            assertEquals(id, FrameCodec.decode(FrameCodec.encode(request)).getAsJsonObject("payload")
                .getAsJsonObject("arguments")["turn_id"].asString)
            fun receipt(offset: Int = 0, text: String = "abc") = JsonObject().apply {
                addProperty("project_id", "project"); addProperty("session_id", "session")
                addProperty("request_id", "task"); addProperty("subtab_id", tab); addProperty("turn_id", id)
                addProperty("role", "user"); addProperty("digest", "a".repeat(64)); addProperty("offset", offset)
                addProperty("next_offset", offset + 3); addProperty("total_bytes", 6)
                addProperty("has_more", offset == 0); addProperty("text", text)
            }
            val first = TranscriptPageCodec.decode(receipt(), projection, "task", null, true, turn)
            val last = TranscriptPageCodec.decode(receipt(3, "def"), projection, "task", first, true, turn)
            assertEquals(id, first.turnId); assertEquals(id, last.turnId)
            assertEquals(listOf(first), TranscriptNavigation.remember(emptyList(), first, last))
            for (field in listOf("turn_id", "request_id", "subtab_id")) {
                assertTrue(runCatching { TranscriptPageCodec.decode(receipt(3, "def").apply { addProperty(field, "foreign") },
                    projection, "task", first, true, turn) }.isFailure)
            }
        }
        for (id in listOf("", " ", "bad\nturn", "bad\u007fturn", "a".repeat(129), "\u754c".repeat(43))) {
            val turn = RemoteTurn(id, "user", "preview")
            assertTrue(runCatching { TranscriptPageCodec.request(projection, "task", null, true, turn) }.isFailure)
        }
    }
    @Test fun retainedMalformedHistoryNeverAuthorizesContinuationOrDecode() {
        val first = WorkerHistoryCodec.decode(toolPage(), projection, "task", null)
        val turn = first.turns.single(); val call = turn.toolCalls.single()
        val next = page().apply {
            addProperty("offset", 1); addProperty("next_offset", 2); addProperty("has_more", false)
            getAsJsonArray("turns")[0].asJsonObject.addProperty("id", "answer")
        }
        val malformedTurns = listOf(turn.copy(role = "user"), turn.copy(toolCallsTruncated = true),
            turn.copy(toolCalls = listOf(call, call)), turn.copy(toolCalls = listOf(call.copy(id = ""))),
            turn.copy(toolCalls = listOf(call.copy(name = "bad\nname"))),
            turn.copy(toolCalls = listOf(call.copy(argumentsPreview = "x".repeat(513)))),
            turn.copy(toolCalls = listOf(call.copy(argumentsPreview = "bad\u0000"))))
        val malformedPages = listOf(first.copy(digest = "bad"), first.copy(offset = -1), first.copy(nextOffset = 2),
            first.copy(totalTurns = 1025), first.copy(turns = emptyList())) + malformedTurns.map { first.copy(turns = listOf(it)) }
        for (bad in malformedPages) {
            assertFalse(WorkerHistoryCodec.validPage(bad))
            assertTrue(runCatching { WorkerHistoryCodec.request(projection, "task", bad) }.isFailure)
            assertTrue(runCatching { WorkerHistoryCodec.decode(next, projection, "task", bad) }.isFailure)
            val state = zenpi.pairing.PairingState(phase = zenpi.pairing.PairingPhase.CONNECTED, projection = projection,
                workerHistoryRequest = "task", workerHistoryPage = bad)
            assertFalse(zenpi.pairing.WorkerHistoryNavigation.navigation(state))
            assertTrue(runCatching { zenpi.pairing.WorkerHistoryNavigation.remember(emptyList(), null, bad) }.isFailure)
        }
        assertFalse(WorkerHistoryCodec.decode(next, projection, "task", first).hasMore)
        assertTrue(runCatching { WorkerHistoryCodec.decode(next.deepCopy().apply {
            getAsJsonArray("turns")[0].asJsonObject.addProperty("id", turn.id)
        }, projection, "task", first) }.isFailure)
        assertTrue(runCatching { WorkerHistoryCodec.decode(next.deepCopy().apply {
            addProperty("offset", 2); addProperty("next_offset", 2); add("turns", com.google.gson.JsonArray())
        }, projection, "task", null) }.isFailure)
    }

    private val tab = "b".repeat(64)
    private val projection = HostProjection(listOf(RemoteProject("project", "Project", "session", workerStatuses = listOf(
        RemoteWorkerStatus("task", "project", "session", tab, "succeeded")))), "project", "session", "idle", emptyList(), "1", true, "1")
    private fun page() = JsonObject().apply {
        addProperty("project_id", "project"); addProperty("session_id", "session"); addProperty("request_id", "task"); addProperty("subtab_id", tab)
        addProperty("digest", "a".repeat(64)); addProperty("offset", 0); addProperty("next_offset", 1); addProperty("total_turns", 2); addProperty("has_more", true)
        add("turns", com.google.gson.JsonArray().apply { add(JsonObject().apply {
            addProperty("id", "input"); addProperty("role", "user"); addProperty("content", "message"); addProperty("content_truncated", true)
        }) })
    }
    @Test fun historyRequestIsReadonlyAndIndividualMessagesRetainTheirRoleAndTaskScope() {
        val history = WorkerHistoryCodec.decode(page(), projection, "task", null)
        assertEquals("user", history.turns.single().role)
        val request = WorkerHistoryCodec.request(projection, "task", history)
        FrameCodec.decode(request.toString().toByteArray())
        assertFalse(request.has("required_owner")); assertFalse(request.has("expected_revision"))
        assertEquals("history", request["payload"].asJsonObject["arguments"].asJsonObject["view"].asString)
        val read = TranscriptPageCodec.request(projection, "task", null, true, history.turns.single())
        assertEquals("input", read["payload"].asJsonObject["arguments"].asJsonObject["turn_id"].asString)
        val result = JsonObject().apply {
            addProperty("project_id", "project"); addProperty("session_id", "session"); addProperty("request_id", "task"); addProperty("subtab_id", tab)
            addProperty("turn_id", "input"); addProperty("role", "user"); addProperty("digest", "a".repeat(64)); addProperty("offset", 0)
            addProperty("next_offset", 3); addProperty("total_bytes", 3); addProperty("has_more", false); addProperty("text", "abc")
        }
        assertEquals("user", TranscriptPageCodec.decode(result, projection, "task", null, true, history.turns.single()).role)
        assertTrue(runCatching { TranscriptPageCodec.decode(result, projection, "task", null, true) }.isFailure)
    }
    @Test fun malformedHistoryAndCrossScopeContinuationsFailClosed() {
        val first = WorkerHistoryCodec.decode(page(), projection, "task", null)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("request_id", "foreign") }, { it.addProperty("subtab_id", "c".repeat(64)) },
            { it.addProperty("offset", 1) }, { it.addProperty("next_offset", 2) }, { it.addProperty("total_turns", 1025) },
            { it.addProperty("has_more", "true") }, { it.addProperty("digest", "bad") }, { it.addProperty("extra", true) },
            { it["turns"].asJsonArray[0].asJsonObject.addProperty("content", "x".repeat(2049)) },
            { it["turns"].asJsonArray[0].asJsonObject.addProperty("role", "foreign") })) {
            assertTrue(runCatching { WorkerHistoryCodec.decode(page().apply(change), projection, "task", null) }.isFailure)
        }
        assertTrue(runCatching { WorkerHistoryCodec.request(projection, "task", first.copy(sessionId = "foreign")) }.isFailure)
        assertTrue(runCatching { WorkerHistoryCodec.request(projection, "task", first.copy(subtabId = "c".repeat(64))) }.isFailure)
        val next = page().apply { addProperty("offset", 1); addProperty("next_offset", 2); addProperty("has_more", false)
            getAsJsonArray("turns")[0].asJsonObject.addProperty("id", "answer") }
        assertFalse(WorkerHistoryCodec.decode(next, projection, "task", first).hasMore)
        next.addProperty("digest", "c".repeat(64))
        assertTrue(runCatching { WorkerHistoryCodec.decode(next, projection, "task", first) }.isFailure)
    }
    private fun toolPage() = page().apply {
        this["turns"].asJsonArray[0].asJsonObject.apply {
            addProperty("role", "assistant"); addProperty("content", "")
            addProperty("tool_calls_truncated", false)
            add("tool_calls", com.google.gson.JsonArray().apply { add(JsonObject().apply {
                addProperty("id", "read-file"); addProperty("name", "read_file")
                addProperty("arguments_preview", "{\"path\":\"evidence.txt\",\"api_key\":\"<redacted>\"}")
                addProperty("arguments_truncated", false)
            }) })
        }
    }
    @Test fun toolMetadataIsStrictBoundedAndNeverAcceptsExecutionEvidence() {
        val turn = WorkerHistoryCodec.decode(toolPage(), projection, "task", null).turns.single()
        assertEquals("read_file", turn.toolCalls.single().name)
        assertTrue(turn.toolCalls.single().argumentsPreview.contains("evidence.txt"))
        assertFalse(turn.toolCallsTruncated)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("name", "x".repeat(65)) }, { it.addProperty("id", "x".repeat(129)) },
            { it.addProperty("name", "bad\nname") }, { it.addProperty("arguments_preview", "\u4e2d".repeat(171)) },
            { it.addProperty("arguments_preview", "bad\u0000value") }, { it.addProperty("arguments_truncated", "true") },
            { it.addProperty("execution", "private") }, { it.remove("arguments_preview") })) {
            val malformed = toolPage().apply { change(this["turns"].asJsonArray[0].asJsonObject["tool_calls"].asJsonArray[0].asJsonObject) }
            assertTrue(runCatching { WorkerHistoryCodec.decode(malformed, projection, "task", null) }.isFailure)
        }
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("role", "tool") }, { it.remove("tool_calls_truncated") }, { it.addProperty("tool_calls_truncated", true) },
            { it.addProperty("tool_calls", "bad") }, { it.addProperty("tool_calls_truncated", "false") },
            { it["tool_calls"].asJsonArray.add(it["tool_calls"].asJsonArray[0].deepCopy()) },
            { val calls = it["tool_calls"].asJsonArray; repeat(8) { calls.add(calls[0].deepCopy()) } })) {
            val malformed = toolPage().apply { change(this["turns"].asJsonArray[0].asJsonObject) }
            assertTrue(runCatching { WorkerHistoryCodec.decode(malformed, projection, "task", null) }.isFailure)
        }
        val bounded = toolPage().apply {
            this["turns"].asJsonArray[0].asJsonObject.apply {
                addProperty("tool_calls_truncated", true)
                val calls = this["tool_calls"].asJsonArray
                repeat(7) { n -> calls.add(calls[0].deepCopy().apply { asJsonObject.addProperty("id", "call-$n") }) }
            }
        }
        assertEquals(8, WorkerHistoryCodec.decode(bounded, projection, "task", null).turns.single().toolCalls.size)
    }
}
