package zenpi

import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class TranscriptBusyRestoreTest {
    private val project = RemoteProject("project", "Workspace", "session")
    private val cached = HostProjection(listOf(project), "project", "session", "idle",
        listOf(RemoteTurn("turn", "assistant", "saved", toolCallsOmitted = true)), "42", false, "7", hasMoreEvents = true)
    private val listing = JsonParser.parseString("""{"mutations_available":false,"active_project_id":"project","required_owner":"${"a".repeat(64)}","workspace_revision":3,"projects":[]}""").asJsonObject
    private fun fallback(previous: HostProjection? = cached, selected: RemoteProject = project, subscribe: Boolean = true,
        available: Boolean = true, mutation: Boolean = false, error: Exception = HostRefusedException("busy")) =
        TranscriptRestorePolicy.busyView(previous, selected, listOf(project.copy(desktopStreamText = "running preview")), listing,
            null, subscribe, available, mutation, error)

    @Test fun busySnapshotPreservesSavedCursorAndOmissionsWhileAdmittingTransientSubscription() {
        val next = fallback()
        assertEquals(cached.turns, next.turns); assertEquals("7", next.eventSequence); assertEquals("42", next.journalSequence)
        assertEquals("running preview", next.projects.single().desktopStreamText)
        assertEquals("3", next.workspaceRevision); assertFalse(next.hasMoreEvents)
        assertFalse(TranscriptRestorePolicy.replay(next, project))
        val restored = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(next))
        assertEquals(cached.turns, restored.turns); assertNull(restored.projects.single().desktopStreamText)
        assertFalse(TranscriptRestorePolicy.replay(restored, project))
        assertTrue(cached.hasMoreEvents)
        val plain = cached.copy(turns = listOf(RemoteTurn("turn", "assistant", "text")))
        assertTrue(TranscriptRestorePolicy.replay(fallback(plain), project))
    }
    @Test fun missingForeignOrNonSubscriptionStateNeverInventsACursorOrSwallowsRefusal() {
        for (previous in listOf(null, cached.copy(eventSequence = null), cached.copy(selectedId = "foreign"), cached.copy(sessionId = "foreign"))) {
            val error = HostRefusedException("busy")
            assertSame(error, runCatching { fallback(previous, error = error) }.exceptionOrNull())
        }
        for (error in listOf(HostRefusedException("unauthorized"), HostRefusedException("owner_mismatch"),
            HostRefusedException("revoked_device"), java.net.SocketTimeoutException("busy"), WireException("Host request refused: busy")))
            assertSame(error, runCatching { fallback(error = error) }.exceptionOrNull())
        for ((subscribe, available, mutation) in listOf(Triple(false, true, false), Triple(true, false, false), Triple(true, true, true))) {
            val error = HostRefusedException("busy")
            assertSame(error, runCatching { fallback(subscribe = subscribe, available = available, mutation = mutation, error = error) }.exceptionOrNull())
        }
        assertTrue(runCatching { fallback(cached.copy(workspaceOwner = "a".repeat(64), workspaceRevision = "4")) }
            .exceptionOrNull() is zenpi.state.StaleProjectionException)
    }
    @Test fun backgroundReadOptsIntoTheListingButNeverForAMutation() {
        // ZE-544: a turn waiting on an approval holds the transcript, so the background read takes
        // the listing (which carries the pending approval) on the unchanged cursor.
        val view = TranscriptRestorePolicy.busyView(cached, project, listOf(project), listing, null,
            subscribe = false, subscriptionAvailable = false, mutation = false,
            error = HostRefusedException("busy"), busyListing = true)
        assertEquals(cached.turns, view.turns); assertEquals("7", view.eventSequence); assertEquals("3", view.workspaceRevision)
        val busy = HostRefusedException("busy")
        assertSame(busy, runCatching {
            TranscriptRestorePolicy.busyView(cached, project, listOf(project), listing, null,
                subscribe = false, subscriptionAvailable = false, mutation = true, error = busy, busyListing = true)
        }.exceptionOrNull())
    }
    @Test fun malformedRetainedCursorOrAmbiguousProjectsCannotResumeOrHideBusyRefusal() {
        val plain = cached.copy(turns = listOf(RemoteTurn("turn", "assistant", "text")))
        val invalid = listOf("", "01", "+1", "-1", "1.0", "1e0", "18446744073709551616")
            .flatMap { listOf(plain.copy(eventSequence = it), plain.copy(journalSequence = it)) } + listOf(
                plain.copy(journalSequence = null), plain.copy(eventSequence = "43"),
                plain.copy(projects = emptyList()), plain.copy(projects = listOf(project.copy(sessionId = "foreign"))),
                plain.copy(projects = listOf(project, project.copy(sessionId = "foreign"))),
                plain.copy(projects = listOf(project, RemoteProject("other", "Other", "other"), RemoteProject("other", "Duplicate", "duplicate"))),
                plain.copy(projects = listOf(project) + List(64) { RemoteProject("p$it", "Project", "s$it") }))
        for (previous in invalid) {
            assertFalse(TranscriptRestorePolicy.replay(previous, project))
            val busy = HostRefusedException("busy")
            assertSame(busy, runCatching { fallback(previous, error = busy) }.exceptionOrNull())
            assertEquals(previous.turns, plain.turns)
        }
        val maximum = plain.copy(eventSequence = "18446744073709551615", journalSequence = "18446744073709551615")
        assertTrue(TranscriptRestorePolicy.replay(maximum, project))
        assertEquals(maximum.eventSequence, fallback(maximum).eventSequence)
        assertTrue(TranscriptRestorePolicy.replay(plain.copy(eventSequence = "0"), project))
    }
    @Test fun busySubscriptionThenDurableTailStillRequiresSnapshotForOldOmittedSummary() = kotlinx.coroutines.runBlocking {
        val initial = fallback()
        val reducer = SubscriptionReducer(initial, project)
        val request = JsonParser.parseString("""{"id":"sub","project_id":"project","session_id":"session","payload":{"type":"subscribe"}}""").asJsonObject
        val transient = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"sub","execution_state":"succeeded","payload":{"type":"response","result":{"cursor":{"session_id":"session","next_sequence":7},"cursor_scope":"transcript_events_v1","snapshot_required":false,"has_more":false,"subscription_transcript":false,"subscription_done":false}}}""").asJsonObject
        val event = JsonParser.parseString("""{"schema_version":1,"channel":"event","project_id":"project","session_id":"session","turn_id":"new","sequence":7,"payload":{"type":"event","name":"transcript_turn","data":{"id":"new","role":"assistant","content":"completed answer"}}}""").asJsonObject
        val terminal = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"sub","execution_state":"succeeded","payload":{"type":"response","result":{"cursor":{"session_id":"session","next_sequence":8},"cursor_scope":"transcript_events_v1","snapshot_required":false,"has_more":false,"phase":"idle","journal_next_sequence":45,"subscription_transcript":true,"subscription_done":true}}}""").asJsonObject
        val wire = java.io.ByteArrayOutputStream().also { output -> listOf(transient, event, terminal).forEach { FrameCodec.write(output, it) } }.toByteArray()
        val publications = mutableListOf<HostProjection>()
        ResponseReader.read(java.io.ByteArrayInputStream(wire), request, {}) { page -> publications += reducer.accept(page.frames, page.result) }
        assertEquals(listOf("7", "8"), publications.map { it.eventSequence })
        assertEquals(listOf("turn", "new"), reducer.projection.turns.map { it.id })
        assertTrue(reducer.projection.turns.first().toolCallsOmitted)
        assertFalse(TranscriptRestorePolicy.replay(reducer.projection, project))
        val snapshot = JsonParser.parseString("""{"project_id":"project","session_id":"session","cursor_scope":"session_journal","phase":"idle","journal_next_sequence":45,"transcript_truncated":false,"mutations_available":false,"event_cursor_scope":"transcript_events_v1","event_cursor":{"session_id":"session","next_sequence":8},"transcript":[{"id":"turn","role":"assistant","content":"saved","tool_calls":[{"id":"call","name":"read_file","arguments_preview":"{}","arguments_truncated":false}],"tool_calls_truncated":false},{"id":"new","role":"assistant","content":"completed answer"}]}""").asJsonObject
        val refreshed = HostProjectionCodec.snapshot(initial.projects, project, snapshot)
        zenpi.state.ProjectionCommitPolicy.requireCurrent(reducer.projection, refreshed)
        assertEquals("read_file", refreshed.turns.first().toolCalls.single().name)
        assertTrue(TranscriptRestorePolicy.replay(refreshed, project))
    }
}
