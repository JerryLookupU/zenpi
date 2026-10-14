package zenpi.pairing

import zenpi.transport.*

internal object PromptControlPolicy {
    fun live(state: PairingState): Boolean = state.phase in setOf(PairingPhase.CONNECTED, PairingPhase.ERROR)
        && !state.scanInProgress && !state.resetting && !state.resetRequired
        && state.pendingWorkspace == null && state.pendingRevoke == null && SessionCatalogCodec.readable(state.projection)
    fun send(state: PairingState): Boolean = live(state) && state.phase == PairingPhase.CONNECTED
        && state.projection?.projects?.singleOrNull { it.id == state.projection.selectedId }?.promptControl == true
    fun active(state: PairingState, requestId: String): Boolean = live(state) && PromptStatusCodec.active(state.projection, requestId) != null
    fun workerSend(state: PairingState): Boolean = live(state) && state.phase == PairingPhase.CONNECTED
        && state.projection?.let { projection ->
            projection.projects.singleOrNull { it.id == projection.selectedId }?.workerControl == true
                && SubtabProjectionCodec.current(projection) != null
        } == true
}
