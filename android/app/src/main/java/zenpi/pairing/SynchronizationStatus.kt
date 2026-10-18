package zenpi.pairing

/** A live read observation, never offline authority or a persisted recovery claim. */
internal object SynchronizationStatus {
    fun label(state: PairingState): String? {
        if (state.phase != PairingPhase.CONNECTED || state.scanInProgress || state.resetting || state.resetRequired
            || state.pendingWorkspace != null || state.pendingRevoke != null) return null
        val projection = state.projection ?: return null
        return when {
            projection.hasMoreEvents -> "Catching up"
            projection.snapshotReplaced -> "Snapshot replaced"
            else -> null
        }
    }
}
