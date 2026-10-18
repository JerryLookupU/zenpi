package zenpi

import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class ReplayTest {
    private val project = RemoteProject("project", "Workspace", "session")
    private val previous = HostProjection(listOf(project), "project", "session", "idle", listOf(RemoteTurn("one", "user", "hello")), "5", false, "1")
    private fun result() = JsonParser.parseString("""{"cursor":{"session_id":"session","next_sequence":2},"cursor_scope":"transcript_events_v1","snapshot_required":false,"has_more":false,"phase":"idle","journal_next_sequence":9}""").asJsonObject
    private fun event() = FrameCodec.decode("""{"schema_version":1,"channel":"event","session_id":"session","project_id":"project","turn_id":"two","sequence":1,"payload":{"type":"event","name":"transcript_turn","data":{"id":"two","role":"assistant","content":"answer"}}}""".toByteArray())
    @Test fun impossibleTerminalCursorNeverPublishesReplayOrSubscription() {
        val cache = zenpi.state.ProjectionCacheCodec.encode(previous)
        for (journal in listOf(0, 1)) {
            val terminal = result().apply { addProperty("journal_next_sequence", journal) }
            assertTrue(runCatching { HostProjectionCodec.replay(previous, listOf(project), project, listOf(event()), terminal) }
                .exceptionOrNull() is WireException)
            val reducer = SubscriptionReducer(previous, project)
            terminal.addProperty("subscription_done", true)
            assertTrue(runCatching { reducer.accept(listOf(event()), terminal) }.exceptionOrNull() is WireException)
            assertEquals(previous, reducer.projection); assertFalse(reducer.done)
            assertEquals(cache, zenpi.state.ProjectionCacheCodec.encode(previous))
        }
    }
    @Test fun contiguousPageAppliesExactlyOnceAndEmptyReconnectKeepsState() {
        val updated = HostProjectionCodec.replay(previous, listOf(project), project, listOf(event()), result())
        assertEquals(listOf("one", "two"), updated.turns.map { it.id }); assertEquals("2", updated.eventSequence)
        assertEquals("9", updated.journalSequence)
        assertEquals(updated, HostProjectionCodec.replay(updated, listOf(project), project, emptyList(), result()))
        assertTrue(runCatching { HostProjectionCodec.replay(updated, listOf(project), project, listOf(event()), result()) }.isFailure)
    }
    @Test fun manyLargePagesRetainByteBoundedTailAndMatchingCursorForCacheCommit() {
        var current = previous.copy(turns = emptyList(), eventSequence = "0")
        repeat(16) { index ->
            val frame = event().apply {
                addProperty("sequence", index); addProperty("turn_id", "large-$index")
                getAsJsonObject("payload").getAsJsonObject("data").apply {
                    addProperty("id", "large-$index"); addProperty("content", "x".repeat(12 * 1024)); addProperty("content_truncated", true)
                }
            }
            val terminal = result().apply { getAsJsonObject("cursor").addProperty("next_sequence", index + 1); addProperty("journal_next_sequence", index + 9) }
            current = HostProjectionCodec.replay(current, listOf(project), project, listOf(frame), terminal)
            assertEquals((index + 1).toString(), current.eventSequence)
            assertEquals("large-$index", current.turns.last().id)
            assertTrue(current.truncated); assertTrue(current.turns.last().contentTruncated)
            val encoded = zenpi.state.ProjectionCacheCodec.encode(current)
            val restored = zenpi.state.ProjectionCacheCodec.decode(encoded)
            assertEquals(current.turns, restored.turns); assertEquals(current.eventSequence, restored.eventSequence)
            assertTrue(restored.truncated)
        }
        assertTrue(current.turns.size < 16)
        assertTrue(current.turns.sumOf { it.content.toByteArray().size } <= 96 * 1024)
    }
    @Test fun escapedContentBudgetDoesNotOverflowEncryptedDisplayRecord() {
        var current = previous.copy(turns = emptyList(), eventSequence = "0")
        repeat(4) { index ->
            val frame = event().apply {
                addProperty("sequence", index); addProperty("turn_id", "escaped-$index")
                getAsJsonObject("payload").getAsJsonObject("data").apply {
                    addProperty("id", "escaped-$index"); addProperty("content", "\u0001".repeat(12 * 1024))
                }
            }
            val terminal = result().apply { getAsJsonObject("cursor").addProperty("next_sequence", index + 1) }
            current = HostProjectionCodec.replay(current, listOf(project), project, listOf(frame), terminal)
            assertEquals(1, current.turns.size)
            assertEquals((index + 1).toString(), current.eventSequence)
            val encoded = zenpi.state.ProjectionCacheCodec.encode(current)
            assertTrue(encoded.toString().toByteArray().size <= zenpi.state.ProjectionCacheCodec.MAX_BYTES)
        }
    }
    @Test fun oversizedLegacyTurnKeepsIdentityAndPagingInsteadOfCommittingAnEmptyDisplay() {
        for (content in listOf("x".repeat(96 * 1024), "\u0001".repeat(20 * 1024), "\uD83D\uDE00".repeat(24 * 1024))) {
            val frame = event().apply { getAsJsonObject("payload").getAsJsonObject("data").addProperty("content", content) }
            val next = HostProjectionCodec.replay(previous, listOf(project), project, listOf(frame), result())
            assertEquals("two", next.turns.single().id); assertEquals("2", next.eventSequence)
            assertTrue(next.truncated); assertTrue(next.turns.single().contentTruncated)
            assertTrue(next.turns.single().content.isNotEmpty()); assertTrue(content.startsWith(next.turns.single().content))
            assertFalse(Character.isHighSurrogate(next.turns.single().content.last()))
            val cached = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(next))
            assertEquals(next.turns, cached.turns); assertEquals("2", cached.eventSequence)
            assertEquals("two", TranscriptPageCodec.request(cached, "two", null)["payload"].asJsonObject["arguments"].asJsonObject["turn_id"].asString)
            val snapshot = JsonParser.parseString("""{"project_id":"project","session_id":"session","cursor_scope":"session_journal","phase":"idle","journal_next_sequence":9,"transcript_truncated":false,"mutations_available":false,"event_cursor_scope":"transcript_events_v1","event_cursor":{"session_id":"session","next_sequence":2},"transcript":[]}""").asJsonObject
            snapshot["transcript"].asJsonArray.add(frame["payload"].asJsonObject["data"].deepCopy())
            val fresh = HostProjectionCodec.snapshot(listOf(project), project, snapshot)
            assertEquals(next.turns, fresh.turns); assertTrue(fresh.truncated)
        }
    }
    @Test fun wrongSequenceOwnerTerminalOrRoleFailsAtomically() {
        for ((field, value) in listOf("sequence" to "0", "session_id" to "foreign", "project_id" to "foreign", "turn_id" to "foreign")) {
            val invalid = event().apply { addProperty(field, value) }
            assertTrue(field, runCatching { HostProjectionCodec.replay(previous, listOf(project), project, listOf(invalid), result()) }.isFailure)
        }
        val wrongRole = event().apply { get("payload").asJsonObject["data"].asJsonObject.addProperty("role", "admin") }
        assertTrue(runCatching { HostProjectionCodec.replay(previous, listOf(project), project, listOf(wrongRole), result()) }.isFailure)
        val terminal = result().apply { get("cursor").asJsonObject.addProperty("next_sequence", 3) }
        assertTrue(runCatching { HostProjectionCodec.replay(previous, listOf(project), project, listOf(event()), terminal) }.isFailure)
        assertEquals(listOf("one"), previous.turns.map { it.id })
    }
    @Test fun gapRequiresBoundSnapshotAndNeverAdvancesOnMissingData() {
        val gap = result().apply { addProperty("snapshot_required", true) }
        assertTrue(runCatching { HostProjectionCodec.replay(previous, listOf(project), project, emptyList(), gap) }.isFailure)
        val stalled = result().apply { addProperty("has_more", true); get("cursor").asJsonObject.addProperty("next_sequence", 1) }
        assertTrue(runCatching { HostProjectionCodec.replay(previous, listOf(project), project, emptyList(), stalled) }.isFailure)
        val snapshot = FrameCodec.decode("""{"schema_version":1,"channel":"bulk","session_id":"session","project_id":"project","payload":{"type":"snapshot","cursor":{"session_id":"session","next_sequence":2},"state":{"project_id":"project","session_id":"session","cursor_scope":"session_journal","phase":"idle","journal_next_sequence":9,"transcript_truncated":true,"mutations_available":false,"event_cursor_scope":"transcript_events_v1","event_cursor":{"session_id":"session","next_sequence":2},"transcript":[{"id":"two","role":"assistant","content":"answer"}]}}}""".toByteArray())
        val replaced = HostProjectionCodec.replay(previous, listOf(project), project, listOf(snapshot), gap)
        assertEquals(listOf("two"), replaced.turns.map { it.id }); assertEquals("2", replaced.eventSequence); assertTrue(replaced.truncated)
        assertTrue(replaced.snapshotReplaced)
        val state = zenpi.pairing.PairingState(phase = zenpi.pairing.PairingPhase.CONNECTED, projection = replaced)
        assertEquals("Snapshot replaced", zenpi.pairing.SynchronizationStatus.label(state))
        val cached = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(replaced))
        assertFalse(cached.snapshotReplaced)
        val heartbeat = HostProjectionCodec.replay(replaced, listOf(project), project, emptyList(), result())
        assertFalse(heartbeat.snapshotReplaced)
        assertNull(zenpi.pairing.SynchronizationStatus.label(state.copy(projection = heartbeat)))
    }
    @Test fun branchReplacementKeepsPhysicalCursorButRemovesOldMessagesAndPageAuthority() {
        val old = previous.copy(turns = listOf(RemoteTurn("abandoned", "assistant", "old prefix", true)), eventSequence = "2")
        val terminal = result().apply { addProperty("snapshot_required", true) }
        val frame = FrameCodec.decode("""{"schema_version":1,"channel":"bulk","session_id":"session","project_id":"project","payload":{"type":"snapshot","cursor":{"session_id":"session","next_sequence":2},"state":{"project_id":"project","session_id":"session","cursor_scope":"session_journal","phase":"idle","journal_next_sequence":9,"transcript_truncated":false,"mutations_available":false,"event_cursor_scope":"transcript_events_v1","event_cursor":{"session_id":"session","next_sequence":2},"transcript":[{"id":"alternate","role":"assistant","content":"other branch"}]}}}""".toByteArray())
        val next = HostProjectionCodec.replay(old, listOf(project), project, listOf(frame), terminal)
        zenpi.state.ProjectionCommitPolicy.requireCurrent(old, next)
        assertEquals(old.eventSequence, next.eventSequence); assertEquals("9", next.journalSequence)
        assertEquals(listOf("alternate"), next.turns.map { it.id }); assertFalse(next.truncated)
        assertFalse(TranscriptPageCodec.retainsTurn(old, next, "abandoned"))
        val cached = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(next))
        assertEquals(next.turns, cached.turns); assertEquals(next.eventSequence, cached.eventSequence)
        frame["payload"].asJsonObject["state"].asJsonObject.add("transcript", com.google.gson.JsonArray())
        val empty = HostProjectionCodec.replay(next, listOf(project), project, listOf(frame), terminal)
        zenpi.state.ProjectionCommitPolicy.requireCurrent(next, empty)
        assertTrue(empty.turns.isEmpty()); assertEquals("2", empty.eventSequence)
        assertFalse(TranscriptPageCodec.retainsTurn(next, empty, "alternate"))
    }
    @Test fun snapshotTerminalAndEnvelopeMustAgreeBeforeDisplayOrCursorReplacement() {
        val gap = result().apply { addProperty("snapshot_required", true) }
        fun frame() = FrameCodec.decode("""{"schema_version":1,"channel":"bulk","session_id":"session","project_id":"project","payload":{"type":"snapshot","cursor":{"session_id":"session","next_sequence":2},"state":{"project_id":"project","session_id":"session","cursor_scope":"session_journal","phase":"idle","journal_next_sequence":9,"transcript_truncated":false,"mutations_available":false,"event_cursor_scope":"transcript_events_v1","event_cursor":{"session_id":"session","next_sequence":2},"transcript":[{"id":"replacement","role":"assistant","content":"fresh"}]}}}""".toByteArray())
        val cache = zenpi.state.ProjectionCacheCodec.encode(previous)
        for (change in listOf<(com.google.gson.JsonObject) -> Unit>(
            { it.addProperty("phase", "running") }, { it.addProperty("phase", "unknown") },
            { it.addProperty("journal_next_sequence", 10) }, { it.addProperty("journal_next_sequence", "9") },
            { it.remove("journal_next_sequence") }, { it.getAsJsonObject("cursor").addProperty("unexpected", 0) })) {
            assertTrue(runCatching { HostProjectionCodec.replay(previous, listOf(project), project, listOf(frame()), gap.deepCopy().apply(change)) }.isFailure)
            assertEquals(cache, zenpi.state.ProjectionCacheCodec.encode(previous))
        }
        for (changed in listOf(frame().apply { addProperty("session_id", "foreign") }, frame().apply { remove("session_id") },
            frame().apply { getAsJsonObject("payload").getAsJsonObject("state").getAsJsonObject("event_cursor").addProperty("extra", 1) })) {
            assertTrue(runCatching { HostProjectionCodec.replay(previous, listOf(project), project, listOf(changed), gap) }.isFailure)
        }
        assertTrue(runCatching { HostProjectionCodec.replay(previous.copy(sessionId = "foreign"), listOf(project), project, listOf(frame()), gap) }.isFailure)
        assertTrue(runCatching { HostProjectionCodec.replay(previous.copy(selectedId = "foreign"), listOf(project), project, listOf(frame()), gap) }.isFailure)
        val valid = HostProjectionCodec.replay(previous, listOf(project), project, listOf(frame()), gap)
        assertEquals(listOf("replacement"), valid.turns.map { it.id }); assertEquals("2", valid.eventSequence); assertEquals("9", valid.journalSequence)
    }
}
