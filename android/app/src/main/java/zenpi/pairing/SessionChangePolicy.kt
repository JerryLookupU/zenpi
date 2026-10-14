package zenpi.pairing

import zenpi.transport.SessionChangedException

internal object SessionChangePolicy {
    fun allows(state: PairingState, error: Exception, foreground: Boolean, network: Boolean): Boolean =
        error is SessionChangedException && foreground && network && !state.scanInProgress && !state.resetting && !state.resetRequired
            && state.pendingWorkspace == null && state.pendingRevoke == null && state.host != null
            && zenpi.transport.SessionCatalogCodec.readable(state.projection)
            && state.projection?.selectedId == error.projectId && state.projection.sessionId == error.previousSessionId

    fun replacement(state: PairingState): PairingState = AuthorityLossPolicy.displayOnly(state).copy(
        phase = PairingPhase.CONNECTING, projection = null, error = null)
}
