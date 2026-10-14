package zenpi.transport

import com.google.gson.JsonObject

internal class SubscriptionReducer(initial: HostProjection, private val selected: RemoteProject) {
    var projection = initial
        private set
    var done = false
        private set
    private var framesSeen = 0
    private var bytesSeen = 0L
    private var pages = 0
    private var modelRevision = initial.modelStatus?.selectionRevision?.let { java.math.BigInteger(it) }
    fun accept(frames: List<JsonObject>, result: JsonObject): HostProjection {
        val finished = result["subscription_done"] ?: throw WireException("Missing subscription completion")
        if (done || !finished.isJsonPrimitive || !finished.asJsonPrimitive.isBoolean)
            throw WireException("Invalid subscription completion")
        val bytes = result.toString().toByteArray().size.toLong() + frames.sumOf { it.toString().toByteArray().size.toLong() }
        if (pages >= 32 || (pages == 31 && !finished.asBoolean)
            || framesSeen + frames.size > 128 || bytesSeen + bytes > 196_608)
            throw WireException("Subscription wire limit")
        var base = projection
        if (result.has("subscription_view")) {
            val view = result["subscription_view"].takeIf { it.isJsonObject }?.asJsonObject ?: throw WireException("Invalid subscription view")
            if (view.keySet() != setOf("listing", "subtabs") && view.keySet() != setOf("listing", "subtabs", "model_status")) throw WireException("Invalid subscription view fields")
            val listing = view["listing"].asJsonObject
            val refreshed = HostProjectionCodec.projects(listing)
            HostProjectionCodec.select(listing, refreshed, selected.id)
            val project = refreshed.singleOrNull { it.id == selected.id && it.sessionId == selected.sessionId }
                ?: throw WireException("Subscription selection changed")
            base = HostProjectionCodec.workspace(base.copy(projects = refreshed), listing)
                .copy(subtabs = SubtabProjectionCodec.decode(view["subtabs"].asJsonObject, project, listing))
            val model = view["model_status"]
            base = base.copy(modelStatus = if (model == null || model.isJsonNull) null else
                ModelStatusCodec.decode(model.takeIf { it.isJsonObject }?.asJsonObject ?: throw WireException("Invalid subscription model"), base))
        }
        val transcript = result["subscription_transcript"]
        if (transcript != null && (!transcript.isJsonPrimitive || !transcript.asJsonPrimitive.isBoolean))
            throw WireException("Invalid subscription transcript flag")
        val next = if (transcript?.asBoolean != false) HostProjectionCodec.replay(base, base.projects, selected, frames, result)
            .copy(workspaceOwner = base.workspaceOwner, workspaceRevision = base.workspaceRevision, subtabs = base.subtabs, modelStatus = base.modelStatus)
        else {
            if (projection.selectedId != selected.id || projection.sessionId != selected.sessionId
                || result.has("phase") || result.has("journal_next_sequence"))
                throw WireException("Transient subscription transcript scope mismatch")
            if (FrameCodec.string(result, "cursor_scope") != "transcript_events_v1")
                throw WireException("Transient cursor scope mismatch")
            val cursor = result["cursor"]?.takeIf { it.isJsonObject }?.asJsonObject ?: throw WireException("Missing transient cursor")
            val sequence = cursor["next_sequence"]
            if (frames.isNotEmpty() || cursor.keySet() != setOf("session_id", "next_sequence")
                || FrameCodec.string(cursor, "session_id") != selected.sessionId || sequence == null
                || !sequence.isJsonPrimitive || !sequence.asJsonPrimitive.isNumber || sequence.toString() != projection.eventSequence
                || result["snapshot_required"].toString() != "false" || result["has_more"].toString() != "false")
                throw WireException("Transient subscription advanced transcript")
            base
        }
        if (!finished.asBoolean && (next.hasMoreEvents || result["snapshot_required"].asBoolean))
            throw WireException("Subscription must finish at replay boundary")
        zenpi.state.ProjectionCommitPolicy.requireCurrent(projection, next)
        val revision = next.modelStatus?.selectionRevision?.let { java.math.BigInteger(it) }
        if (revision != null && modelRevision?.let { revision < it } == true) throw zenpi.state.StaleProjectionException()
        if (revision != null) modelRevision = revision
        framesSeen += frames.size; bytesSeen += bytes; pages++
        done = finished.asBoolean; projection = next
        return next
    }
}
