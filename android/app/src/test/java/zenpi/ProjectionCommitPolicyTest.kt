package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.state.ProjectionCommitPolicy
import zenpi.transport.*

class ProjectionCommitPolicyTest {
    private val initial = HostProjection(listOf(RemoteProject("project", "Workspace", "session")), "project", "session", "idle",
        listOf(RemoteTurn("one", "assistant", "old")), "42", false, "7")
    @Test fun staleJournalCannotCommitEvenWhenTranscriptCursorIsEqualOrGreater() {
        for (event in listOf("6", "7", "8")) {
            assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(initial, initial.copy(journalSequence = "41", eventSequence = event)) }.isFailure)
        }
        assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(initial, initial.copy(eventSequence = "6")) }.isFailure)
        ProjectionCommitPolicy.requireCurrent(initial, initial)
    }
    @Test fun newerSnapshotCanReplaceDisplayWithoutRegressingTransportCursor() {
        val shorter = initial.copy(turns = emptyList(), journalSequence = "43")
        ProjectionCommitPolicy.requireCurrent(initial, shorter)
        assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(shorter, initial) }.isFailure)
        assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(initial, shorter.copy(eventSequence = "0")) }.isFailure)
    }
    @Test fun cursorScopeIsProjectAndSessionNotAnUnrelatedCounter() {
        ProjectionCommitPolicy.requireCurrent(initial, initial.copy(selectedId = "other", journalSequence = "0", eventSequence = "0"))
        ProjectionCommitPolicy.requireCurrent(initial, initial.copy(sessionId = "replacement", journalSequence = "0", eventSequence = "0"))
        ProjectionCommitPolicy.requireCurrent(null, initial)
    }
    @Test fun maximumSequencesAndMalformedNumbersAreNotCoerced() {
        val maximum = initial.copy(journalSequence = "18446744073709551615", eventSequence = "18446744073709551615")
        ProjectionCommitPolicy.requireCurrent(maximum, maximum)
        assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(maximum, initial) }.isFailure)
        for (bad in listOf("-1", "01", "1.0", "18446744073709551616")) {
            assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(initial, initial.copy(journalSequence = bad)) }.isFailure)
            assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(initial, initial.copy(eventSequence = bad)) }.isFailure)
        }
    }
    @Test fun transientViewsRemainCommittableAtTheSameDurableCursor() {
        ProjectionCommitPolicy.requireCurrent(initial, initial.copy(projects = initial.projects.map { it.copy(desktopStreamText = "preview") }))
    }
    @Test fun workspaceRevisionCannotRegressWithinTheSameOwnerEvenWithNewerTranscript() {
        val live = initial.copy(workspaceOwner = "a".repeat(64), workspaceRevision = "9")
        for (journal in listOf("42", "43")) {
            assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(live,
                live.copy(workspaceRevision = "8", journalSequence = journal, eventSequence = "8")) }
                .exceptionOrNull() is zenpi.state.StaleProjectionException)
        }
        ProjectionCommitPolicy.requireCurrent(live, live.copy(workspaceRevision = "10"))
        ProjectionCommitPolicy.requireCurrent(live, live.copy(workspaceOwner = "b".repeat(64), workspaceRevision = "0"))
        ProjectionCommitPolicy.requireCurrent(live, live.copy(workspaceOwner = null, workspaceRevision = null))
        ProjectionCommitPolicy.requireCurrent(live.copy(journalSequence = null), live.copy(journalSequence = null, workspaceRevision = "10"))
        assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(live.copy(journalSequence = null),
            live.copy(journalSequence = null, workspaceRevision = "8")) }.exceptionOrNull() is zenpi.state.StaleProjectionException)
        ProjectionCommitPolicy.requireCurrent(live, live.copy(sessionId = "replacement", workspaceRevision = "10", journalSequence = "0", eventSequence = "0"))
        for (bad in listOf("-1", "01", "9.0", "18446744073709551616")) {
            assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(live, live.copy(workspaceRevision = bad)) }.isFailure)
        }
    }
    @Test fun globalWorkspaceRevisionCannotRollBackDuringProjectOrSessionSwitch() {
        val live = initial.copy(workspaceOwner = "a".repeat(64), workspaceRevision = "9")
        for (replacement in listOf(live.copy(selectedId = "other"), live.copy(sessionId = "replacement"),
            live.copy(selectedId = null, sessionId = null))) {
            val stale = replacement.copy(workspaceRevision = "8", journalSequence = "0", eventSequence = "0")
            assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(live, stale) }.exceptionOrNull() is zenpi.state.StaleProjectionException)
            ProjectionCommitPolicy.requireCurrent(live, stale.copy(workspaceRevision = "9"))
            ProjectionCommitPolicy.requireCurrent(live, stale.copy(workspaceRevision = "10"))
            ProjectionCommitPolicy.requireCurrent(live, stale.copy(workspaceOwner = "b".repeat(64), workspaceRevision = "0"))
            for (bad in listOf("-1", "01", "9.0", "18446744073709551616"))
                assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(live, stale.copy(workspaceRevision = bad)) }.isFailure)
        }
    }
    @Test fun eventCursorMonotonicityDoesNotDependOnOptionalJournalSequence() {
        val eventOnly = initial.copy(journalSequence = null)
        assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(eventOnly, eventOnly.copy(eventSequence = "6")) }
            .exceptionOrNull() is zenpi.state.StaleProjectionException)
        ProjectionCommitPolicy.requireCurrent(eventOnly, eventOnly.copy(eventSequence = "8"))
        for (bad in listOf("-1", "01", "7.0", "18446744073709551616"))
            assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(eventOnly, eventOnly.copy(eventSequence = bad)) }.isFailure)
        ProjectionCommitPolicy.requireCurrent(eventOnly, eventOnly.copy(sessionId = "replacement", eventSequence = "0"))
    }
    @Test fun knownEventCursorCannotDisappearWithinTheSameProjectAndSession() {
        for (journal in listOf(null, "42", "43")) {
            val saved = initial.copy(journalSequence = journal)
            var durable = saved
            var visible = saved
            val missing = saved.copy(turns = emptyList(), eventSequence = null)
            val error = runCatching {
                ProjectionCommitPolicy.requireCurrent(durable, missing)
                durable = missing; visible = missing
            }.exceptionOrNull()
            assertTrue(error is WireException)
            assertEquals("Missing cache event sequence", error?.message)
            assertEquals(saved, durable); assertEquals(saved, visible)
            ProjectionCommitPolicy.requireCurrent(saved, saved.copy(eventSequence = "8"))
            ProjectionCommitPolicy.requireCurrent(saved, missing.copy(selectedId = "other"))
            ProjectionCommitPolicy.requireCurrent(saved, missing.copy(sessionId = "replacement"))
        }
        val legacy = initial.copy(eventSequence = null)
        ProjectionCommitPolicy.requireCurrent(null, legacy)
        ProjectionCommitPolicy.requireCurrent(legacy, legacy)
        ProjectionCommitPolicy.requireCurrent(legacy, initial)
    }
    private val tabs = RemoteSubtabs("project", "7", "a".repeat(64),
        listOf(RemoteSubtab("a".repeat(64), "renamed", "main", 1)), false)
    @Test fun tabRevisionCannotRegressAtEqualOrNewerTranscriptCursor() {
        val saved = initial.copy(subtabs = tabs)
        for (journal in listOf("42", "43")) {
            assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(saved,
                saved.copy(journalSequence = journal, subtabs = tabs.copy(revision = "6"))) }.exceptionOrNull() is zenpi.state.StaleProjectionException)
        }
        assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(saved, saved.copy(subtabs = null)) }.isFailure)
        ProjectionCommitPolicy.requireCurrent(saved, saved.copy(subtabs = tabs.copy(revision = "8")))
        ProjectionCommitPolicy.requireCurrent(saved, saved.copy(subtabs = tabs.copy(control = true)))
    }
    @Test fun cachedTabsKeepTheRevisionFenceWithoutKeepingAuthority() {
        val live = initial.copy(subtabs = tabs.copy(control = true))
        val cached = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(live))
        assertFalse(requireNotNull(cached.subtabs).control)
        assertEquals("7", cached.subtabs?.revision)
        assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(cached,
            live.copy(subtabs = tabs.copy(revision = "6"))) }.isFailure)
        ProjectionCommitPolicy.requireCurrent(cached, live)
        ProjectionCommitPolicy.requireCurrent(cached, initial.copy(selectedId = "other", subtabs = null))
        ProjectionCommitPolicy.requireCurrent(cached, initial.copy(sessionId = "replacement", subtabs = null))
    }
    @Test fun tabRevisionDoesNotDependOnOptionalJournalAndRejectsMalformedOrForeignScope() {
        val saved = initial.copy(journalSequence = null, subtabs = tabs)
        assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(saved, saved.copy(subtabs = tabs.copy(revision = "6"))) }.isFailure)
        for (revision in listOf("-1", "01", "7.0", "18446744073709551616")) {
            assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(saved, saved.copy(subtabs = tabs.copy(revision = revision))) }.isFailure)
        }
        for (value in listOf(saved.copy(subtabs = tabs.copy(projectId = "foreign")),
            saved.copy(subtabs = tabs.copy(revision = "18446744073709551615")))) {
            assertTrue(runCatching { ProjectionCommitPolicy.requireCurrent(value, saved) }.isFailure)
        }
    }
    @Test fun refusedPublicationStopsReaderWithoutReplacingDurableDisplay() = kotlinx.coroutines.runBlocking {
        val newer = initial.copy(journalSequence = "43", eventSequence = "8")
        var durable = newer
        var visible = newer
        val frame = com.google.gson.JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"sub","execution_state":"succeeded","payload":{"type":"response","result":{"subscription_done":true}}}""").asJsonObject
        val bytes = java.io.ByteArrayOutputStream().also { FrameCodec.write(it, frame) }.toByteArray()
        val request = com.google.gson.JsonParser.parseString("""{"id":"sub","payload":{"type":"subscribe"}}""").asJsonObject
        val error = runCatching {
            ResponseReader.read(java.io.ByteArrayInputStream(bytes), request, {}) {
                ProjectionCommitPolicy.requireCurrent(durable, initial)
                durable = initial
                visible = initial
            }
        }.exceptionOrNull()
        assertTrue(error is WireException); assertEquals(newer, durable); assertEquals(newer, visible)
    }
}
