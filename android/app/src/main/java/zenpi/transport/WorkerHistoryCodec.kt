package zenpi.transport

import com.google.gson.JsonObject
import java.util.UUID

data class WorkerHistoryPage(val projectId: String, val sessionId: String, val requestId: String, val subtabId: String,
    val digest: String, val offset: Int, val nextOffset: Int, val totalTurns: Int, val hasMore: Boolean, val turns: List<RemoteTurn>)

internal object WorkerHistoryCodec {
    fun validPage(page: WorkerHistoryPage): Boolean = page.digest.matches(Regex("[0-9a-f]{64}"))
        && page.totalTurns in 1..1024 && page.offset in 0..page.totalTurns
        && page.nextOffset in page.offset..page.totalTurns && page.turns.size in 1..16
        && page.nextOffset - page.offset == page.turns.size && page.hasMore == (page.nextOffset < page.totalTurns)
        && page.turns.map { it.id }.distinct().size == page.turns.size
        && page.turns.all { it.id.isNotBlank() && it.id.toByteArray(Charsets.UTF_8).size <= 128 && it.id.none(Char::isISOControl)
            && it.role in setOf("system", "user", "assistant", "tool")
            && it.content.toByteArray(Charsets.UTF_8).size <= 2048 && !it.content.contains('\u0000')
            && ToolCallProjectionCodec.valid(it) }

    fun request(projection: HostProjection, requestId: String, previous: WorkerHistoryPage?): JsonObject {
        require(WorkerStatusPageCodec.readable(projection) && TranscriptPageCodec.retainsTurn(projection, projection, requestId, true))
        if (previous != null) require(validPage(previous) && previous.projectId == projection.selectedId && previous.sessionId == projection.sessionId
            && previous.requestId == requestId && previous.hasMore
            && projection.projects.singleOrNull { it.id == projection.selectedId }?.workerStatuses
                ?.singleOrNull { it.requestId == requestId && it.state == "succeeded" }?.subtabId == previous.subtabId)
        return JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            addProperty("project_id", projection.selectedId); addProperty("session_id", projection.sessionId)
            add("payload", JsonObject().apply {
                addProperty("type", "request"); addProperty("operation", "transcript_read")
                add("arguments", JsonObject().apply {
                    addProperty("request_id", requestId); addProperty("view", "history"); addProperty("offset", previous?.nextOffset ?: 0)
                    add("digest", previous?.digest?.let { com.google.gson.JsonPrimitive(it) } ?: com.google.gson.JsonNull.INSTANCE)
                })
            })
        }
    }
    fun decode(value: JsonObject, projection: HostProjection, requestId: String, previous: WorkerHistoryPage?): WorkerHistoryPage {
        request(projection, requestId, previous)
        if (!WorkerStatusPageCodec.readable(projection)) throw WireException("Worker history selection unavailable")
        val status = projection.projects.singleOrNull { it.id == projection.selectedId }?.workerStatuses?.singleOrNull { it.requestId == requestId && it.state == "succeeded" }
            ?: throw WireException("Worker task unavailable")
        if (value.keySet() != setOf("project_id", "session_id", "request_id", "subtab_id", "digest", "offset", "next_offset", "total_turns", "has_more", "turns")
            || FrameCodec.string(value, "project_id") != projection.selectedId || FrameCodec.string(value, "session_id") != projection.sessionId
            || FrameCodec.string(value, "request_id") != requestId || FrameCodec.string(value, "subtab_id") != status.subtabId) throw WireException("Worker history scope mismatch")
        fun number(name: String): Int = value[name].let {
            if (!it.isJsonPrimitive || !it.asJsonPrimitive.isNumber || !it.toString().matches(Regex("0|[1-9][0-9]*"))) throw WireException("Invalid history offset")
            it.toString().toIntOrNull()?.takeIf { n -> n in 0..1024 } ?: throw WireException("History limit")
        }
        val offset = number("offset"); val next = number("next_offset"); val total = number("total_turns")
        val digest = FrameCodec.string(value, "digest")
        val flag = value["has_more"]
        val items = value["turns"]
        if (!items.isJsonArray || items.asJsonArray.size() > 16) throw WireException("History page limit")
        val turns = items.asJsonArray.map { entry ->
            val fields = setOf("id", "role", "content", "content_truncated")
            if (!entry.isJsonObject || entry.asJsonObject.keySet() !in setOf(fields, fields + setOf("tool_calls", "tool_calls_truncated"))) throw WireException("Invalid history turn")
            val item = entry.asJsonObject
            val id = FrameCodec.string(item, "id"); val role = FrameCodec.string(item, "role"); val content = FrameCodec.string(item, "content")
            val truncated = item["content_truncated"]
            if (id.isBlank() || id.toByteArray().size > 128 || id.any { it.isISOControl() } || role !in setOf("system", "user", "assistant", "tool")
                || content.toByteArray().size > 2048 || content.contains('\u0000') || !truncated.isJsonPrimitive || !truncated.asJsonPrimitive.isBoolean) throw WireException("Invalid history turn")
            val calls = ToolCallProjectionCodec.decode(item, role)
            RemoteTurn(id, role, content, truncated.asBoolean, calls, item.get("tool_calls_truncated")?.asBoolean ?: false)
        }
        if (!digest.matches(Regex("[0-9a-f]{64}")) || !flag.isJsonPrimitive || !flag.asJsonPrimitive.isBoolean
            || offset != (previous?.nextOffset ?: 0) || next - offset != turns.size || next > total || total == 0
            || flag.asBoolean != (next < total) || flag.asBoolean && turns.isEmpty() || turns.map { it.id }.distinct().size != turns.size
            || previous != null && (previous.digest != digest || previous.totalTurns != total || previous.projectId != projection.selectedId
                || previous.sessionId != projection.sessionId || previous.requestId != requestId || previous.subtabId != status.subtabId || !previous.hasMore)) throw WireException("Worker history continuity mismatch")
        if (previous?.turns?.any { old -> turns.any { it.id == old.id } } == true) throw WireException("Repeated worker history turn")
        return WorkerHistoryPage(requireNotNull(projection.selectedId), requireNotNull(projection.sessionId), requestId, status.subtabId, digest, offset, next, total, flag.asBoolean, turns)
            .also { if (!validPage(it)) throw WireException("Invalid retained worker history") }
    }
}
