package zenpi.state

import java.math.BigInteger
import zenpi.transport.HostProjection
import zenpi.transport.WireException

internal class StaleProjectionException : WireException("Stale cache projection")

internal object ProjectionCommitPolicy {
    fun requireCurrent(previous: HostProjection?, next: HostProjection) {
        if (previous == null) return
        // Workspace revisions belong to the host owner, not the selected session.
        if (previous.workspaceOwner != null && previous.workspaceOwner == next.workspaceOwner
            && previous.workspaceRevision != null && next.workspaceRevision != null
            && unsigned(next.workspaceRevision) < unsigned(previous.workspaceRevision)) throw StaleProjectionException()
        if (previous.selectedId != next.selectedId || previous.sessionId != next.sessionId) return
        if (previous.modelStatus != null && next.modelStatus != null
            && unsigned(next.modelStatus.selectionRevision) < unsigned(previous.modelStatus.selectionRevision)) throw StaleProjectionException()
        previous.subtabs?.let { old ->
            val current = next.subtabs ?: throw StaleProjectionException()
            if (old.projectId != previous.selectedId || current.projectId != next.selectedId)
                throw WireException("Cache subtab project mismatch")
            if (unsigned(current.revision) < unsigned(old.revision)) throw StaleProjectionException()
        }
        previous.journalSequence?.let { value ->
            val oldJournal = unsigned(value)
            val newJournal = next.journalSequence?.let(::unsigned) ?: throw WireException("Missing cache journal sequence")
            if (newJournal < oldJournal) throw StaleProjectionException()
        }
        previous.eventSequence?.let { value ->
            val oldEvent = unsigned(value)
            val newEvent = next.eventSequence?.let(::unsigned) ?: throw WireException("Missing cache event sequence")
            if (newEvent < oldEvent) throw StaleProjectionException()
        }
    }
    private fun unsigned(value: String): BigInteger {
        if (!value.matches(Regex("0|[1-9][0-9]*"))) throw WireException("Invalid cache sequence")
        return BigInteger(value).also {
            if (it > BigInteger("18446744073709551615")) throw WireException("Invalid cache sequence")
        }
    }
}
