package zenpi.transport

data class RemotePromptStatus(val requestId: String, val projectId: String, val sessionId: String, val state: String, val streamText: String? = null, val streamTruncated: Boolean = false, val failureCode: String? = null)

internal object PromptStatusCodec {
    private fun valid(status: RemotePromptStatus, project: String, session: String): Boolean =
        status.projectId == project && status.sessionId == session
            && status.requestId.isNotEmpty() && status.requestId.toByteArray(Charsets.UTF_8).size <= 128
            && status.requestId.none(Char::isISOControl) && status.state in RemoteSchema.EXECUTION_STATES
            && (status.failureCode == null || status.state == "failed"
                && status.failureCode in RemoteSchema.ERROR_CODES - "session_changed")
            && (status.streamText == null || status.state in setOf("accepted", "running")
                && status.streamText.toByteArray(Charsets.UTF_8).size <= 4096 && !status.streamText.contains('\u0000'))
            && (!status.streamTruncated || status.streamText != null)

    fun active(projection: HostProjection?, requestId: String): RemotePromptStatus? {
        if (!SessionCatalogCodec.readable(projection)) return null
        val project = projection?.projects?.singleOrNull { it.id == projection.selectedId } ?: return null
        if (!project.promptControl) return null
        return project.promptStatuses.singleOrNull { it.requestId == requestId }?.takeIf {
            valid(it, project.id, project.sessionId) && it.state in setOf("accepted", "running")
        }
    }
    fun decode(value: com.google.gson.JsonElement?, project: String, session: String): List<RemotePromptStatus> {
        if (value == null) return emptyList()
        if (!value.isJsonArray || value.asJsonArray.size() > 16) throw WireException("Prompt status limit")
        return value.asJsonArray.map { element ->
            if (!element.isJsonObject) throw WireException("Invalid prompt status")
            val item = element.asJsonObject
            val fields = setOf("request_id", "project_id", "session_id", "state")
            val displayFields=item.keySet()-"failure_code"
            if (displayFields != fields && displayFields != fields + "stream_text"
                && displayFields != fields + setOf("stream_text", "stream_truncated")
                || FrameCodec.string(item, "project_id") != project || FrameCodec.string(item, "session_id") != session)
                throw WireException("Prompt status owner mismatch")
            val id = FrameCodec.string(item, "request_id")
            if (id.isEmpty() || id.toByteArray().size > 128 || id.any { Character.isISOControl(it) }) throw WireException("Invalid prompt status ID")
            val state = FrameCodec.string(item, "state")
            if (state !in setOf("accepted", "running", "succeeded", "failed", "cancelled", "unknown_outcome")) throw WireException("Invalid prompt status state")
            val failure=if(item.has("failure_code"))FrameCodec.string(item,"failure_code").also {
                if(state!="failed" || it !in setOf("invalid_request","unsupported_version","unauthorized","revoked_device","owner_mismatch","revision_conflict",
                    "request_id_conflict","busy","replay_gap","resource_limit","internal_error"))throw WireException("Invalid prompt failure")
            }else null
            val stream = if (item.has("stream_text")) FrameCodec.string(item, "stream_text").also {
                if (state !in setOf("accepted", "running") || it.toByteArray().size > 4096 || it.contains('\u0000')) throw WireException("Invalid live prompt text")
            } else null
            val truncated = if (item.has("stream_truncated")) item["stream_truncated"].let {
                if (!it.isJsonPrimitive || !it.asJsonPrimitive.isBoolean || !it.asBoolean || stream == null)
                    throw WireException("Invalid live prompt truncation")
                true
            } else false
            RemotePromptStatus(id, project, session, state, stream, truncated,failure)
        }.also {
            if (it.map { status -> status.requestId }.distinct().size != it.size) throw WireException("Duplicate prompt status")
            if (!it.all { status -> valid(status, project, session) }) throw WireException("Invalid retained prompt status")
        }
    }
}
