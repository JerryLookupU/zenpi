package zenpi

import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import zenpi.lifecycle.ResumeReadPolicy
import zenpi.transport.*

class ResumeReadPolicyTest {
    private val previous = HostProjection(listOf(RemoteProject("project", "Workspace", "old")), "project", "old", "idle",
        listOf(RemoteTurn("old-turn", "assistant", "old display")), "42", false, "7")
    private val replacement = previous.copy(projects = listOf(RemoteProject("project", "Workspace", "new")), sessionId = "new",
        turns = emptyList(), journalSequence = "0", eventSequence = "0")

    @Test fun verifiedRaceDropsOldCursorForExactlyOneFreshRead() = runBlocking {
        val reads = mutableListOf<HostProjection?>()
        val next = ResumeReadPolicy.read(previous) { selected ->
            reads += selected
            if (selected != null) throw SessionChangedException("project", "old")
            replacement
        }
        assertEquals(listOf(previous, null), reads); assertEquals(replacement, next)
        assertEquals("7", previous.eventSequence); assertEquals("old display", previous.turns.single().content)
    }
    @Test fun successfulReadAndFirstAttachDoNotIssueAnotherRead() = runBlocking {
        for (selected in listOf(previous, null)) {
            var reads = 0
            assertEquals(replacement, ResumeReadPolicy.read(selected) { assertEquals(selected, it); reads++; replacement })
            assertEquals(1, reads)
        }
    }
    @Test fun foreignRaceAndAuthorityOrTransportErrorsRemainUnchanged() = runBlocking {
        for (error in listOf(SessionChangedException("foreign", "old"), SessionChangedException("project", "foreign"),
            HostRefusedException("owner_mismatch"), HostRefusedException("unauthorized"), HostRefusedException("revoked_device"),
            SelectedProjectUnavailableException("project"),
            WireException("malformed reply"), java.net.SocketTimeoutException("timeout"))) {
            var reads = 0
            assertSame(error, runCatching { ResumeReadPolicy.read(previous) { reads++; throw error } }.exceptionOrNull())
            assertEquals(1, reads)
        }
        val error = SessionChangedException("project", "old")
        assertSame(error, runCatching { ResumeReadPolicy.read(null) { throw error } }.exceptionOrNull())
    }
    @Test fun secondRaceRefusalOrCancellationNeverLoopsOrReturnsStaleDisplay() = runBlocking {
        for (error in listOf(SessionChangedException("project", "new"), HostRefusedException("revoked_device"),
            kotlinx.coroutines.CancellationException("stopped"))) {
            var reads = 0
            assertSame(error, runCatching { ResumeReadPolicy.read(previous) {
                if (++reads == 1) throw SessionChangedException("project", "old")
                throw error
            } }.exceptionOrNull())
            assertEquals(2, reads)
        }
    }
    @Test fun authorizedFramedReadRaceProducesFreshSnapshotWithoutReusingOldResume() = runBlocking {
        val request = com.google.gson.JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"old-resume","project_id":"project","session_id":"old","payload":{"type":"resume","cursor":{"session_id":"old","next_sequence":7},"limit":128}}""").asJsonObject
        val failure = com.google.gson.JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"old-resume","project_id":"project","session_id":"old","code":"owner_mismatch","execution_state":"failed","payload":{"type":"response","result":null}}""").asJsonObject
        val bytes = java.io.ByteArrayOutputStream().also { FrameCodec.write(it, failure) }.toByteArray()
        val listing = com.google.gson.JsonParser.parseString("""{"mutations_available":false,"active_project_id":"project","projects":[{"id":"project","title":"Workspace","session_id":"new"}]}""").asJsonObject
        var reads = 0; var probes = 0
        val next = ResumeReadPolicy.read(previous) { selected ->
            reads++
            if (selected != null) {
                try {
                    ResponseReader.read(java.io.ByteArrayInputStream(bytes), request,
                        { throw HostRefusedException(FrameCodec.string(it, "code")) })
                    error("Refusal must stop the old read")
                } catch (refused: HostRefusedException) {
                    throw SessionChangeProbe.resolve(request, refused, true) { probe ->
                        probes++; assertNotEquals(request["id"], probe["id"])
                        assertEquals("workspace_list", probe.getAsJsonObject("payload")["operation"].asString)
                        listing
                    }
                }
            }
            val snapshot = com.google.gson.JsonParser.parseString("""{"project_id":"project","session_id":"new","cursor_scope":"session_journal","phase":"idle","journal_next_sequence":0,"transcript_truncated":false,"mutations_available":false,"event_cursor_scope":"transcript_events_v1","event_cursor":{"session_id":"new","next_sequence":0},"transcript":[]}""").asJsonObject
            HostProjectionCodec.snapshot(replacement.projects, replacement.projects.single(), snapshot)
        }
        assertEquals(replacement, next); assertEquals(2, reads); assertEquals(1, probes)
        val cached = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(next))
        assertEquals("new", cached.sessionId); assertEquals("0", cached.eventSequence)
        assertTrue(cached.turns.isEmpty()); assertFalse(cached.projects.single().sessionControl)
    }
}
