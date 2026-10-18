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

    /**
     * Rebuild every live text from the host's ordered deltas: keep `offset` code points of the
     * text this subscription already holds for the stream, then append `delta`. Full texts are
     * refused on a subscription and an offset past the held text is a gap, so a replayed or
     * reordered page can neither duplicate nor skip output.
     */
    private fun liveText(listing: JsonObject): JsonObject {
        val copy = listing.deepCopy()
        val projects = copy["projects"]?.takeIf { it.isJsonArray }?.asJsonArray ?: return copy
        for (element in projects) {
            val item = element.takeIf { it.isJsonObject }?.asJsonObject ?: continue
            val id = item["id"]?.takeIf { it.isJsonPrimitive }?.asString ?: continue
            val held = projection.projects.singleOrNull { it.id == id }
            merge(item, "desktop_stream", held?.desktopStreamText)
            for ((list, texts) in listOf("prompt_statuses" to held?.promptStatuses.orEmpty().associate { it.requestId to it.streamText },
                "worker_statuses" to held?.workerStatuses.orEmpty().associate { it.requestId to it.streamText })) {
                for (status in item[list]?.takeIf { it.isJsonArray }?.asJsonArray ?: continue) {
                    val value = status.takeIf { it.isJsonObject }?.asJsonObject ?: continue
                    val request = value["request_id"]?.takeIf { it.isJsonPrimitive }?.asString ?: continue
                    merge(value, "stream", texts[request])
                }
            }
        }
        return copy
    }

    private fun merge(item: JsonObject, prefix: String, held: String?) {
        if (item["${prefix}_text"]?.isJsonNull == false) throw WireException("Subscription live text must be a delta")
        val offset = item.remove("${prefix}_offset")
        val delta = item.remove("${prefix}_delta")
        if (offset == null && delta == null) return
        if (offset == null || delta == null || !offset.isJsonPrimitive || !offset.asJsonPrimitive.isNumber
            || !delta.isJsonPrimitive || !delta.asJsonPrimitive.isString) throw WireException("Invalid live text delta")
        val base = held.orEmpty()
        val keep = offset.asJsonPrimitive.asBigDecimal
        if (keep.signum() < 0 || keep.stripTrailingZeros().scale() > 0
            || keep > base.codePointCount(0, base.length).toBigDecimal()) throw WireException("Live text gap")
        item.addProperty("${prefix}_text", base.substring(0, base.offsetByCodePoints(0, keep.toInt())) + delta.asString)
    }
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
            val listing = liveText(view["listing"].takeIf { it.isJsonObject }?.asJsonObject ?: throw WireException("Invalid subscription listing"))
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
