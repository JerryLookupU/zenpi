package zenpi.transport

import com.google.gson.JsonObject
import java.io.InputStream

internal class HostNegotiationException : WireException("Host protocol negotiation refused") {
    companion object {
        fun refused(error: Throwable): Boolean {
            val seen = java.util.Collections.newSetFromMap(java.util.IdentityHashMap<Throwable, Boolean>())
            var current: Throwable? = error
            while (current != null && seen.add(current)) {
                if (current is HostNegotiationException) return true
                current = current.cause
            }
            return false
        }
    }
}

/** Negotiation is connection-scoped, never a routed request or permission receipt. */
internal object HandshakePrelude {
    fun applicationProtocol(value: String?) {
        if (value != "zenpi/1") throw HostNegotiationException()
    }
    private fun read(input: InputStream, type: String, fields: Set<String>): JsonObject {
        val frame = try { FrameCodec.read(input) }
        catch (_: WireException) { throw HostNegotiationException() }
            ?: throw WireException("Host disconnected during negotiation")
        if (frame.keySet() != setOf("schema_version", "channel", "payload")
            || FrameCodec.string(frame, "channel") != "control") throw HostNegotiationException()
        val payload = frame["payload"]?.takeIf { it.isJsonObject }?.asJsonObject
            ?: throw HostNegotiationException()
        if (payload.keySet() != fields || FrameCodec.string(payload, "type") != type)
            throw HostNegotiationException()
        return payload
    }

    fun welcome(input: InputStream) {
        val payload = read(input, "welcome", setOf("type", "selected_version"))
        if (payload["selected_version"].toString() != "1") throw HostNegotiationException()
    }

    fun challenge(input: InputStream, fingerprint: String): String {
        val payload = read(input, "challenge", setOf("type", "nonce", "host_fingerprint"))
        if (FrameCodec.string(payload, "host_fingerprint") != fingerprint)
            throw HostNegotiationException()
        return FrameCodec.string(payload, "nonce")
    }
}
