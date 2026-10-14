package zenpi.pairing

import zenpi.transport.ResourceCatalogCodec

/** Catalog metadata reads do not borrow prompt or reload authority. */
internal object ResourceReadPolicy {
    fun browse(state: PairingState): Boolean = state.phase == PairingPhase.CONNECTED
        && !state.scanInProgress && !state.resetting && !state.resetRequired
        && state.pendingWorkspace == null && state.pendingRevoke == null && !state.resourcesLoading
        && state.projection?.let(ResourceCatalogCodec::readable) == true

    fun navigation(state: PairingState): Boolean = browse(state) && state.resourcesError == null
        && state.resources?.let { ResourceCatalogCodec.sameScope(it, state.projection)
            && ResourceCatalogCodec.validChain(state.resourcesPrevious, it) } == true

    fun insertion(state: PairingState): Boolean = navigation(state)
        && state.projection?.projects?.singleOrNull {
            it.id == state.projection.selectedId && it.sessionId == state.projection.sessionId
        }?.let { it.promptControl && it.promptStatuses.none { status -> status.state in setOf("accepted", "running") } } == true
}
