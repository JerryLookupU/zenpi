package zenpi.security

import zenpi.state.ProjectionCacheCodec

internal object RePairPolicy {
    fun state(saved: SavedHostState?, host: PairedHost): SavedHostState {
        check(saved?.pendingWorkspace == null && saved?.pendingRevoke == null) {
            "Resolve or forget the pending request before pairing again"
        }
        val sameIdentity = saved?.host?.fingerprint == host.fingerprint && saved.host.deviceId == host.deviceId
        // Keep display/cursors, never cached grants, live previews or owner authority.
        val display = saved?.projection?.takeIf { sameIdentity && it.selectedId != null }
            ?.let { ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(it)) }
        return SavedHostState(host, display)
    }
}
