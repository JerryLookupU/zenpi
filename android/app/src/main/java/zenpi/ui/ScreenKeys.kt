package zenpi.ui

import zenpi.pairing.PairingInvite
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.transport.RemoteProject
import zenpi.transport.RemoteSubtabs

/**
 * Values derived once per [PairingState] and shared by the screen sections. The scope
 * pairs are `remember` keys: UI-local dialog state is discarded whenever the host,
 * session or (for subtab dialogs) the subtab projection changes underneath it.
 */
class ScreenScope(val state: PairingState) {
    val hostScope: Pair<String, String>? = state.host?.let { it.fingerprint to it.deviceId }
    val sessionScope: Pair<String?, String?>? = state.projection?.let { it.selectedId to it.sessionId }
    val subtabScope: Pair<String, String>? = state.projection?.subtabs?.let { it.activeId to it.revision }
    val selected: RemoteProject? = state.projection?.projects?.singleOrNull { it.id == state.projection.selectedId }
    val mutationEnabled: Boolean = state.phase == PairingPhase.CONNECTED && !state.scanInProgress && state.pendingWorkspace == null
        && state.pendingRevoke == null && !state.resetting && !state.resetRequired
    val workspaceEnabled: Boolean = mutationEnabled && selected?.workspaceControl == true
    val currentSubtabs: RemoteSubtabs? = zenpi.transport.SubtabProjectionCodec.current(state.projection)
    val subtabEnabled: Boolean = workspaceEnabled && currentSubtabs?.control == true
    val draftKey: DraftKey = DraftKey.of(state)
}

/**
 * Identity of a composer draft: host, session and tab ids only. Revisions are
 * deliberately absent, so a remote rename, a concurrency change or a run finishing
 * never discards text the user is still typing. Steering text is addressed to one run,
 * so its draft is additionally scoped by that run's id ([DraftPolicy.steerKey]).
 */
data class DraftKey(val host: Pair<String, String>?, val projectId: String?, val sessionId: String?, val tabId: String?) {
    companion object {
        fun of(state: PairingState): DraftKey = DraftKey(
            state.host?.let { it.fingerprint to it.deviceId },
            state.projection?.selectedId, state.projection?.sessionId, state.projection?.subtabs?.activeId)
    }
}

/**
 * A *submitted* draft is cleared when the host acknowledges it, which is observed as a
 * change of the draft's commit marker (the active message id plus subtab revision for the
 * message composer, the subtab revision for the task composer). Text edited after the
 * submission, or never submitted, survives every marker change.
 */
object DraftPolicy {
    private val activeStates = setOf("accepted", "running")

    fun promptTarget(state: PairingState): String? = state.projection?.projects?.singleOrNull {
        it.id == state.projection.selectedId && it.sessionId == state.projection.sessionId
    }?.promptStatuses?.firstOrNull { it.state in activeStates }?.requestId

    /** Key of the steering draft: the message draft key plus the run being steered. */
    fun steerKey(state: PairingState): Any = DraftKey.of(state) to promptTarget(state)

    fun promptMarker(state: PairingState): Any? = state.projection?.subtabs?.revision to promptTarget(state)

    fun workerMarker(state: PairingState): Any? = state.projection?.subtabs?.revision

    /** The draft to show after the marker moved from [previous] to [current]. */
    fun settle(draft: String, submitted: String?, previous: Any?, current: Any?): String =
        if (previous != current && submitted != null && draft == submitted) "" else draft
}

/**
 * Drives the invitation countdown. It ticks once per second only while an invitation
 * is still redeemable; with no invitation (the normal connected state) it publishes one
 * reading and stops, so the screen does not recompose every second while idle.
 */
object InviteTicker {
    fun active(invite: PairingInvite?, now: Long): Boolean = invite != null && invite.remainingSeconds(now) > 0

    suspend fun run(active: (Long) -> Boolean, clock: () -> Long, sleep: suspend (Long) -> Unit, publish: (Long) -> Unit) {
        while (true) {
            val now = clock()
            publish(now)
            if (!active(now)) return
            sleep(1000)
        }
    }
}
