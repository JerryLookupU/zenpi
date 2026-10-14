package zenpi.pairing

import zenpi.transport.*

internal object SessionHistoryReadPolicy {
    fun browse(state: PairingState): Boolean = state.phase == PairingPhase.CONNECTED && !state.scanInProgress
        && !state.resetting && !state.resetRequired && state.pendingWorkspace == null && state.pendingRevoke == null
        && state.transcriptTurn == null && !state.sessionHistoryLoading && SessionHistoryCodec.readable(state.projection)

    fun navigation(state: PairingState): Boolean = browse(state) && state.sessionHistoryOpen && state.sessionHistoryError == null
        && state.sessionHistory?.let { SessionHistoryCodec.sameScope(it, state.projection)
            && SessionHistoryCodec.validChain(state.sessionHistoryPrevious, it) } == true

    fun projection(state: PairingState, projection: HostProjection?): HostProjection? {
        if (state.transcriptWorker || state.transcriptHistoryTurn == null) return projection
        val page = state.sessionHistory ?: return null
        if (!state.sessionHistoryOpen || state.sessionHistoryError != null || !SessionHistoryCodec.sameScope(page, projection)
            || !SessionHistoryCodec.validChain(state.sessionHistoryPrevious, page)
            || page.turns.singleOrNull { it.id == state.transcriptHistoryTurn.id } != state.transcriptHistoryTurn) return null
        // This transient host-validated page authorizes reads only, never writes or cache commits.
        return projection?.copy(turns = page.turns)
    }
    fun dismissMessage(state: PairingState): PairingState {
        val valid = state.transcriptHistoryTurn != null && !state.transcriptWorker
            && state.transcriptTurn == state.transcriptHistoryTurn.id && projection(state, state.projection) != null
            && (state.transcriptToolCall == null || state.transcriptHistoryTurn.role == "assistant"
                && state.transcriptHistoryTurn.toolCalls.singleOrNull { it.id == state.transcriptToolCall.id } == state.transcriptToolCall)
        val cleared = state.copy(transcriptTurn = null, transcriptHistoryTurn = null, transcriptToolCall = null,
            transcriptPage = null, transcriptPreviousPages = emptyList(), transcriptLoading = false, transcriptError = null)
        return if (valid) cleared else cleared.copy(sessionHistoryOpen = false, sessionHistory = null,
            sessionHistoryPrevious = emptyList(), sessionHistoryLoading = false, sessionHistoryError = null)
    }
}
