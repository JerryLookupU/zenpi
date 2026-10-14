package zenpi.transport

import com.google.gson.JsonObject
import java.math.BigInteger

internal object EventAckCodec {
    private fun sequence(value: String): BigInteger {
        if (!value.matches(Regex("0|[1-9][0-9]*"))) throw WireException("Invalid ACK sequence")
        return BigInteger(value).also {
            if (it > BigInteger("18446744073709551615")) throw WireException("Invalid ACK sequence")
        }
    }
    private fun identifier(value: String) {
        if (value.isEmpty() || value.toByteArray(Charsets.UTF_8).size > 128
            || value.any { Character.isISOControl(it) }) throw WireException("Invalid ACK scope")
    }
    fun request(project: String, session: String, sequence: String): JsonObject {
        identifier(project); identifier(session)
        val next = sequence(sequence)
        return JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control")
            addProperty("project_id", project); addProperty("session_id", session)
            add("payload", JsonObject().apply {
                addProperty("type", "ack")
                add("cursor", JsonObject().apply {
                    addProperty("session_id", session); addProperty("next_sequence", next)
                })
            })
        }
    }
    suspend fun acknowledge(request: JsonObject, device: String, project: String, session: String, sequence: String,
        exchange: suspend (JsonObject) -> JsonObject): Boolean {
        val expected = EventAckCodec.request(project, session, sequence)
        if (request.has("id")) identifier(FrameCodec.string(request, "id"))
        val actual = request.deepCopy().apply { remove("id") }
        if (actual != expected) throw WireException("ACK request scope mismatch")
        FrameCodec.encode(request)
        val confirmed = try { exchange(request) } catch (error: HostRefusedException) {
            // The owner can be executing while its read-only display is available.
            // Defer this persisted cursor to a later read; never invent an ACK receipt.
            if (error.code == "busy") return false
            throw error
        }
        validate(confirmed, device, project, session, sequence)
        return true
    }
    fun validate(value: JsonObject, device: String, project: String, session: String, sequence: String) {
        identifier(project); identifier(session)
        val expected = sequence(sequence)
        val cursor = value["cursor"]
        if (value.keySet() != setOf("device_id", "project_id", "cursor_scope", "cursor")
            || FrameCodec.string(value, "device_id") != device || FrameCodec.string(value, "project_id") != project
            || FrameCodec.string(value, "cursor_scope") != "transcript_events_v1"
            || cursor == null || !cursor.isJsonObject || cursor.asJsonObject.keySet() != setOf("session_id", "next_sequence")
            || FrameCodec.string(cursor.asJsonObject, "session_id") != session) throw WireException("ACK receipt mismatch")
        val next = cursor.asJsonObject["next_sequence"]
        if (next == null || !next.isJsonPrimitive || !next.asJsonPrimitive.isNumber
            || !next.toString().matches(Regex("0|[1-9][0-9]*")) || next.asBigInteger < expected
            || next.asBigInteger > BigInteger("18446744073709551615")) throw WireException("ACK receipt mismatch")
    }
}
