package zenpi.transport

import com.google.gson.JsonElement

data class RemoteApproval(val requestId: String, val projectId: String, val sessionId: String,
    val turnId: String, val callId: String, val tool: String, val sideEffect: String, val origin: String, val digest: String,
    val preview: RemoteApprovalPreview? = null, val workerSessionId: String? = null, val subtabId: String? = null)
data class RemoteApprovalPreview(val path: String, val patch: String, val changed: Boolean,
    val truncated: Boolean, val redacted: Boolean, val withheld: Boolean)

internal object ApprovalProjectionCodec {
    private fun identifier(value: String): Boolean = value.isNotBlank() && value.toByteArray(Charsets.UTF_8).size <= 128
        && value.none { Character.isISOControl(it) }
    private fun valid(target: RemoteApproval): Boolean = listOf(target.requestId, target.projectId, target.sessionId,
        target.turnId, target.callId, target.tool).all(::identifier)
        && target.sideEffect in setOf("read_only", "workspace_write", "command_execution")
        && target.origin in setOf("agent_tool", "user_shell", "blueprint_worker")
        && target.digest.matches(Regex("[0-9a-f]{64}"))
        && (if (target.workerSessionId == null) target.subtabId == null else identifier(target.workerSessionId)
            && target.subtabId?.matches(Regex("[0-9a-f]{64}")) == true)
        && target.preview?.let { view -> view.path.isNotBlank() && view.path.toByteArray(Charsets.UTF_8).size <= 4096
            && view.path.none { Character.isISOControl(it) } && view.patch.toByteArray(Charsets.UTF_8).size <= 2048
            && view.patch.none { Character.isISOControl(it) && it != '\n' && it != '\t' }
            && (!view.withheld || view.path == "<withheld>" && view.patch == "<withheld>" && view.redacted) } != false

    fun current(projection: HostProjection?, target: RemoteApproval): Boolean {
        if (!SessionCatalogCodec.readable(projection)) return false
        val project = projection?.projects?.singleOrNull { it.id == projection.selectedId } ?: return false
        return project.approvalControl && valid(target) && target.projectId == project.id && target.sessionId == project.sessionId
            && project.pendingApprovals.size <= 16 && project.pendingApprovals.all {
                valid(it) && it.projectId == project.id && it.sessionId == project.sessionId }
            && project.pendingApprovals.map { it.workerSessionId to it.requestId }.distinct().size == project.pendingApprovals.size
            && project.pendingApprovals.singleOrNull { it.requestId == target.requestId && it.workerSessionId == target.workerSessionId } == target
    }
    fun decode(value: JsonElement?, project: String, session: String): List<RemoteApproval> {
        if (value == null) return emptyList()
        if (!value.isJsonArray || value.asJsonArray.size() > 16) throw WireException("Approval projection limit")
        val fields = setOf("request_id", "project_id", "session_id", "turn_id", "call_id", "tool", "side_effect", "origin", "digest")
        return value.asJsonArray.map { element ->
            val workerFields = fields + setOf("worker_session_id", "subtab_id")
            if (!element.isJsonObject || element.asJsonObject.keySet() !in setOf(fields, fields + "preview", workerFields, workerFields + "preview")) throw WireException("Invalid approval projection fields")
            val item = element.asJsonObject
            fun identifier(name: String): String = FrameCodec.string(item, name).also {
                if (it.isBlank() || it.toByteArray().size > 128 || it.any { char -> Character.isISOControl(char) }) throw WireException("Invalid approval identity")
            }
            if (identifier("project_id") != project || identifier("session_id") != session) throw WireException("Approval owner mismatch")
            val worker = if (item.has("worker_session_id")) identifier("worker_session_id") else null
            val subtab = if (worker != null) identifier("subtab_id").also {
                if (!it.matches(Regex("[0-9a-f]{64}"))) throw WireException("Invalid approval subtab")
            } else null
            val effect = identifier("side_effect")
            if (effect !in setOf("read_only", "workspace_write", "command_execution")) throw WireException("Invalid approval effect")
            val origin = identifier("origin")
            if (origin !in setOf("agent_tool", "user_shell", "blueprint_worker")) throw WireException("Invalid approval origin")
            val digest = identifier("digest")
            if (!digest.matches(Regex("[0-9a-f]{64}"))) throw WireException("Invalid approval digest")
            val preview = item["preview"]?.let { raw ->
                if (!raw.isJsonObject || raw.asJsonObject.keySet() != setOf("kind", "path", "patch", "changed", "truncated", "redacted", "withheld")) throw WireException("Invalid approval preview fields")
                val view = raw.asJsonObject
                if (FrameCodec.string(view, "kind") != "diff") throw WireException("Invalid approval preview kind")
                fun flag(name: String): Boolean {
                    val flag = view[name]
                    if (!flag.isJsonPrimitive || !flag.asJsonPrimitive.isBoolean) throw WireException("Invalid approval preview flag")
                    return flag.asBoolean
                }
                val path = FrameCodec.string(view, "path")
                val patch = FrameCodec.string(view, "patch")
                if (path.isBlank() || path.toByteArray().size > 4096 || path.any { Character.isISOControl(it) }
                    || patch.toByteArray().size > 2048 || patch.any { Character.isISOControl(it) && it != '\n' && it != '\t' }) throw WireException("Invalid approval preview text")
                RemoteApprovalPreview(path, patch, flag("changed"), flag("truncated"), flag("redacted"), flag("withheld")).also {
                    if (it.withheld && (it.path != "<withheld>" || it.patch != "<withheld>" || !it.redacted)) throw WireException("Invalid withheld preview")
                }
            }
            RemoteApproval(identifier("request_id"), project, session, identifier("turn_id"), identifier("call_id"), identifier("tool"), effect, origin, digest, preview, worker, subtab)
                .also { if (!valid(it)) throw WireException("Invalid retained approval") }
        }.also { if (it.map { approval -> approval.workerSessionId to approval.requestId }.distinct().size != it.size) throw WireException("Duplicate approval identity") }
    }
}
