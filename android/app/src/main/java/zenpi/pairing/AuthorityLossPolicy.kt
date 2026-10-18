package zenpi.pairing

import zenpi.state.ProjectionCacheCodec
import zenpi.transport.HostRefusedException
import zenpi.transport.WorkspaceRefusedException
import zenpi.transport.SelectedProjectUnavailableException

internal object AuthorityLossPolicy {
    fun refused(error: Exception): Boolean {
        if (zenpi.transport.HostIdentityFailure.refused(error)) return true
        if (zenpi.transport.HostNegotiationException.refused(error)) return true
        if (zenpi.transport.HostAuthenticationException.refused(error)) return true
        if (error is SelectedProjectUnavailableException) return true
        val code = when (error) {
            is HostRefusedException -> error.code
            is WorkspaceRefusedException -> error.code
            is zenpi.transport.WorkspaceUnknownOutcomeException -> error.code
            else -> null
        }
        return code in setOf("unauthorized", "revoked_device", "owner_mismatch")
    }
    fun displayOnly(state: PairingState): PairingState = state.copy(
        projection = state.projection?.takeIf { it.selectedId != null }
            ?.let { runCatching { ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(it)) }.getOrNull() },
        diff = null, diffPrevious = emptyList(), diffLoading = false, diffError = null, diffOpen = false,
        sessionsOpen = false, sessions = null, sessionsPrevious = emptyList(), sessionsLoading = false, sessionsError = null, sessionsQuery = null,
        sessionHistoryOpen = false, sessionHistory = null, sessionHistoryPrevious = emptyList(), sessionHistoryLoading = false, sessionHistoryError = null,
        terminalOpen = false, terminalPage = null, terminalPrevious = emptyList(), terminalLoading = false, terminalError = null,
        transcriptTurn = null, transcriptWorker = false, transcriptHistoryTurn = null, transcriptToolCall = null, transcriptPage = null, transcriptPreviousPages = emptyList(), transcriptLoading = false, transcriptError = null,
        workerHistoryRequest = null, workerReadScope = null, workerHistoryPage = null, workerHistoryPrevious = emptyList(), workerHistoryLoading = false, workerHistoryError = null,
        workerStatusesOpen = false, workerStatusesPage = null, workerStatusesPrevious = emptyList(), workerStatusesLoading = false, workerStatusesError = null,
        filesOpen = false, files = null, filesPrevious = emptyList(), filesLoading = false, filesError = null,
        resourcesOpen = false, resources = null, resourcesPrevious = emptyList(), resourcesLoading = false, resourcesError = null,
        modelOpen = false, modelStatus = null, modelPrevious = emptyList(), modelLoading = false, modelError = null)
}
