package zenpi.pairing

import zenpi.transport.WorkerStatusPageCodec

/** Task metadata remains readable without the independent task-control grant. */
internal object WorkerStatusReadPolicy {
    fun browse(state: PairingState): Boolean = state.phase == PairingPhase.CONNECTED && !state.scanInProgress
        && !state.resetting && !state.resetRequired && state.pendingWorkspace == null && state.pendingRevoke == null
        && !state.workerStatusesLoading && state.projection?.let(WorkerStatusPageCodec::readable) == true

    fun navigation(state: PairingState): Boolean = browse(state) && state.workerStatusesOpen
        && state.workerStatusesError == null && state.workerStatusesPage?.let {
            WorkerStatusPageCodec.sameScope(it, state.projection)
                && WorkerStatusPageCodec.validChain(state.workerStatusesPrevious, it)
        } == true
}
