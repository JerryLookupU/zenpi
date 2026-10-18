package zenpi.pairing

import zenpi.transport.HostProjection
import zenpi.transport.WorkerHistoryPage
import zenpi.transport.TranscriptPageCodec
import zenpi.transport.WorkerHistoryCodec

internal object WorkerHistoryNavigation {
    const val MAX_PREVIOUS_PAGES = 7

    private fun sameHistory(first: WorkerHistoryPage, second: WorkerHistoryPage): Boolean =
        first.projectId == second.projectId && first.sessionId == second.sessionId
            && first.requestId == second.requestId && first.subtabId == second.subtabId
            && first.digest == second.digest && first.totalTurns == second.totalTurns

    private fun validPage(page: WorkerHistoryPage): Boolean = WorkerHistoryCodec.validPage(page)

    private fun consecutive(first: WorkerHistoryPage, second: WorkerHistoryPage): Boolean =
        sameHistory(first, second) && first.hasMore && first.nextOffset == second.offset

    fun validChain(pages: List<WorkerHistoryPage>, current: WorkerHistoryPage): Boolean {
        if (pages.size > MAX_PREVIOUS_PAGES || !validPage(current) || !pages.all(::validPage)
            || !pages.zipWithNext().all { (first, second) -> consecutive(first, second) }
            || pages.lastOrNull()?.let { consecutive(it, current) } == false) return false
        val ids = (pages + current).flatMap { it.turns }.map { it.id }
        return ids.distinct().size == ids.size
    }

    fun navigation(state: PairingState): Boolean {
        val page = state.workerHistoryPage ?: return false
        return state.phase == PairingPhase.CONNECTED && !state.resetting && !state.resetRequired
            && state.pendingWorkspace == null && state.pendingRevoke == null && state.transcriptTurn == null
            && !state.workerHistoryLoading && state.workerHistoryError == null && state.workerHistoryRequest == page.requestId
            && validScope(page, WorkerReadPolicy.projection(state, page.requestId))
            && validChain(state.workerHistoryPrevious, page)
    }

    fun validScope(page: WorkerHistoryPage, projection: HostProjection?): Boolean {
        if (projection == null || page.projectId != projection.selectedId || page.sessionId != projection.sessionId) return false
        val task = projection.projects.singleOrNull { it.id == page.projectId }?.workerStatuses
            ?.singleOrNull { it.requestId == page.requestId && it.state == "succeeded" } ?: return false
        return task.subtabId == page.subtabId && TranscriptPageCodec.retainsTurn(projection, projection, page.requestId, true)
    }

    fun remember(pages: List<WorkerHistoryPage>, current: WorkerHistoryPage?, next: WorkerHistoryPage): List<WorkerHistoryPage> {
        if (current == null) { require(next.offset == 0 && validChain(emptyList(), next)); return emptyList() }
        require(validChain(pages, current) && validPage(next) && consecutive(current, next))
        require((pages + current).flatMap { it.turns }.none { old -> next.turns.any { it.id == old.id } }
            && next.turns.map { it.id }.distinct().size == next.turns.size)
        return (pages + current).takeLast(MAX_PREVIOUS_PAGES)
    }

    fun previous(state: PairingState): PairingState {
        val previous = state.workerHistoryPrevious.lastOrNull() ?: return state
        if (!navigation(state)) return state
        return state.copy(workerHistoryPage = previous, workerHistoryPrevious = state.workerHistoryPrevious.dropLast(1))
    }

    fun dismissMessage(state: PairingState): PairingState {
        val cleared = state.copy(transcriptTurn = null, transcriptWorker = false, transcriptHistoryTurn = null, transcriptToolCall = null,
            transcriptPage = null, transcriptPreviousPages = emptyList(), transcriptLoading = false, transcriptError = null)
        val history = state.workerHistoryPage
        val projection = if (state.workerReadScope != null && history != null) WorkerReadPolicy.projection(state, history.requestId) else state.projection
        return if (state.transcriptWorker && state.transcriptHistoryTurn != null && history != null
            && state.transcriptTurn == history.requestId && state.workerHistoryRequest == history.requestId
            && history.turns.singleOrNull { it.id == state.transcriptHistoryTurn.id } == state.transcriptHistoryTurn
            && (state.transcriptToolCall == null || state.transcriptHistoryTurn.role == "assistant"
                && state.transcriptHistoryTurn.toolCalls.singleOrNull { it.id == state.transcriptToolCall.id } == state.transcriptToolCall)
            && validChain(state.workerHistoryPrevious, history)
            && validScope(history, projection)) cleared
        else cleared.copy(workerHistoryRequest = null, workerReadScope = null, workerHistoryPage = null, workerHistoryPrevious = emptyList(),
            workerHistoryLoading = false, workerHistoryError = null)
    }
}
