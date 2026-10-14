package zenpi.transport

import com.google.gson.JsonObject
import java.util.UUID

data class TranscriptPage(val projectId: String, val sessionId: String, val turnId: String, val role: String,
    val digest: String, val offset: Int, val nextOffset: Int, val totalBytes: Int, val hasMore: Boolean, val text: String,
    val workerRequestId: String? = null, val subtabId: String? = null, val toolCallId: String? = null, val toolName: String? = null,
    val journalRevision: String? = null)

internal object TranscriptPageCodec {
    private fun identifier(value: String): Boolean = value.isNotBlank()
        && value.toByteArray(Charsets.UTF_8).size <= 128 && value.none(Char::isISOControl)
    fun retainsTurn(previous: HostProjection, next: HostProjection?, turnId: String, worker: Boolean = false): Boolean {
        if (!SessionCatalogCodec.readable(previous) || !SessionCatalogCodec.readable(next)) return false
        if (worker) {
            val status = previous.projects.singleOrNull { it.id == previous.selectedId }?.workerStatuses?.singleOrNull { it.requestId == turnId } ?: return false
            if (status.state != "succeeded" || !WorkerStatusCodec.validDisplay(status, requireNotNull(previous.selectedId), requireNotNull(previous.sessionId))) return false
            return next != null && previous.selectedId == next.selectedId && previous.sessionId == next.sessionId
                && next.projects.singleOrNull { it.id == next.selectedId }?.workerStatuses?.singleOrNull { it.requestId == turnId } == status
        }
        val turn = previous.turns.singleOrNull { it.id == turnId } ?: return false
        return ToolCallProjectionCodec.valid(turn) && next != null && previous.selectedId == next.selectedId && previous.sessionId == next.sessionId
            && next.turns.singleOrNull { it.id == turnId } == turn
    }
    fun request(projection: HostProjection, turnId: String, previous: TranscriptPage?, worker: Boolean = false, historyTurn: RemoteTurn? = null, toolCall: RemoteToolCall? = null,
        journalRevision: String? = null): JsonObject {
        require(projection.selectedId != null && projection.sessionId != null && retainsTurn(projection, projection, turnId, worker))
        require(SessionCatalogCodec.readable(projection))
        require(journalRevision == null || !worker && journalRevision == projection.journalSequence
            && journalRevision.matches(Regex("0|[1-9][0-9]*")) && java.math.BigInteger(journalRevision) <= java.math.BigInteger("18446744073709551615"))
        require(previous == null || previous.journalRevision == journalRevision)
        if (previous != null) require(previous.projectId == projection.selectedId && previous.sessionId == projection.sessionId
            && (if (worker) previous.workerRequestId == turnId else previous.turnId == turnId && previous.workerRequestId == null
                && previous.subtabId == null) && previous.hasMore && TranscriptNavigation.validChain(emptyList(), previous))
        if (worker && previous != null) require(projection.projects.singleOrNull { it.id == projection.selectedId }
            ?.workerStatuses?.singleOrNull { it.requestId == turnId && it.state == "succeeded" }?.subtabId == previous.subtabId)
        if (historyTurn != null && previous != null) require(previous.turnId == historyTurn.id && previous.role == historyTurn.role)
        val selectedTurn = if (worker) historyTurn else projection.turns.singleOrNull { it.id == turnId }
        if (selectedTurn != null) require(identifier(selectedTurn.id) && ToolCallProjectionCodec.valid(selectedTurn))
        if (previous != null) require(previous.role == (selectedTurn?.role ?: "assistant"))
        if (toolCall != null) require(selectedTurn?.role == "assistant" && selectedTurn.toolCalls.singleOrNull { it.id == toolCall.id } == toolCall)
        if (previous != null) require(previous.toolCallId == toolCall?.id && previous.toolName == toolCall?.name)
        return JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            addProperty("project_id", projection.selectedId); addProperty("session_id", projection.sessionId)
            add("payload", JsonObject().apply {
                addProperty("type", "request"); addProperty("operation", "transcript_read")
                add("arguments", JsonObject().apply {
                    addProperty(if (worker) "request_id" else "turn_id", turnId); addProperty("offset", previous?.nextOffset ?: 0)
                    if (historyTurn != null) { require(worker); addProperty("turn_id", historyTurn.id) }
                    if (toolCall != null) addProperty("tool_call_id", toolCall.id)
                    journalRevision?.let { addProperty("journal_revision", java.math.BigInteger(it)) }
                    add("digest", previous?.digest?.let { com.google.gson.JsonPrimitive(it) } ?: com.google.gson.JsonNull.INSTANCE)
                })
            })
        }
    }
    fun decode(value: JsonObject, projection: HostProjection, turnId: String, previous: TranscriptPage?, worker: Boolean = false, historyTurn: RemoteTurn? = null, toolCall: RemoteToolCall? = null,
        journalRevision: String? = null): TranscriptPage {
        request(projection, turnId, previous, worker, historyTurn, toolCall, journalRevision)
        if (journalRevision != null) {
            val revision = value["journal_revision"]
            require(revision != null && revision.isJsonPrimitive && revision.asJsonPrimitive.isNumber && revision.toString() == journalRevision)
        }
        val fields = setOf("project_id", "session_id", "turn_id", "role", "digest", "offset", "next_offset", "total_bytes", "has_more", "text")
        val actualTurn = FrameCodec.string(value, "turn_id")
        val subtab = if (worker) projection.projects.singleOrNull { it.id == projection.selectedId }?.workerStatuses?.singleOrNull { it.requestId == turnId && it.state == "succeeded" }?.subtabId else null
        val selectedTurn = if (worker) historyTurn else projection.turns.singleOrNull { it.id == turnId }
        if (toolCall != null && (selectedTurn?.role != "assistant" || selectedTurn.toolCalls.singleOrNull { it.id == toolCall.id } != toolCall
            || FrameCodec.string(value, "tool_call_id") != toolCall.id || FrameCodec.string(value, "tool_name") != toolCall.name)) throw WireException("Tool argument scope mismatch")
        if (!worker && historyTurn != null || value.keySet() != fields + (if (worker) setOf("request_id", "subtab_id") else emptySet())
            + (if (toolCall != null) setOf("tool_call_id", "tool_name") else emptySet())
            + (if (journalRevision != null) setOf("journal_revision") else emptySet())
            || FrameCodec.string(value, "project_id") != projection.selectedId || FrameCodec.string(value, "session_id") != projection.sessionId
            || (if (worker) subtab == null || FrameCodec.string(value, "request_id") != turnId || FrameCodec.string(value, "subtab_id") != subtab
                || !identifier(actualTurn) || historyTurn != null && actualTurn != historyTurn.id else actualTurn != turnId)) throw WireException("Transcript page scope mismatch")
        fun number(name: String): Int {
            val n = value[name]
            if (!n.isJsonPrimitive || !n.asJsonPrimitive.isNumber || !n.toString().matches(Regex("0|[1-9][0-9]*"))) throw WireException("Invalid transcript offset")
            return n.toString().toIntOrNull()?.takeIf { it in 0..(if (toolCall != null) 64 * 1024 else 4 * 1024 * 1024) } ?: throw WireException("Transcript size limit")
        }
        val offset = number("offset"); val next = number("next_offset"); val total = number("total_bytes")
        val text = FrameCodec.string(value, "text"); val bytes = text.toByteArray(Charsets.UTF_8).size
        val role = FrameCodec.string(value, "role"); val digest = FrameCodec.string(value, "digest")
        if ((if (worker) historyTurn?.role ?: "assistant" else projection.turns.singleOrNull { it.id == turnId }?.role) != role) throw WireException("Transcript turn mismatch")
        val flag = value["has_more"]
        if (!flag.isJsonPrimitive || !flag.asJsonPrimitive.isBoolean || !digest.matches(Regex("[0-9a-f]{64}"))
            || role !in setOf("system", "user", "assistant", "tool") || text.contains('\u0000') || bytes > 8192 || next - offset != bytes
            || next > total || offset != (previous?.nextOffset ?: 0) || flag.asBoolean != (next < total)
            || flag.asBoolean && bytes == 0 || previous != null && (previous.digest != digest || previous.totalBytes != total || previous.role != role
                || previous.projectId != projection.selectedId || previous.sessionId != projection.sessionId || previous.turnId != actualTurn
                || previous.workerRequestId != (if (worker) turnId else null) || previous.subtabId != subtab
                || previous.toolCallId != toolCall?.id || previous.toolName != toolCall?.name || !previous.hasMore))
            throw WireException("Transcript page mismatch")
        return TranscriptPage(requireNotNull(projection.selectedId), requireNotNull(projection.sessionId), actualTurn, role, digest, offset, next, total, flag.asBoolean, text, if (worker) turnId else null, subtab, toolCall?.id, toolCall?.name, journalRevision)
    }
}
