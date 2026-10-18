package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class WorkerHistoryNavigationTest {
    private val tab = "b".repeat(64)
    private val projection = HostProjection(listOf(RemoteProject("project", "Project", "session", workerStatuses = listOf(
        RemoteWorkerStatus("task", "project", "session", tab, "succeeded")))), "project", "session", "idle", emptyList(), "1", true, "1")
    private fun page(index: Int) = WorkerHistoryPage("project", "session", "task", tab, "a".repeat(64), index, index + 1, 20, true,
        listOf(RemoteTurn("turn-$index", "tool", "result-$index", true)))
    @Test fun previousPagesAreBoundedContiguousAndRetainTaskDigestAndScope() {
        var pages = emptyList<WorkerHistoryPage>()
        for (index in 0..9) pages = WorkerHistoryNavigation.remember(pages, page(index), page(index + 1))
        assertEquals(7, pages.size); assertEquals(3, pages.first().offset)
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, workerHistoryRequest = "task", workerHistoryPage = page(10), workerHistoryPrevious = pages)
        val previous = WorkerHistoryNavigation.previous(state)
        assertEquals(page(9), previous.workerHistoryPage); assertEquals(6, previous.workerHistoryPrevious.size)
        assertEquals(pages, WorkerHistoryNavigation.remember(previous.workerHistoryPrevious, previous.workerHistoryPage, page(10)))
    }
    @Test fun mismatchedHistoryCannotEnterNavigationOrReplaceCurrentPage() {
        for (changed in listOf(page(1).copy(requestId = "foreign"), page(1).copy(subtabId = "c".repeat(64)),
            page(1).copy(sessionId = "foreign"), page(1).copy(digest = "c".repeat(64)), page(1).copy(totalTurns = 21),
            page(1).copy(offset = 2), page(1).copy(turns = page(0).turns))) {
            assertTrue(runCatching { WorkerHistoryNavigation.remember(emptyList(), page(0), changed) }.isFailure)
        }
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, workerHistoryRequest = "task", workerHistoryPage = page(1), workerHistoryPrevious = listOf(page(0)))
        for (changed in listOf(state.copy(projection = projection.copy(sessionId = "foreign")), state.copy(workerHistoryRequest = "foreign"),
            state.copy(workerHistoryPrevious = listOf(page(0).copy(digest = "c".repeat(64)))))) assertEquals(changed, WorkerHistoryNavigation.previous(changed))
    }
    @Test fun dismissingSelectedMessageReturnsToSameHistoryPageWithoutKeepingContent() {
        val history = page(4)
        val state = PairingState(projection = projection, workerHistoryRequest = "task", workerHistoryPage = history,
            workerHistoryPrevious = listOf(page(3)), transcriptTurn = "task", transcriptWorker = true, transcriptHistoryTurn = history.turns.single(),
            transcriptLoading = true, transcriptError = "error", transcriptPage = TranscriptPage("project", "session", "turn-4", "tool", "a".repeat(64), 0, 3, 3, false, "abc", "task", tab))
        val restored = WorkerHistoryNavigation.dismissMessage(state)
        assertEquals(history, restored.workerHistoryPage); assertEquals(listOf(page(3)), restored.workerHistoryPrevious)
        assertNull(restored.transcriptTurn); assertNull(restored.transcriptPage); assertNull(restored.transcriptHistoryTurn)
        assertFalse(restored.transcriptLoading); assertNull(restored.transcriptError)
    }
    @Test fun staleScopeOrDifferentMessageCannotRestoreHistoryOnDismissal() {
        val state = PairingState(projection = projection, workerHistoryRequest = "task", workerHistoryPage = page(0), transcriptTurn = "task",
            transcriptWorker = true, transcriptHistoryTurn = page(0).turns.single())
        for (changed in listOf(state.copy(projection = null), state.copy(projection = projection.copy(sessionId = "foreign")),
            state.copy(transcriptTurn = "foreign"), state.copy(transcriptHistoryTurn = RemoteTurn("foreign", "tool", "foreign")), state.copy(transcriptWorker = false),
            state.copy(workerHistoryPrevious = listOf(page(0).copy(requestId = "foreign"))))) {
            val result = WorkerHistoryNavigation.dismissMessage(changed)
            assertNull(result.workerHistoryRequest); assertNull(result.workerHistoryPage); assertTrue(result.workerHistoryPrevious.isEmpty())
        }
        val fenced = AuthorityLossPolicy.displayOnly(state)
        assertNull(fenced.workerHistoryPage); assertNull(fenced.transcriptHistoryTurn)
    }
    @Test fun invalidEarlierStackAndSelectedTurnCannotBeReusedForContinuation() {
        assertTrue(runCatching { WorkerHistoryNavigation.remember(listOf(page(0), page(2)), page(3), page(4)) }.isFailure)
        assertTrue(runCatching { WorkerHistoryNavigation.remember(listOf(page(0).copy(digest = "c".repeat(64))), page(1), page(2)) }.isFailure)
        val current = TranscriptPage("project", "session", "turn-1", "tool", "a".repeat(64), 0, 3, 6, true, "abc", "task", tab)
        assertTrue(runCatching { TranscriptPageCodec.request(projection, "task", current, true, page(2).turns.single()) }.isFailure)
        assertTrue(runCatching { TranscriptPageCodec.request(projection, "task", current, true, page(1).turns.single().copy(role = "user")) }.isFailure)
        assertEquals("turn-1", TranscriptPageCodec.request(projection, "task", current, true, page(1).turns.single())
            ["payload"].asJsonObject["arguments"].asJsonObject["turn_id"].asString)
    }
    @Test fun entireRetainedHistoryIsValidatedBeforePreviousOrContinuation() {
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, workerHistoryRequest = "task",
            workerHistoryPage = page(3), workerHistoryPrevious = listOf(page(0), page(1), page(2)))
        assertTrue(WorkerHistoryNavigation.navigation(state))
        for (history in listOf(listOf(page(0).copy(digest = "c".repeat(64)), page(1), page(2)),
            listOf(page(0).copy(hasMore = false), page(1), page(2)), listOf(page(1), page(0), page(2)),
            listOf(page(0), page(1).copy(turns = page(0).turns), page(2)), (0..7).map(::page))) {
            val changed = state.copy(workerHistoryPrevious = history)
            assertFalse(WorkerHistoryNavigation.navigation(changed))
            assertEquals(changed, WorkerHistoryNavigation.previous(changed))
            assertTrue(runCatching { WorkerHistoryNavigation.remember(history, page(3), page(4)) }.isFailure)
        }
    }
    @Test fun malformedPagesNeverEnterOrRemainInNavigation() {
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, workerHistoryRequest = "task", workerHistoryPage = page(0))
        for (bad in listOf(page(0).copy(digest = "bad"), page(0).copy(totalTurns = 0), page(0).copy(totalTurns = 1025),
            page(0).copy(nextOffset = 2), page(0).copy(hasMore = false), page(0).copy(turns = emptyList()),
            page(0).copy(turns = listOf(page(0).turns.single().copy(role = "invalid"))),
            page(0).copy(turns = listOf(page(0).turns.single().copy(content = "x".repeat(2049)))),
            page(0).copy(turns = listOf(page(0).turns.single().copy(content = "bad\u0000"))),
            page(0).copy(nextOffset = 2, turns = listOf(page(0).turns.single(), page(0).turns.single())))) {
            assertFalse(WorkerHistoryNavigation.navigation(state.copy(workerHistoryPage = bad)))
            assertTrue(runCatching { WorkerHistoryNavigation.remember(emptyList(), null, bad) }.isFailure)
        }
        assertTrue(runCatching { WorkerHistoryNavigation.remember(emptyList(), null, page(1)) }.isFailure)
    }
    @Test fun liveSelectionAndAuthorityAreRequiredForAllHistoryActions() {
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, workerHistoryRequest = "task", workerHistoryPage = page(0))
        assertTrue(WorkerHistoryNavigation.navigation(state))
        for (bad in listOf(state.copy(scanInProgress = true), state.copy(phase = PairingPhase.OFFLINE), state.copy(resetting = true), state.copy(resetRequired = true),
            state.copy(pendingRevoke = "revoke"), state.copy(workerHistoryLoading = true), state.copy(workerHistoryError = "changed"),
            state.copy(transcriptTurn = "task"), state.copy(workerHistoryRequest = "foreign"),
            state.copy(projection = projection.copy(projects = emptyList())),
            state.copy(projection = projection.copy(sessionId = "replacement")),
            state.copy(projection = projection.copy(projects = projection.projects.map { project ->
                project.copy(workerStatuses = project.workerStatuses.map { it.copy(state = "failed") })
            })))) assertFalse(WorkerHistoryNavigation.navigation(bad))
    }
}
