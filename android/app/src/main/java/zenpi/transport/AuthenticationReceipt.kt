package zenpi.transport

import com.google.gson.JsonObject
import java.math.BigInteger

internal class HostAuthenticationException : WireException("Host device authentication unconfirmed") {
    companion object {
        fun refused(error: Throwable): Boolean {
            val seen = java.util.Collections.newSetFromMap(java.util.IdentityHashMap<Throwable, Boolean>())
            var current: Throwable? = error
            while (current != null && seen.add(current)) {
                if (current is HostAuthenticationException) return true
                current = current.cause
            }
            return false
        }
    }
}

internal object AuthenticationReceipt {
    fun validateResponse(response: JsonObject, deviceId: String) {
        try { requireResponse(response, deviceId) }
        catch (_: WireException) { throw HostAuthenticationException() }
    }
    private fun requireResponse(response: JsonObject, deviceId: String) {
        val allowed = if (FrameCodec.string(response, "execution_state") == "failed")
            setOf("schema_version", "channel", "id", "code", "execution_state", "payload")
        else setOf("schema_version", "channel", "id", "execution_state", "payload")
        if (response.keySet() != allowed
            || FrameCodec.string(response, "channel") != "control" || FrameCodec.string(response, "execution_state") != "succeeded")
            throw WireException("Invalid authentication envelope")
        val result = response["payload"].asJsonObject["result"]
            ?.takeIf { it.isJsonObject }?.asJsonObject
            ?: throw WireException("Invalid authentication receipt")
        validate(result, deviceId)
    }
    fun validate(value: JsonObject, deviceId: String): String? {
        return try { requireReceipt(value, deviceId) }
        catch (_: WireException) { throw HostAuthenticationException() }
    }
    private fun requireReceipt(value: JsonObject, deviceId: String): String? {
        val required = setOf("authenticated", "device_id", "control_available")
        if (!value.keySet().containsAll(required) || value.keySet().any { it !in required + "device_revision" })
            throw WireException("Invalid authentication receipt")
        fun flag(name: String): Boolean {
            val flag = value[name]
            if (!flag.isJsonPrimitive || !flag.asJsonPrimitive.isBoolean) throw WireException("Invalid authentication receipt")
            return flag.asBoolean
        }
        flag("control_available")
        if (!deviceId.matches(Regex("[0-9a-f]{64}")) || FrameCodec.string(value, "device_id") != deviceId
            || !flag("authenticated")) throw WireException("Device authentication refused")
        return value["device_revision"]?.let { revision ->
            if (!revision.isJsonPrimitive || !revision.asJsonPrimitive.isNumber
                || !revision.toString().matches(Regex("[1-9][0-9]*"))
                || revision.asBigInteger > BigInteger("18446744073709551615")) throw WireException("Invalid device revision")
            revision.toString()
        }
    }
}
