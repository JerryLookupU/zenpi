package zenpi

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class ToolCallProjectionTest {
    private val project = RemoteProject("project", "Workspace", "session")
    private fun calls() = JsonParser.parseString("""[{"id":"call","name":"read_file","arguments_preview":"{\"path\":\"note.txt\"}","arguments_truncated":false}]""").asJsonArray
    private fun turn() = JsonObject().apply {
        addProperty("id", "turn"); addProperty("role", "assistant"); addProperty("content", "")
        add("tool_calls", calls()); addProperty("tool_calls_truncated", false)
    }
    private fun snapshot(turn: JsonObject) = JsonObject().apply {
        addProperty("project_id", "project"); addProperty("session_id", "session")
        addProperty("cursor_scope", "session_journal"); addProperty("phase", "idle")
        addProperty("journal_next_sequence", 3); addProperty("transcript_truncated", false)
        addProperty("mutations_available", false)
        addProperty("event_cursor_scope", "transcript_events_v1")
        add("event_cursor", JsonObject().apply { addProperty("session_id", "session"); addProperty("next_sequence", 1) })
        add("transcript", JsonArray().apply { add(turn) })
    }
    @Test fun snapshotAndReplayRetainCallsButOfflineCacheDoesNotPersistArguments() {
        val source = turn()
        val projected = HostProjectionCodec.snapshot(listOf(project), project, snapshot(source))
        assertEquals("read_file", projected.turns.single().toolCalls.single().name)
        val previous = projected.copy(turns = emptyList(), eventSequence = "0")
        val frame = JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "event")
            addProperty("project_id", "project"); addProperty("session_id", "session")
            addProperty("turn_id", "turn"); addProperty("sequence", 0)
            add("payload", JsonObject().apply {
                addProperty("type", "event"); addProperty("name", "transcript_turn"); add("data", source)
            })
        }
        val result = JsonParser.parseString("""{"cursor":{"session_id":"session","next_sequence":1},"cursor_scope":"transcript_events_v1","snapshot_required":false,"has_more":false,"phase":"idle","journal_next_sequence":3}""").asJsonObject
        val replayed = HostProjectionCodec.replay(previous, listOf(project), project, listOf(frame), result)
        assertEquals(projected.turns, replayed.turns)
        val cached = zenpi.state.ProjectionCacheCodec.encode(replayed)
        assertFalse(cached.toString().contains("arguments_preview"))
        val restored = zenpi.state.ProjectionCacheCodec.decode(cached)
        assertTrue(restored.turns.single().toolCalls.isEmpty())
        assertTrue(restored.turns.single().toolCallsOmitted)
        assertEquals(cached, zenpi.state.ProjectionCacheCodec.encode(restored))
        assertEquals("1", restored.eventSequence)
    }
    @Test fun invalidRoleDuplicateOversizeAndUnpairedFlagsFailClosed() {
        val invalid = mutableListOf<JsonObject>()
        invalid.add(turn().apply { addProperty("role", "tool") })
        invalid.add(turn().apply { remove("tool_calls_truncated") })
        invalid.add(turn().apply { remove("tool_calls") })
        invalid.add(turn().apply { addProperty("tool_calls_truncated", true) })
        invalid.add(turn().apply { addProperty("tool_calls_truncated", "false") })
        invalid.add(turn().apply { getAsJsonArray("tool_calls").add(getAsJsonArray("tool_calls")[0].deepCopy()) })
        invalid.add(turn().apply { repeat(8) { getAsJsonArray("tool_calls").add(calls()[0].deepCopy()) } })
        for ((field, value) in listOf("name" to "x".repeat(65), "id" to "x".repeat(129),
            "arguments_preview" to "界".repeat(171), "arguments_preview" to "\u0000")) {
            invalid.add(turn().apply { getAsJsonArray("tool_calls")[0].asJsonObject.addProperty(field, value) })
        }
        invalid.forEach { item ->
            assertTrue(runCatching { HostProjectionCodec.snapshot(listOf(project), project, snapshot(item)) }.isFailure)
        }
    }
    @Test fun restoredDisplayRequiresSnapshotBeforeTailCanRecoverInvocationDetails() {
        val live = HostProjectionCodec.snapshot(listOf(project), project, snapshot(turn())).copy(
            workspaceOwner = "a".repeat(64), workspaceRevision = "1")
        assertTrue(TranscriptRestorePolicy.replay(live, project))
        val cached = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(live))
        assertTrue(cached.turns.single().toolCalls.isEmpty()); assertEquals(live.eventSequence, cached.eventSequence)
        assertFalse(TranscriptRestorePolicy.replay(cached, project))
        val refreshed = HostProjectionCodec.snapshot(listOf(project), project, snapshot(turn())).copy(
            workspaceOwner = live.workspaceOwner, workspaceRevision = live.workspaceRevision)
        zenpi.state.ProjectionCommitPolicy.requireCurrent(cached, refreshed)
        assertEquals(live.turns, refreshed.turns); assertEquals(cached.eventSequence, refreshed.eventSequence)
        assertTrue(TranscriptRestorePolicy.replay(refreshed, project))
        assertTrue(TranscriptRestorePolicy.replay(live.copy(workspaceOwner = null), project))
        val plain = cached.copy(turns = listOf(RemoteTurn("plain", "assistant", "text")))
        assertTrue(TranscriptRestorePolicy.replay(plain, project))
        for (previous in listOf(null, live.copy(eventSequence = null), live.copy(selectedId = "foreign"),
            live.copy(sessionId = "foreign")))
            assertFalse(TranscriptRestorePolicy.replay(previous, project))
    }
    @Test fun cacheOmissionMarkerIsBooleanAssistantOnlyAndNeverContainsArguments() {
        val live = HostProjectionCodec.snapshot(listOf(project), project, snapshot(turn()))
        val encoded = zenpi.state.ProjectionCacheCodec.encode(live)
        val item = encoded["snapshot"].asJsonObject["transcript"].asJsonArray[0].asJsonObject
        assertEquals("true", item["tool_calls_omitted"].toString())
        for (change in listOf<(JsonObject) -> Unit>({ it.addProperty("tool_calls_omitted", false) },
            { it.addProperty("tool_calls_omitted", "true") }, { it.addProperty("role", "tool") },
            { it.addProperty("tool_calls", "private arguments") })) {
            val bad = encoded.deepCopy(); change(bad["snapshot"].asJsonObject["transcript"].asJsonArray[0].asJsonObject)
            assertTrue(runCatching { zenpi.state.ProjectionCacheCodec.decode(bad) }.isFailure)
        }
        val plain = live.copy(turns = listOf(RemoteTurn("plain", "assistant", "text")))
        assertFalse(zenpi.state.ProjectionCacheCodec.encode(plain).toString().contains("tool_calls_omitted"))
    }
    @Test fun legacyUnmarkedAssistantRequiresSnapshotAndMigrationCannotEraseThatRequirement() {
        val live = HostProjectionCodec.snapshot(listOf(project), project, snapshot(turn()))
            .copy(turns = listOf(RemoteTurn("user", "user", "question"), RemoteTurn("assistant", "assistant", "answer")))
        for (version in listOf(1, 2, 3)) {
            val old = zenpi.state.ProjectionCacheCodec.encode(live).apply {
                addProperty("version", version)
                if (version == 1) remove("subtabs")
                if (version < 3) getAsJsonObject("listing").getAsJsonArray("projects").forEach {
                    it.asJsonObject.remove("prompt_statuses")
                }
            }
            val original = old.deepCopy()
            val restored = zenpi.state.ProjectionCacheCodec.decode(old)
            assertEquals(original, old)
            assertFalse(restored.turns[0].toolCallsOmitted)
            assertTrue(restored.turns[1].toolCallsOmitted)
            assertEquals(live.eventSequence, restored.eventSequence)
            assertEquals(live.journalSequence, restored.journalSequence)
            assertFalse(TranscriptRestorePolicy.replay(restored, project))
            val upgraded = zenpi.state.ProjectionCacheCodec.encode(restored)
            assertEquals("4", upgraded["version"].toString())
            assertEquals(restored, zenpi.state.ProjectionCacheCodec.decode(upgraded))
            assertFalse(TranscriptRestorePolicy.replay(zenpi.state.ProjectionCacheCodec.decode(upgraded), project))
            val refreshed = HostProjectionCodec.snapshot(listOf(project), project, snapshot(turn()))
            zenpi.state.ProjectionCommitPolicy.requireCurrent(restored, refreshed)
            assertTrue(TranscriptRestorePolicy.replay(refreshed, project))
        }
        val plainV4 = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(live))
        assertTrue(TranscriptRestorePolicy.replay(plainV4, project))
    }
    @Test fun earlierMarkedV3StillLoadsButOlderSchemasRejectTheAdditiveMarker() {
        val encoded = zenpi.state.ProjectionCacheCodec.encode(HostProjectionCodec.snapshot(listOf(project), project, snapshot(turn())))
        val earlier = encoded.deepCopy().apply { addProperty("version", 3) }
        assertTrue(zenpi.state.ProjectionCacheCodec.decode(earlier).turns.single().toolCallsOmitted)
        for (version in listOf(1, 2)) {
            val invalid = earlier.deepCopy().apply {
                addProperty("version", version)
                if (version == 1) remove("subtabs")
                getAsJsonObject("listing").getAsJsonArray("projects").forEach { it.asJsonObject.remove("prompt_statuses") }
            }
            assertTrue(runCatching { zenpi.state.ProjectionCacheCodec.decode(invalid) }.isFailure)
        }
    }
    @Test fun legacyUserOnlyAndEmptyDisplayStillResumeWithoutInventingAssistantOmissions() {
        val live = HostProjectionCodec.snapshot(listOf(project), project, snapshot(turn()))
        for (turns in listOf(emptyList(), listOf(RemoteTurn("user", "user", "question")))) {
            for (version in listOf(1, 2, 3)) {
                val old = zenpi.state.ProjectionCacheCodec.encode(live.copy(turns = turns)).apply {
                    addProperty("version", version)
                    if (version == 1) remove("subtabs")
                    if (version < 3) getAsJsonObject("listing").getAsJsonArray("projects").forEach {
                        it.asJsonObject.remove("prompt_statuses")
                    }
                }
                val restored = zenpi.state.ProjectionCacheCodec.decode(old)
                assertEquals(turns, restored.turns)
                assertTrue(TranscriptRestorePolicy.replay(restored, project))
            }
        }
    }
}
