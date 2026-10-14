package zenpi.transport

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import java.io.InputStream
import java.io.OutputStream
import java.util.UUID
import zenpi.pairing.PairingInvite

/** Runs only after pinned TLS/ALPN verification; never retries a one-use invitation. */
internal object PairingHandshake {
    fun exchange(input: InputStream, output: OutputStream, invite: PairingInvite,
        safetyCode: String, deviceName: String, publicKey: String, deviceId: String,
        sign: (String) -> String, checkActive: () -> Unit = {},
        now: () -> Long = { System.currentTimeMillis() / 1000 }, checkIdentity: () -> Unit = {}): PairingResult {
        fun current() {
            checkActive()
            checkIdentity()
            // Keystore reads can return after the connection owner was cancelled.
            checkActive()
        }
        fun fresh() { current(); invite.requireFresh(now()) }
        fresh()
        if (safetyCode != invite.safetyCode) throw WireException("Safety code mismatch")
        if (deviceName.isBlank() || deviceName.toByteArray(Charsets.UTF_8).size > 128
            || deviceName.any { Character.isISOControl(it) }) throw WireException("Invalid device name")
        FrameCodec.write(output, envelope(JsonObject().apply {
            addProperty("type", "hello"); add("supported_versions", JsonArray().apply { add(1) })
        }))
        HandshakePrelude.welcome(input)
        fresh()
        HandshakePrelude.challenge(input, invite.fingerprint)
        fresh()
        val id = UUID.randomUUID().toString()
        val pair = envelope(JsonObject().apply {
            addProperty("type", "pair"); addProperty("token", invite.token)
            addProperty("fingerprint", invite.fingerprint); addProperty("safety_code", safetyCode)
            addProperty("device_name", deviceName); addProperty("public_key", publicKey)
            addProperty("signature", sign(id))
        }).apply { addProperty("id", id) }
        fresh()
        FrameCodec.write(output, pair)
        val response = FrameCodec.read(input) ?: throw WireException("Pairing outcome unknown")
        current()
        if (response.keySet().any { it !in setOf("schema_version", "channel", "id", "code", "execution_state", "payload") }
            || FrameCodec.string(response, "id") != id
            || FrameCodec.string(response["payload"].asJsonObject, "type") != "response") throw WireException("Pairing response mismatch")
        if (FrameCodec.string(response, "execution_state") != "failed" && response["code"]?.isJsonNull == false)
            throw WireException("Unexpected pairing response code")
        if (FrameCodec.string(response, "execution_state") != "succeeded") throw WireException("Invitation refused or already used")
        val result = response["payload"].asJsonObject["result"]
            ?.takeIf { it.isJsonObject }?.asJsonObject
            ?: throw WireException("Invalid pairing result")
        val required = setOf("paired", "control_available", "device_registered", "device_id")
        if (!result.keySet().containsAll(required) || result.keySet().any { it !in required + "device_revision" }) throw WireException("Invalid pairing result")
        result["device_revision"]?.let {
            if (!it.isJsonPrimitive || !it.asJsonPrimitive.isNumber || !it.toString().matches(Regex("[1-9][0-9]*"))
                || it.asBigInteger > java.math.BigInteger("18446744073709551615")) throw WireException("Invalid device revision")
        }
        fun flag(name: String): Boolean {
            val value = result[name] ?: throw WireException("Invalid pairing result")
            if (!value.isJsonPrimitive || !value.asJsonPrimitive.isBoolean) throw WireException("Invalid pairing result")
            return value.asBoolean
        }
        if (FrameCodec.string(result, "device_id") != deviceId) throw WireException("Registered identity mismatch")
        val paired = flag("paired")
        val registered = flag("device_registered")
        val control = flag("control_available")
        if (!paired || !registered || control) throw WireException("Host pairing unconfirmed")
        return PairingResult(paired, control, registered, deviceId)
    }
    private fun envelope(payload: JsonObject) = JsonObject().apply {
        addProperty("schema_version", 1); addProperty("channel", "control"); add("payload", payload)
    }
}
