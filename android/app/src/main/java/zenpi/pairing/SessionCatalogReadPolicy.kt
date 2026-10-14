package zenpi.pairing

import zenpi.transport.SessionCatalogCodec

/** Catalog reads do not borrow the independent session-resume capability. */
internal object SessionCatalogReadPolicy {
    fun browse(state: PairingState): Boolean = state.phase == PairingPhase.CONNECTED
        && !state.scanInProgress && !state.resetting && !state.resetRequired && !state.sessionsLoading
        && state.pendingWorkspace == null && state.pendingRevoke == null
        && SessionCatalogCodec.readable(state.projection)

    fun navigation(state: PairingState): Boolean = browse(state) && state.sessionsOpen && state.sessionsError == null
        && state.sessions?.let { SessionCatalogCodec.sameScope(it, state.projection) && it.query == state.sessionsQuery
            && SessionCatalogCodec.validChain(state.sessionsPrevious, it) } == true
}
