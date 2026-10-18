package zenpi.lifecycle

import zenpi.transport.HostProjection
import zenpi.transport.SessionChangedException

/** One fresh snapshot after a verified session race; never retry a mutation. */
internal object ResumeReadPolicy {
    suspend fun read(previous: HostProjection?, inspect: suspend (HostProjection?) -> HostProjection): HostProjection {
        return try { inspect(previous) }
        catch (changed: SessionChangedException) {
            if (previous == null || previous.selectedId != changed.projectId || previous.sessionId != changed.previousSessionId) throw changed
            inspect(null)
        }
    }
}
