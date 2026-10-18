package zenpi.transport

import com.google.gson.*
import com.google.gson.stream.JsonReader
import com.google.gson.stream.JsonToken
import java.io.*
import java.nio.ByteBuffer
import java.nio.charset.CodingErrorAction

/** The host accepted a write but could not prove its final outcome. */
class WorkspaceUnknownOutcomeException(val code: String? = null) : WireException("Workspace request outcome unknown")

internal object WorkspaceOutcomeCodec {
    fun unknownWrite(request: JsonObject, response: JsonObject): Boolean {
        val payload = request["payload"]?.takeIf { it.isJsonObject }?.asJsonObject ?: return false
        return payload["type"]?.asString == "request" && payload["operation"]?.asString in RemoteSchema.WRITE_OPERATIONS
            && response["execution_state"]?.asString == "unknown_outcome"
            && response["payload"]?.takeIf { it.isJsonObject }?.asJsonObject?.get("result")?.isJsonNull == true
    }
    fun requireKnown(request: JsonObject, response: JsonObject) {
        if (unknownWrite(request, response)) throw WorkspaceUnknownOutcomeException(
            response["code"]?.takeUnless { it.isJsonNull }?.let { FrameCodec.string(response, "code") })
    }
}

open class WireException(message: String) : IOException(message)
class HostRefusedException(val code: String) : WireException("Host request refused: $code")
class WorkspaceRefusedException(message: String, val code: String? = null) : WireException(message)
/** A validated authorized listing no longer contains the requested project. */
internal class SelectedProjectUnavailableException(val projectId: String) : WireException("Selected project unavailable")

object FrameCodec {
    const val MAX_FRAME = 1_048_576
    const val MAX_CONTROL = 262_144
    private val envelopeFields = setOf("schema_version", "channel", "id", "session_id", "project_id", "turn_id",
        "sequence", "code", "execution_state", "required_owner", "expected_revision", "payload")
    private fun scalarString(value: String): String {
        var index = 0
        while (index < value.length) {
            val character = value[index++]
            if (Character.isHighSurrogate(character)) {
                if (index == value.length || !Character.isLowSurrogate(value[index++])) throw WireException("Invalid Unicode scalar")
            } else if (Character.isLowSurrogate(character)) throw WireException("Invalid Unicode scalar")
        }
        return value
    }

    fun read(input: InputStream): JsonObject? = read(input) {}

    internal fun read(input: InputStream, consumeBytes: (Int) -> Unit): JsonObject? {
        val first = input.read()
        if (first == -1) return null
        val prefix = ByteArray(4); prefix[0] = first.toByte()
        DataInputStream(input).readFully(prefix, 1, 3)
        val length = ByteBuffer.wrap(prefix).int
        if (length !in 1..MAX_FRAME) throw WireException("Frame length refused")
        consumeBytes(length)
        val bytes = ByteArray(length)
        DataInputStream(input).readFully(bytes)
        return decode(bytes)
    }

    fun encode(envelope: JsonObject): ByteArray {
        val encoded = Charsets.UTF_8.newEncoder().onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT).encode(java.nio.CharBuffer.wrap(envelope.toString()))
        val bytes = ByteArray(encoded.remaining()).also { encoded.get(it) }
        if (bytes.size > MAX_FRAME) throw WireException("Frame too large")
        decode(bytes)
        return bytes
    }

    fun write(output: OutputStream, envelope: JsonObject) {
        val bytes = encode(envelope)
        val writer = DataOutputStream(output)
        writer.writeInt(bytes.size); writer.write(bytes); writer.flush()
    }

    fun decode(bytes: ByteArray): JsonObject {
        if (bytes.size !in 1..MAX_FRAME) throw WireException("Frame too large")
        try {
            val text = Charsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT)
                .onUnmappableCharacter(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes)).toString()
            if (text.startsWith('\uFEFF')) throw WireException("JSON byte order mark refused")
            val reader = JsonReader(StringReader(text)).apply { strictness = Strictness.STRICT }
            var nodes = 0
            fun parse(depth: Int): JsonElement {
                if (++nodes > 8192 || depth > 40) throw WireException("JSON complexity refused")
                return when (reader.peek()) {
                    JsonToken.BEGIN_OBJECT -> JsonObject().also { result ->
                        reader.beginObject()
                        while (reader.hasNext()) {
                            val key = scalarString(reader.nextName())
                            if (result.has(key)) throw WireException("Duplicate JSON field")
                            result.add(key, parse(depth + 1))
                        }; reader.endObject()
                    }
                    JsonToken.BEGIN_ARRAY -> JsonArray().also { result ->
                        reader.beginArray(); while (reader.hasNext()) result.add(parse(depth + 1)); reader.endArray()
                    }
                    JsonToken.STRING -> JsonPrimitive(scalarString(reader.nextString()))
                    JsonToken.NUMBER -> {
                        val number = reader.nextString()
                        if (!number.toDouble().isFinite()) throw WireException("Number out of range")
                        JsonParser.parseString(number)
                    }
                    JsonToken.BOOLEAN -> JsonPrimitive(reader.nextBoolean())
                    JsonToken.NULL -> { reader.nextNull(); JsonNull.INSTANCE }
                    else -> throw WireException("Invalid JSON")
                }
            }
            val value = parse(0)
            if (reader.peek() != JsonToken.END_DOCUMENT || !value.isJsonObject) throw WireException("Invalid envelope")
            val envelope = value.asJsonObject
            validate(envelope, bytes.size)
            return envelope
        } catch (error: WireException) { throw error }
        catch (_: Exception) { throw WireException("Invalid remote frame") }
    }

    fun string(value: JsonObject, field: String): String {
        val element = value[field] ?: throw WireException("Missing $field")
        if (!element.isJsonPrimitive || !element.asJsonPrimitive.isString) throw WireException("Invalid $field")
        return element.asString
    }
    private fun identifier(value: String) {
        if (value.isEmpty() || value.toByteArray(Charsets.UTF_8).size > 128 || value.any { Character.isISOControl(it) })
            throw WireException("Invalid identifier")
    }
    private fun unsigned(value: JsonElement): java.math.BigInteger {
        if (!value.isJsonPrimitive || !value.asJsonPrimitive.isNumber || !value.toString().matches(Regex("0|[1-9][0-9]*")))
            throw WireException("Invalid unsigned integer")
        return value.asBigInteger.also {
            if (it > java.math.BigInteger("18446744073709551615")) throw WireException("Integer overflow")
        }
    }
    private fun validate(envelope: JsonObject, bytes: Int) {
        if (envelope.keySet().any { it !in envelopeFields } || envelope["schema_version"]?.let(::unsigned) != java.math.BigInteger.valueOf(RemoteSchema.VERSION.toLong()))
            throw WireException("Unsupported envelope")
        val channel = string(envelope, "channel")
        if (channel !in setOf("control", "event", "bulk") || bytes > if (channel == "bulk") MAX_FRAME else MAX_CONTROL)
            throw WireException("Channel limit")
        for (field in listOf("id", "session_id", "project_id", "turn_id", "required_owner")) {
            if (envelope.has(field) && !envelope[field].isJsonNull) identifier(string(envelope, field))
        }
        for (field in listOf("expected_revision", "sequence")) {
            if (envelope.has(field) && !envelope[field].isJsonNull) unsigned(envelope[field])
        }
        val payload = envelope["payload"]?.takeIf { it.isJsonObject }?.asJsonObject ?: throw WireException("Missing payload")
        val type = string(payload, "type")
        if (type in setOf("hello", "welcome", "challenge", "pair", "authenticate", "revoke_status")) {
            val allowed = setOf("schema_version", "channel", "payload") +
                if (type in setOf("pair", "authenticate", "revoke_status")) setOf("id") else emptySet()
            if (envelope.keySet() != allowed) throw WireException("Connection handshake fields")
        }
        val fields = RemoteSchema.MESSAGE_FIELDS[type] ?: throw WireException("Unknown message")
        if (payload.keySet() != fields) throw WireException("Invalid message fields")
        val expectedChannel = RemoteSchema.MESSAGE_CHANNELS[type]
        if (channel != expectedChannel) throw WireException("Channel mismatch")
        if (type != "response" && listOf("code", "execution_state").any { envelope.has(it) && !envelope[it].isJsonNull })
            throw WireException("Response fields on request")
        if (type != "request" && listOf("required_owner", "expected_revision").any { envelope.has(it) && !envelope[it].isJsonNull })
            throw WireException("Request fields on response")
        if (type != "event" && envelope.has("sequence") && !envelope["sequence"].isJsonNull) throw WireException("Invalid sequence")
        when (type) {
            "hello" -> {
                val versions = payload["supported_versions"].asJsonArray
                if (versions.size() !in 1..8 || versions.none { unsigned(it) == java.math.BigInteger.ONE }
                    || versions.any { unsigned(it) > java.math.BigInteger.valueOf(65535) }) throw WireException("Unsupported versions")
            }
            "welcome" -> if (unsigned(payload["selected_version"]) != java.math.BigInteger.ONE) throw WireException("Unsupported version")
            "challenge" -> {
                val nonce = string(payload, "nonce")
                if (!nonce.matches(Regex("[A-Za-z0-9_-]{43}"))
                    || java.util.Base64.getUrlDecoder().decode(nonce).let {
                        it.size != 32 || java.util.Base64.getUrlEncoder().withoutPadding().encodeToString(it) != nonce
                    }
                    || !string(payload, "host_fingerprint").matches(Regex("[0-9a-f]{64}"))) throw WireException("Invalid challenge")
            }
            "authenticate", "revoke_status" -> {
                identifier(string(envelope, "id"))
                if (!string(payload, "device_id").matches(Regex("[0-9a-f]{64}"))
                    || !string(payload, "signature").matches(Regex("[A-Za-z0-9_-]{1,104}"))) throw WireException("Invalid authentication")
                if (type == "revoke_status") {
                    identifier(string(payload, "request_id"))
                    if (listOf("project_id", "session_id", "turn_id").any { envelope.has(it) && !envelope[it].isJsonNull }) throw WireException("Invalid revoke status routing")
                }
            }
            "pair" -> {
                identifier(string(envelope, "id")); identifier(string(payload, "device_name"))
                if (!string(payload, "token").matches(Regex("[A-Za-z0-9_-]{43}"))
                    || !string(payload, "fingerprint").matches(Regex("[0-9a-f]{64}"))) throw WireException("Invalid pairing proof")
                val key = string(payload, "public_key")
                val signature = string(payload, "signature")
                if (!key.matches(Regex("[A-Za-z0-9_-]{87}")) || java.util.Base64.getUrlDecoder().decode(key).let {
                        it.size != 65 || it[0] != 4.toByte() || java.util.Base64.getUrlEncoder().withoutPadding().encodeToString(it) != key }
                    || !signature.matches(Regex("[A-Za-z0-9_-]{1,104}"))) throw WireException("Invalid device proof")
            }
            "response" -> {
                identifier(string(envelope, "id"))
                val state = string(envelope, "execution_state")
                if (state !in RemoteSchema.EXECUTION_STATES) throw WireException("Unknown outcome")
                if (state == "failed" && (!envelope.has("code") || envelope["code"].isJsonNull)) throw WireException("Missing failure code")
                if (envelope.has("code") && !envelope["code"].isJsonNull && string(envelope, "code") !in RemoteSchema.ERROR_CODES) throw WireException("Unknown error")
            }
            "request" -> {
                identifier(string(envelope, "id"))
                val operation = string(payload, "operation")
                if (operation !in RemoteSchema.OPERATIONS) throw WireException("Unknown operation")
                if (!payload["arguments"].isJsonObject) throw WireException("Invalid arguments")
                if (operation in RemoteSchema.WRITE_OPERATIONS) {
                    identifier(string(envelope, "required_owner")); unsigned(envelope["expected_revision"])
                }
            }
            "event" -> { identifier(string(payload, "name")); identifier(string(envelope, "session_id")); unsigned(envelope["sequence"]) }
            "ack", "resume", "subscribe", "snapshot" -> {
                val cursor = payload["cursor"].asJsonObject
                if (cursor.keySet() != setOf("session_id", "next_sequence") || string(cursor, "session_id") != string(envelope, "session_id")) throw WireException("Cursor mismatch")
                identifier(string(cursor, "session_id"))
                unsigned(cursor["next_sequence"])
                if (type in setOf("resume", "subscribe") && unsigned(payload["limit"]) !in java.math.BigInteger.ONE..java.math.BigInteger.valueOf(128)) throw WireException("Replay limit")
                if (type == "subscribe") {
                    identifier(string(envelope, "id")); identifier(string(envelope, "project_id"))
                    if (envelope.has("turn_id") && !envelope["turn_id"].isJsonNull || unsigned(payload["duration_ms"]) !in java.math.BigInteger.valueOf(1000)..java.math.BigInteger.valueOf(5000))
                        throw WireException("Invalid bounded subscription")
                }
            }
        }
        val data = when (type) { "request" -> "arguments"; "response" -> "result"; "event" -> "data"; "snapshot" -> "state"; else -> null }
        if (data != null) {
            var count = 0
            fun bounded(value: JsonElement, depth: Int) {
                if (++count > 4096 || depth > 32) throw WireException("Payload complexity")
                if (value.isJsonObject) value.asJsonObject.entrySet().forEach { bounded(it.value, depth + 1) }
                if (value.isJsonArray) value.asJsonArray.forEach { bounded(it, depth + 1) }
            }
            bounded(payload[data], 0)
        }
    }
}
