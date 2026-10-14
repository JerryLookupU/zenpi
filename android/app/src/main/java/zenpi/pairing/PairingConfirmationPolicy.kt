package zenpi.pairing

/** Confirmation belongs to a fresh invitation, never to saved host authority. */
internal object PairingConfirmationPolicy {
    fun allows(state: PairingState, now: Long = System.currentTimeMillis() / 1000): Boolean {
        val invite = state.invite ?: return false
        return state.phase == PairingPhase.CONFIRMING && !state.scanInProgress && !state.resetting && !state.resetRequired
            && state.pendingWorkspace == null && state.pendingRevoke == null
            && runCatching { invite.requireFresh(now) }.isSuccess
    }
}
