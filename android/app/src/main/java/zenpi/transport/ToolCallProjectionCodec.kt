package zenpi.transport

import com.google.gson.JsonObject

/** Bounded public invocation summary shared by host transcript and worker history. */
internal object ToolCallProjectionCodec {
    fun valid(turn: RemoteTurn): Boolean = turn.toolCalls.size <= 8
        && (turn.role == "assistant" || turn.toolCalls.isEmpty() && !turn.toolCallsTruncated)
        && (!turn.toolCallsTruncated || turn.toolCalls.size == 8)
        && turn.toolCalls.map { it.id }.distinct().size == turn.toolCalls.size
        && turn.toolCalls.all { call -> call.id.isNotBlank() && call.id.toByteArray(Charsets.UTF_8).size <= 128
            && call.id.none(Char::isISOControl) && call.name.isNotBlank() && call.name.toByteArray(Charsets.UTF_8).size <= 64
            && call.name.none(Char::isISOControl) && call.argumentsPreview.toByteArray(Charsets.UTF_8).size <= 512
            && !call.argumentsPreview.contains('\u0000') }

    fun decode(item: JsonObject, role: String): List<RemoteToolCall> {
        if (!item.has("tool_calls") && !item.has("tool_calls_truncated")) return emptyList()
        val values = item["tool_calls"]
        val more = item["tool_calls_truncated"]
        if (role != "assistant" || values == null || !values.isJsonArray || values.asJsonArray.size() > 8
            || more == null || !more.isJsonPrimitive || !more.asJsonPrimitive.isBoolean
            || more.asBoolean && values.asJsonArray.size() != 8) throw WireException("Invalid tool calls")
        return values.asJsonArray.map { call ->
            if (!call.isJsonObject || call.asJsonObject.keySet() != setOf("id", "name", "arguments_preview", "arguments_truncated"))
                throw WireException("Invalid tool call")
            val obj = call.asJsonObject
            val id = FrameCodec.string(obj, "id")
            val name = FrameCodec.string(obj, "name")
            val preview = FrameCodec.string(obj, "arguments_preview")
            val cut = obj["arguments_truncated"]
            if (id.isBlank() || id.toByteArray().size > 128 || id.any { it.isISOControl() }
                || name.isBlank() || name.toByteArray().size > 64 || name.any { it.isISOControl() }
                || preview.toByteArray().size > 512 || preview.contains('\u0000')
                || !cut.isJsonPrimitive || !cut.asJsonPrimitive.isBoolean) throw WireException("Invalid tool call")
            RemoteToolCall(id, name, preview, cut.asBoolean)
        }.also { if (it.map { call -> call.id }.distinct().size != it.size) throw WireException("Duplicate tool call") }
    }
}
