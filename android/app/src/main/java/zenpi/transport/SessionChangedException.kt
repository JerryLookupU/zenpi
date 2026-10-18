package zenpi.transport

/** An authenticated subscription found a different session in the same project. */
internal class SessionChangedException(val projectId: String, val previousSessionId: String) : WireException("Host session changed") {
    companion object {
        fun fromSubscription(request: com.google.gson.JsonObject, response: com.google.gson.JsonObject, readOnly: Boolean): SessionChangedException? {
            if (!readOnly || response["code"]?.asString != "session_changed"
                || FrameCodec.string(response, "execution_state") != "failed"
                || FrameCodec.string(request.getAsJsonObject("payload"), "type") != "subscribe") return null
            require(request["id"] == response["id"] && request["project_id"] == response["project_id"])
            return SessionChangedException(FrameCodec.string(request, "project_id"), FrameCodec.string(request, "session_id"))
        }
    }
}
