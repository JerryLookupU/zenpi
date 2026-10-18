package zenpi.pairing

import zenpi.transport.RemoteWorkerStatus
import zenpi.transport.WorkerStatusPageCodec
import zenpi.transport.WorkerStatusCodec

/** Fresh summary wins; a current page may supply a task outside the bounded summary. */
internal object WorkerTargetPolicy {
    fun cancellable(state: PairingState, requestId: String): RemoteWorkerStatus? {
        if (!PromptControlPolicy.live(state)) return null
        val projection = state.projection ?: return null
        val project = projection.projects.singleOrNull {
            it.id == projection.selectedId && it.sessionId == projection.sessionId && it.workerControl
        } ?: return null
        val summary = project.workerStatuses.filter { it.requestId == requestId }
        if (summary.isNotEmpty()) {
            return summary.singleOrNull()?.takeIf { WorkerStatusCodec.validDisplay(it, project.id, project.sessionId) && it.canCancel }
        }
        if (state.phase != PairingPhase.CONNECTED || !state.workerStatusesOpen
            || state.workerStatusesLoading || state.workerStatusesError != null) return null
        val page = state.workerStatusesPage ?: return null
        if (!WorkerStatusPageCodec.sameScope(page, projection)
            || !WorkerStatusPageCodec.validChain(state.workerStatusesPrevious, page)) return null
        return page.tasks.singleOrNull { it.requestId == requestId }?.takeIf { it.projectId == project.id
            && it.sessionId == project.sessionId && it.canCancel }
    }
}
