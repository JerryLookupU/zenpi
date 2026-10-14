package zenpi.pairing

import zenpi.transport.ApprovalProjectionCodec
import zenpi.transport.RemoteApproval

internal object ApprovalDecisionPolicy {
    fun allows(state: PairingState, target: RemoteApproval): Boolean =
        state.phase in setOf(PairingPhase.CONNECTED, PairingPhase.ERROR)
            && !state.scanInProgress && !state.resetting && !state.resetRequired
            && state.pendingWorkspace == null && state.pendingRevoke == null
            && ApprovalProjectionCodec.current(state.projection, target)

    /** Approvals of the selected project shown to the user, in the host's queue order. */
    fun queue(state: PairingState): List<RemoteApproval> =
        if (state.phase !in setOf(PairingPhase.CONNECTED, PairingPhase.ERROR)) emptyList()
        else state.projection?.projects?.singleOrNull { it.id == state.projection.selectedId }?.pendingApprovals.orEmpty()

    /**
     * One tap decides the head of the queue, so approvals are answered strictly in queue order. [shown]
     * is the approval on screen when the tap (or the biometric confirmation that follows it) lands: a
     * decision is refused once the head moved on or its summary (digest, preview) changed underneath.
     */
    fun decides(state: PairingState, shown: RemoteApproval): Boolean =
        queue(state).firstOrNull() == shown && allows(state, shown)
}
