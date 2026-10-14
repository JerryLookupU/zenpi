package zenpi.pairing

/** Stop the connection, not the durable pairing or an unresolved request. */
internal object ConnectionCancelPolicy {
    fun stopped(state: PairingState): PairingState {
        if (state.host == null && state.pendingWorkspace == null && state.pendingRevoke == null) return PairingState()
        return AuthorityLossPolicy.displayOnly(state).copy(phase = PairingPhase.OFFLINE, invite = null, scanError = null,
            error = when {
                state.pendingRevoke != null -> "Host revocation unconfirmed. Check the desktop before forgetting."
                state.pendingWorkspace != null -> "Workspace result unknown. Retry the same request to resolve it."
                else -> null
            })
    }
}
