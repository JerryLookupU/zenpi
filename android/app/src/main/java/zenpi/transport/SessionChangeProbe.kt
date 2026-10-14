package zenpi.transport

import com.google.gson.JsonObject
import java.util.UUID

/** Reclassify only an automatic read race using a new authorized listing. */
internal object SessionChangeProbe {
    suspend fun resolve(request: JsonObject, error: HostRefusedException, automaticRead: Boolean,
        exchange: suspend (JsonObject) -> JsonObject): Exception {
        if (!automaticRead || error.code != "owner_mismatch") return error
        if (request.keySet() != setOf("schema_version", "channel", "id", "project_id", "session_id", "payload")
            || runCatching { FrameCodec.string(request, "id"); FrameCodec.encode(request) }.isFailure) return error
        val payload = request.getAsJsonObject("payload")
        val type = FrameCodec.string(payload, "type")
        if (type !in setOf("ack", "resume") && (type != "request"
                || payload["operation"]?.asString !in setOf("inspect", "subtab_list"))) return error
        if (type == "request" && payload.getAsJsonObject("arguments").size() != 0) return error
        val project = runCatching { FrameCodec.string(request, "project_id") }.getOrElse { return error }
        val session = runCatching { FrameCodec.string(request, "session_id") }.getOrElse { return error }
        val probe = JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            add("payload", JsonObject().apply {
                addProperty("type", "request"); addProperty("operation", "workspace_list"); add("arguments", JsonObject())
            })
        }
        val listing = exchange(probe)
        val projects = HostProjectionCodec.projects(listing)
        HostProjectionCodec.select(listing, projects, null)
        val current = projects.singleOrNull { it.id == project } ?: return error
        return if (current.sessionId != session) SessionChangedException(project, session) else error
    }
}
