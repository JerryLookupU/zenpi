package zenpi.pairing

import zenpi.transport.HostProjection
import zenpi.transport.RemoteWorkerStatus
import zenpi.transport.WorkerStatusPageCodec
import zenpi.transport.WorkerStatusCodec

data class WorkerReadScope internal constructor(val target: RemoteWorkerStatus, val owner: String?, val revision: String?)

/** A transient read selection is not a workspace capability or a cached task status. */
internal object WorkerReadPolicy {
    fun projection(state: PairingState, requestId: String, next: HostProjection? = state.projection): HostProjection? {
        val current = next ?: return null
        if (state.phase != PairingPhase.CONNECTED || state.scanInProgress || state.resetting || state.resetRequired
            || state.pendingWorkspace != null || state.pendingRevoke != null) return null
        if (!WorkerStatusPageCodec.readable(current)) return null
        val project = current.projects.single { it.id == current.selectedId }
        val scope = state.workerReadScope?.takeIf { it.target.requestId == requestId
            && (state.transcriptWorker && state.transcriptTurn == requestId || state.workerHistoryRequest == requestId) }
        if (scope != null && (scope.owner != current.workspaceOwner || scope.revision != current.workspaceRevision
                || scope.target.state != "succeeded" || !WorkerStatusCodec.validDisplay(scope.target, project.id, project.sessionId))) return null
        val summary = project.workerStatuses.filter { it.requestId == requestId }
        if (summary.isNotEmpty()) return current.takeIf { summary.singleOrNull()?.let {
            it.state == "succeeded" && WorkerStatusCodec.validDisplay(it, project.id, project.sessionId)
                && (scope == null || it.subtabId == scope.target.subtabId)
        } == true }
        val target = if (scope != null) {
            scope.target
        } else {
            if (!state.workerStatusesOpen || state.workerStatusesLoading || state.workerStatusesError != null) return null
            val page = state.workerStatusesPage ?: return null
            if (!WorkerStatusPageCodec.sameScope(page, current)
                || !WorkerStatusPageCodec.validChain(state.workerStatusesPrevious, page)
                || state.projection?.workspaceOwner != current.workspaceOwner || state.projection?.workspaceRevision != current.workspaceRevision) return null
            page.tasks.singleOrNull { it.requestId == requestId } ?: return null
        }
        if (target.state != "succeeded" || !WorkerStatusCodec.validDisplay(target, project.id, project.sessionId)) return null
        return current.copy(projects = current.projects.map {
            if (it.id == project.id) it.copy(workerStatuses = it.workerStatuses + target) else it
        })
    }
    fun scope(projection: HostProjection, requestId: String): WorkerReadScope {
        require(zenpi.transport.TranscriptPageCodec.retainsTurn(projection, projection, requestId, true))
        return WorkerReadScope(
            projection.projects.single { it.id == projection.selectedId }.workerStatuses.single { it.requestId == requestId && it.state == "succeeded" },
            projection.workspaceOwner, projection.workspaceRevision)
    }
}
