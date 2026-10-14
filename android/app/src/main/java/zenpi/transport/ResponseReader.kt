package zenpi.transport

import com.google.gson.JsonObject
import java.io.InputStream

internal data class ResponsePage(val frames: List<JsonObject>, val result: JsonObject)

/** Framing and correlation only. The caller validates outcomes before any page is published. */
internal object ResponseReader {
    suspend fun read(input: InputStream, request: JsonObject,
        validateResponse: (JsonObject) -> Unit,
        onSubscriptionPage: (suspend (ResponsePage) -> Unit)? = null): ResponsePage {
        val id = FrameCodec.string(request, "id")
        val operation = FrameCodec.string(request["payload"].asJsonObject, "type")
        fun refuse(message: String): Nothing {
            if (operation == "authenticate") throw HostAuthenticationException()
            throw WireException(message)
        }
        var pages = 0
        var subscriptionBytes = 0L
        var frameLength = 0
        fun readFrame(): JsonObject? = try {
            FrameCodec.read(input) { length ->
                frameLength = length
                if (operation == "subscribe") {
                    subscriptionBytes += length
                    if (subscriptionBytes > 196_608) throw WireException("Subscription wire limit")
                }
            }
        } catch (error: WireException) {
            if (operation == "authenticate") throw HostAuthenticationException()
            throw error
        }
        do {
            val frames = mutableListOf<JsonObject>()
            var frameBytes = 0L
            var response = readFrame() ?: throw WireException("Host disconnected")
            while (FrameCodec.string(response["payload"].asJsonObject, "type") in setOf("event", "snapshot")) {
                if (operation !in setOf("resume", "subscribe")) refuse("Unexpected event on control request")
                frameBytes += frameLength
                if (frames.size >= 128 || frameBytes > 196_608) throw WireException("Replay wire limit")
                frames.add(response)
                response = readFrame() ?: throw WireException("Incomplete replay")
            }
            if (response.keySet().any { it !in setOf("schema_version", "channel", "id", "session_id", "project_id", "code", "execution_state", "payload") }
                || FrameCodec.string(response, "channel") != "control"
                || FrameCodec.string(response["payload"].asJsonObject, "type") != "response"
                || FrameCodec.string(response, "id") != id)
                refuse("Host response mismatch")
            if (FrameCodec.string(response, "execution_state") != "failed" && response["code"]?.isJsonNull == false
                && !WorkspaceOutcomeCodec.unknownWrite(request, response))
                refuse("Unexpected host response code")
            // Routing is optional on the response envelope for legacy/read-only
            // operations, but when present it may never contradict the request
            // (or the scoped result).  Do not reject the host's project/session
            // routing fields merely because older fixtures omitted them.
            val resultElement = response["payload"].asJsonObject["result"]
            for (field in listOf("project_id", "session_id")) {
                val routed = response[field]
                if (routed != null && !routed.isJsonNull) {
                    val requestValue = request[field]
                    if (requestValue != null && !requestValue.isJsonNull
                        && FrameCodec.string(response, field) != FrameCodec.string(request, field))
                        refuse("Host response $field mismatch")
                    val resultValue = resultElement?.takeIf { it.isJsonObject }?.asJsonObject?.get(field)
                    if (resultValue != null && !resultValue.isJsonNull && resultValue.isJsonPrimitive
                        && resultValue.asJsonPrimitive.isString
                        && FrameCodec.string(response, field) != resultValue.asString)
                        refuse("Host response $field scope mismatch")
                }
            }
            validateResponse(response)
            val result = resultElement?.takeIf { it.isJsonObject }?.asJsonObject
                ?: refuse("Host response result missing")
            val page = ResponsePage(frames.toList(), result)
            if (operation != "subscribe") return page
            val done = result["subscription_done"] ?: throw WireException("Missing subscription completion")
            if (!done.isJsonPrimitive || !done.asJsonPrimitive.isBoolean) throw WireException("Invalid subscription completion")
            if (++pages > 32 || (pages == 32 && !done.asBoolean)) throw WireException("Subscription wire limit")
            val publish = onSubscriptionPage ?: throw WireException("Missing subscription consumer")
            publish(page)
            if (done.asBoolean) return page
        } while (true)
    }
}
