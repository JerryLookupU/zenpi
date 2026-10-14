package zenpi.pairing

import zenpi.transport.DiffProjectionCodec

/** Diff reads require current advertised scope, never prompt authority. */
internal object DiffReadPolicy {
    fun browse(state: PairingState): Boolean = state.phase == PairingPhase.CONNECTED
        && !state.scanInProgress && !state.resetting && !state.resetRequired && !state.diffLoading
        && state.pendingWorkspace == null && state.pendingRevoke == null
        && DiffProjectionCodec.readable(state.projection)

    fun navigation(state: PairingState): Boolean = browse(state) && state.diffOpen && state.diffError == null
        && state.diff?.let { DiffProjectionCodec.sameSelection(it, state.projection)
            && DiffProjectionCodec.validChain(state.diffPrevious, it) } == true
}
