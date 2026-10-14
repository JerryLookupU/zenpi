package zenpi.pairing

import zenpi.transport.*

internal object TranscriptReadPolicy {
    fun browse(state: PairingState): Boolean = state.phase == PairingPhase.CONNECTED && !state.scanInProgress
        && !state.resetting && !state.resetRequired && state.pendingWorkspace == null && state.pendingRevoke == null
        && !state.transcriptLoading && SessionCatalogCodec.readable(state.projection)
    fun tool(state: PairingState, turnId: String, callId: String): Boolean = browse(state) && state.transcriptTurn == null
        && state.projection?.turns?.singleOrNull { it.id == turnId }?.let {
            it.role == "assistant" && ToolCallProjectionCodec.valid(it) && it.toolCalls.singleOrNull { call -> call.id == callId } != null
        } == true
    fun matches(state: PairingState, page: TranscriptPage): Boolean {
        val id = state.transcriptTurn ?: return false
        val projection = (if (state.transcriptWorker) WorkerReadPolicy.projection(state, id) else SessionHistoryReadPolicy.projection(state, state.projection)) ?: return false
        if (page.projectId != projection.selectedId || page.sessionId != projection.sessionId) return false
        if (projection.projects.singleOrNull { it.id == page.projectId && it.sessionId == page.sessionId } == null) return false
        val turn = if (state.transcriptWorker) state.transcriptHistoryTurn else projection.turns.singleOrNull { it.id == id }
        if (turn != null && !ToolCallProjectionCodec.valid(turn)) return false
        if (page.journalRevision != (if (!state.transcriptWorker && state.transcriptHistoryTurn != null) state.sessionHistory?.journalRevision else null)) return false
        if (!TranscriptPageCodec.retainsTurn(projection, projection, id, state.transcriptWorker)) return false
        state.transcriptToolCall?.let { call ->
            if (turn?.role != "assistant" || turn.toolCalls.singleOrNull { it.id == call.id } != call) return false
        }
        if (state.transcriptWorker) {
            val task = projection.projects.singleOrNull { it.id == projection.selectedId && it.sessionId == projection.sessionId }
                ?.workerStatuses?.singleOrNull { it.requestId == id && it.state == "succeeded" } ?: return false
            if (page.workerRequestId != id || page.subtabId != task.subtabId || turn != null && page.turnId != turn.id) return false
        } else if (page.turnId != id || page.workerRequestId != null || page.subtabId != null) return false
        return page.role == (turn?.role ?: "assistant") && page.toolCallId == state.transcriptToolCall?.id
            && page.toolName == state.transcriptToolCall?.name
    }
    fun navigation(state: PairingState): Boolean = browse(state) && state.transcriptError == null
        && state.transcriptPage?.let { matches(state, it) && TranscriptNavigation.validChain(state.transcriptPreviousPages, it) } == true
    fun previous(state: PairingState): TranscriptPage? = if (navigation(state)) state.transcriptPage?.let {
        runCatching { TranscriptNavigation.previous(state.transcriptPreviousPages, it) }.getOrNull()
    } else null
}
