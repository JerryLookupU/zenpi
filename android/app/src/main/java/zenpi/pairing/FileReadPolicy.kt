package zenpi.pairing

import zenpi.transport.WorkspaceFilesCodec

/** File metadata navigation is separate from inserting a task attachment. */
internal object FileReadPolicy {
    fun browse(state: PairingState): Boolean = state.phase == PairingPhase.CONNECTED
        && !state.scanInProgress && !state.resetting && !state.resetRequired
        && state.pendingWorkspace == null && state.pendingRevoke == null && !state.filesLoading
        && state.projection?.let(WorkspaceFilesCodec::readable) == true

    fun navigation(state: PairingState): Boolean = browse(state) && state.filesError == null
        && state.files?.let { WorkspaceFilesCodec.matchesSelection(it, state.projection)
            && WorkspaceFilesCodec.validChain(state.filesPrevious, it) } == true

    fun attachment(state: PairingState): Boolean = navigation(state)
        && state.projection?.projects?.singleOrNull {
            it.id == state.projection.selectedId && it.sessionId == state.projection.sessionId
        }?.let { it.promptControl && it.promptStatuses.none { status -> status.state in setOf("accepted", "running") } } == true
}
