package zenpi.pairing

import zenpi.transport.ModelStatusCodec

/** Inspecting a model/catalog does not require the separate model-selection grant. */
internal object ModelReadPolicy {
    fun browse(state: PairingState): Boolean = state.phase == PairingPhase.CONNECTED && !state.scanInProgress
        && !state.resetting && !state.resetRequired && !state.modelLoading
        && state.pendingWorkspace == null && state.pendingRevoke == null
        && state.projection?.let(ModelStatusCodec::readable) == true

    fun navigation(state: PairingState): Boolean = browse(state) && state.modelOpen && state.modelError == null
        && state.modelStatus?.let { status ->
            ModelStatusCodec.sameScope(status, state.projection) && status.catalog != null
                && ModelStatusCodec.validChain(state.modelPrevious, status)
                && state.projection?.modelStatus?.let { current ->
                    ModelStatusCodec.sameScope(current, state.projection)
                        && current.selectionRevision == status.selectionRevision && current.registryBound == status.registryBound
                        && current.catalog?.snapshotId == status.catalog.snapshotId
                } != false
        } == true
}
