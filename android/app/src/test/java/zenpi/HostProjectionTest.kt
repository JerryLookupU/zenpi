package zenpi

import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.HostProjectionCodec
import zenpi.transport.RemoteProject
import zenpi.transport.HostProjection

class HostProjectionTest {
    private val project = RemoteProject("project", "Workspace", "session")
    private fun listing() = JsonParser.parseString("""{"projects":[{"id":"project","title":"Workspace","session_id":"session"}],"active_project_id":"project","mutations_available":false}""").asJsonObject
    private fun snapshot() = JsonParser.parseString("""{"project_id":"project","session_id":"session","cursor_scope":"session_journal","phase":"idle","journal_next_sequence":18446744073709551615,"transcript_truncated":false,"mutations_available":false,"transcript":[{"id":"turn","role":"assistant","content":"answer"}]}""").asJsonObject
    @Test fun eventCursorCannotExceedItsDurableJournalButIsNotRequiredToEqualIt() {
        for ((journal, event) in listOf("0" to "0", "9" to "2", "18446744073709551615" to "18446744073709551615")) {
            val value = snapshot().apply {
                add("journal_next_sequence", JsonParser.parseString(journal))
                addProperty("event_cursor_scope", "transcript_events_v1")
                add("event_cursor", JsonParser.parseString("""{"session_id":"session","next_sequence":$event}"""))
            }
            assertEquals(event, HostProjectionCodec.snapshot(listOf(project), project, value).eventSequence)
        }
        for ((journal, event) in listOf("0" to "1", "9" to "10", "18446744073709551614" to "18446744073709551615")) {
            val value = snapshot().apply {
                add("journal_next_sequence", JsonParser.parseString(journal))
                addProperty("event_cursor_scope", "transcript_events_v1")
                add("event_cursor", JsonParser.parseString("""{"session_id":"session","next_sequence":$event}"""))
            }
            assertTrue(runCatching { HostProjectionCodec.snapshot(listOf(project), project, value) }.exceptionOrNull() is zenpi.transport.WireException)
        }
    }
    @Test fun validSnapshotKeepsUnsignedCursorWithoutFloatingPointLoss() {
        val listing = listing()
        val projects = HostProjectionCodec.projects(listing)
        assertEquals(listOf(project), projects)
        assertEquals(project, HostProjectionCodec.select(listing, projects, null))
        val result = HostProjectionCodec.snapshot(projects, project, snapshot())
        assertEquals("18446744073709551615", result.journalSequence)
        assertEquals("answer", result.turns.single().content)
    }
    @Test fun freshListingWithoutWorkspaceMetadataCannotRetainOldAuthority() {
        val live = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false, "0",
            workspaceOwner = "a".repeat(64), workspaceRevision = "9")
        val display = HostProjectionCodec.workspace(live, listing())
        assertNull(display.workspaceOwner); assertNull(display.workspaceRevision)
        assertEquals(live.turns, display.turns); assertEquals(live.eventSequence, display.eventSequence)
        val complete = listing().apply {
            addProperty("required_owner", "b".repeat(64)); addProperty("workspace_revision", 10)
        }
        val replacement = HostProjectionCodec.workspace(live, complete)
        assertEquals("b".repeat(64), replacement.workspaceOwner); assertEquals("10", replacement.workspaceRevision)
        for (missing in listOf("required_owner", "workspace_revision")) {
            assertTrue(runCatching { HostProjectionCodec.workspace(live, complete.deepCopy().apply { remove(missing) }) }.isFailure)
        }
        assertEquals("a".repeat(64), live.workspaceOwner)
    }
    @Test fun largeTurnTruncationRequiresTrueBooleanAndBoundedContent() {
        val value = snapshot()
        val turn = value["transcript"].asJsonArray[0].asJsonObject
        turn.addProperty("content", "x".repeat(12 * 1024)); turn.addProperty("content_truncated", true)
        val parsed = HostProjectionCodec.snapshot(listOf(project), project, value)
        assertTrue(parsed.turns.single().contentTruncated); assertTrue(parsed.truncated)
        for (bad in listOf(com.google.gson.JsonPrimitive(false), com.google.gson.JsonPrimitive("true"), com.google.gson.JsonPrimitive(1))) {
            turn.add("content_truncated", bad)
            assertTrue(runCatching { HostProjectionCodec.snapshot(listOf(project), project, value) }.isFailure)
        }
        turn.addProperty("content_truncated", true); turn.addProperty("content", "x".repeat(96 * 1024 + 1))
        assertTrue(runCatching { HostProjectionCodec.snapshot(listOf(project), project, value) }.isFailure)
    }
    @Test fun malformedWorkspaceAndMissingSelectionFailClosed() {
        for (field in listOf("id", "title", "session_id")) {
            val invalid = listing()
            invalid["projects"].asJsonArray[0].asJsonObject.addProperty(field, 42)
            assertTrue(field, runCatching { HostProjectionCodec.projects(invalid) }.isFailure)
        }
        val duplicate = listing()
        duplicate["projects"].asJsonArray.add(duplicate["projects"].asJsonArray[0].deepCopy())
        assertTrue(runCatching { HostProjectionCodec.projects(duplicate) }.isFailure)
        val invalidActive = listing().apply { addProperty("active_project_id", "foreign") }
        assertTrue(runCatching { HostProjectionCodec.select(invalidActive, listOf(project), null) }.isFailure)
        assertTrue(runCatching { HostProjectionCodec.select(listing(), listOf(project), "removed") }.isFailure)
    }
    @Test fun validatedListingMissingSelectedProjectHasTypedLossButMalformedListingDoesNot() {
        for (value in listOf(listing(), listing().apply {
            addProperty("active_project_id", null as String?); add("projects", com.google.gson.JsonArray())
        })) {
            val projects = HostProjectionCodec.projects(value)
            val error = runCatching { HostProjectionCodec.select(value, projects, "removed") }.exceptionOrNull()
            assertTrue(error is zenpi.transport.SelectedProjectUnavailableException)
            assertEquals("removed", (error as zenpi.transport.SelectedProjectUnavailableException).projectId)
            assertTrue(zenpi.pairing.AuthorityLossPolicy.refused(error))
        }
        val malformed = listing().apply { addProperty("active_project_id", "foreign") }
        val error = requireNotNull(runCatching { HostProjectionCodec.select(malformed, HostProjectionCodec.projects(malformed), "removed") }.exceptionOrNull())
        assertFalse(error is zenpi.transport.SelectedProjectUnavailableException)
        assertFalse(zenpi.pairing.AuthorityLossPolicy.refused(error as Exception))
        assertNull(HostProjectionCodec.select(listing().apply {
            addProperty("active_project_id", null as String?); add("projects", com.google.gson.JsonArray())
        }, emptyList(), null))
    }
    @Test fun authenticatedListingRemovalStopsBeforeReplayAckOrProjectionCommit() = kotlinx.coroutines.runBlocking {
        val request = JsonParser.parseString("""{"id":"listing","payload":{"type":"request","operation":"workspace_list"}}""").asJsonObject
        val response = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"listing","execution_state":"succeeded","payload":{"type":"response","result":{"mutations_available":false,"active_project_id":null,"projects":[]}}}""").asJsonObject
        val bytes = java.io.ByteArrayOutputStream().also { zenpi.transport.FrameCodec.write(it, response) }.toByteArray()
        var followups = 0; var commits = 0
        val error = runCatching {
            val page = zenpi.transport.ResponseReader.read(java.io.ByteArrayInputStream(bytes), request, {})
            val projects = HostProjectionCodec.projects(page.result)
            HostProjectionCodec.select(page.result, projects, "project")
            followups++; commits++
        }.exceptionOrNull()
        assertTrue(error is zenpi.transport.SelectedProjectUnavailableException)
        assertTrue(zenpi.lifecycle.ResumeRetryPolicy.permanent(requireNotNull(error) as Exception))
        assertEquals(0, followups); assertEquals(0, commits)
    }
    @Test fun ownerCursorPhaseAndBooleanCoercionAreRefused() {
        for ((field, value) in listOf("session_id" to "foreign", "project_id" to "foreign", "cursor_scope" to "event_replay", "phase" to "unknown",
            "journal_next_sequence" to "1", "transcript_truncated" to "false", "mutations_available" to "false")) {
            val invalid = snapshot().apply { addProperty(field, value) }
            assertTrue(field, runCatching { HostProjectionCodec.snapshot(listOf(project), project, invalid) }.isFailure)
        }
        for (sequence in listOf(-1L, 1.5)) {
            val invalid = snapshot().apply { addProperty("journal_next_sequence", sequence) }
            assertTrue(runCatching { HostProjectionCodec.snapshot(listOf(project), project, invalid) }.isFailure)
        }
    }
    @Test fun duplicateTurnsUnknownRolesAndOversizeProjectsAreRefused() {
        val duplicate = snapshot()
        duplicate["transcript"].asJsonArray.add(duplicate["transcript"].asJsonArray[0].deepCopy())
        assertTrue(runCatching { HostProjectionCodec.snapshot(listOf(project), project, duplicate) }.isFailure)
        val unknown = snapshot()
        unknown["transcript"].asJsonArray[0].asJsonObject.addProperty("role", "admin")
        assertTrue(runCatching { HostProjectionCodec.snapshot(listOf(project), project, unknown) }.isFailure)
        val oversize = listing()
        repeat(64) { oversize["projects"].asJsonArray.add(oversize["projects"].asJsonArray[0].deepCopy()) }
        assertTrue(runCatching { HostProjectionCodec.projects(oversize) }.isFailure)
    }
    @Test fun optionalLegacyCursorMustBeAbsentAsAPairOrStrictlyScoped() {
        val legacy = snapshot()
        assertNull(HostProjectionCodec.snapshot(listOf(project), project, legacy).eventSequence)
        val current = legacy.deepCopy().apply {
            addProperty("event_cursor_scope", "transcript_events_v1")
            add("event_cursor", JsonParser.parseString("""{"session_id":"session","next_sequence":3}"""))
        }
        assertEquals("3", HostProjectionCodec.snapshot(listOf(project), project, current).eventSequence)
        for (change in listOf<(com.google.gson.JsonObject) -> Unit>(
            { it.remove("event_cursor") }, { it.remove("event_cursor_scope") },
            { it.add("event_cursor", com.google.gson.JsonNull.INSTANCE) },
            { it.getAsJsonObject("event_cursor").addProperty("extra", 0) },
            { it.getAsJsonObject("event_cursor").addProperty("session_id", "foreign") })) {
            assertTrue(runCatching { HostProjectionCodec.snapshot(listOf(project), project, current.deepCopy().apply(change)) }.isFailure)
        }
    }
}
