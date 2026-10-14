package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class SessionReadScopeTest {
    private val project = RemoteProject("project", "Workspace", "session", sessionControl = true)
    private val turn = RemoteTurn("turn", "assistant", "preview", true)
    private val tab = "b".repeat(64)
    private val projection = HostProjection(listOf(project), project.id, project.sessionId, "idle", listOf(turn), "80", false, "1",
        workspaceOwner = "a".repeat(64), workspaceRevision = "1",
        subtabs = RemoteSubtabs(project.id, "1", tab, listOf(RemoteSubtab(tab, "Main", "main", 1)), false))
    private val catalog = SessionCatalog(project.id, "session", "d".repeat(64), 0, 16,
        listOf(RemoteSession("history", "1", 1, false)) + (1 until 16).map { RemoteSession("history-$it", "1", 1, false) })
    private val history = SessionHistoryPage(project.id, "session", "80", "d".repeat(64), 0, 1, 2, true, listOf(turn))
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection,
        sessionsOpen = true, sessions = catalog, sessionHistoryOpen = true, sessionHistory = history,
        transcriptTurn = turn.id, transcriptPage = TranscriptPage(project.id, "session", turn.id, turn.role, "c".repeat(64), 0, 3, 9, true, "abc"))
    private val ambiguous = projection.copy(projects = listOf(project, project.copy(sessionId = "foreign")))

    @Test fun mixedDuplicateProjectIdsRefuseInitialAndContinuationRequestsWithoutBorrowingAuthority() {
        assertTrue(SessionCatalogCodec.readable(projection))
        assertTrue(SessionCatalogReadPolicy.navigation(state))
        assertTrue(SessionHistoryReadPolicy.navigation(state.copy(transcriptTurn = null)))
        assertTrue(TranscriptReadPolicy.navigation(state))
        SessionCommandCodec.newSession(projection)
        SessionCommandCodec.rename(projection, "Renamed")
        SessionCommandCodec.resume(projection, catalog, "history")
        for (invalid in listOf(ambiguous, projection.copy(projects = emptyList()), projection.copy(sessionId = "foreign"))) {
            assertFalse(SessionCatalogCodec.readable(invalid))
            assertFalse(SessionCatalogCodec.sameScope(catalog, invalid))
            assertFalse(SessionHistoryCodec.sameScope(history, invalid))
            assertFalse(SessionCatalogReadPolicy.navigation(state.copy(projection = invalid)))
            assertFalse(SessionHistoryReadPolicy.navigation(state.copy(projection = invalid, transcriptTurn = null)))
            assertFalse(TranscriptReadPolicy.navigation(state.copy(projection = invalid)))
            assertFalse(TranscriptReadPolicy.matches(state.copy(projection = invalid), requireNotNull(state.transcriptPage)))
            assertTrue(runCatching { SessionCatalogCodec.request(invalid, null, 0) }.isFailure)
            assertTrue(runCatching { SessionCatalogCodec.request(invalid, catalog, 16) }.isFailure)
            assertTrue(runCatching { SessionHistoryCodec.request(invalid, null) }.isFailure)
            assertTrue(runCatching { SessionHistoryCodec.request(invalid, history) }.isFailure)
            assertTrue(runCatching { TranscriptPageCodec.request(invalid, turn.id, null) }.isFailure)
            assertTrue(runCatching { TranscriptPageCodec.request(invalid, turn.id, state.transcriptPage) }.isFailure)
            assertTrue(runCatching { SessionCommandCodec.newSession(invalid) }.isFailure)
            assertTrue(runCatching { SessionCommandCodec.rename(invalid, "Renamed") }.isFailure)
            assertTrue(runCatching { SessionCommandCodec.resume(invalid, catalog, "history") }.isFailure)
        }
    }

    @Test fun ambiguousSnapshotCannotRetainMessagesOrWorkersInEitherDirection() {
        assertTrue(TranscriptPageCodec.retainsTurn(projection, projection, turn.id))
        assertFalse(TranscriptPageCodec.retainsTurn(projection, ambiguous, turn.id))
        assertFalse(TranscriptPageCodec.retainsTurn(ambiguous, projection, turn.id))
        val task = RemoteWorkerStatus("worker", project.id, "session", tab, "succeeded")
        val workerProject = project.copy(workerStatuses = listOf(task))
        val workerProjection = projection.copy(projects = listOf(workerProject))
        val ambiguousWorker = workerProjection.copy(projects = listOf(workerProject, project.copy(sessionId = "foreign")))
        assertTrue(TranscriptPageCodec.retainsTurn(workerProjection, workerProjection, task.requestId, true))
        assertFalse(TranscriptPageCodec.retainsTurn(workerProjection, ambiguousWorker, task.requestId, true))
        assertFalse(TranscriptPageCodec.retainsTurn(ambiguousWorker, workerProjection, task.requestId, true))
    }
}
