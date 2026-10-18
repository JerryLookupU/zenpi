package zenpi.pairing

/** Destructive device controls cannot borrow authority while the scanner owns the UI. */
internal object DeviceActionPolicy {
    fun revoke(state: PairingState): Boolean = state.host != null && !state.scanInProgress
        && !state.resetting && !state.resetRequired && state.phase != PairingPhase.CONNECTING
        && state.pendingWorkspace == null && state.pendingRevoke == null

    fun checkRevocation(state: PairingState): Boolean = state.host != null && state.pendingRevoke != null
        && !state.scanInProgress && !state.resetting && !state.resetRequired
        && state.phase != PairingPhase.CONNECTING && state.pendingWorkspace == null

    fun forget(state: PairingState): Boolean = !state.scanInProgress && !state.resetting
        && (state.resetRequired || state.phase != PairingPhase.CONNECTING)
}
