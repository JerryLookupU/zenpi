package zenpi.pairing

import zenpi.transport.ApprovalProjectionCodec
import zenpi.transport.RemoteApproval

internal object ApprovalDecisionPolicy {
    fun allows(state: PairingState, target: RemoteApproval): Boolean =
        state.phase in setOf(PairingPhase.CONNECTED, PairingPhase.ERROR)
            && !state.scanInProgress && !state.resetting && !state.resetRequired
            && state.pendingWorkspace == null && state.pendingRevoke == null
            && ApprovalProjectionCodec.current(state.projection, target)
}
