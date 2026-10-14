package zenpi

import org.junit.Assert.*
import org.junit.Test
import com.google.gson.JsonObject
import zenpi.transport.FrameCodec
import java.io.*

class FrameCodecTest {
    @Test fun connectionHandshakesCannotCarryRoutingOrNullAuthority() {
        val public = java.util.Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(65).apply { this[0] = 4 })
        val payloads = listOf(
            """{"type":"hello","supported_versions":[1]}""",
            """{"type":"welcome","selected_version":1}""",
            """{"type":"challenge","nonce":"${"A".repeat(43)}","host_fingerprint":"${"a".repeat(64)}"}""",
            """{"type":"pair","token":"${"A".repeat(43)}","fingerprint":"${"a".repeat(64)}","safety_code":"123456","device_name":"phone","public_key":"$public","signature":"proof"}""",
            """{"type":"authenticate","device_id":"${"a".repeat(64)}","signature":"proof"}""",
            """{"type":"revoke_status","device_id":"${"a".repeat(64)}","request_id":"original","signature":"proof"}"""
        )
        for ((index, payload) in payloads.withIndex()) {
            val identified = if (index >= 3) "\"id\":\"handshake\"," else ""
            val valid = decode("""{"schema_version":1,"channel":"control",${identified}"payload":$payload}""")
            for (field in listOf("id", "project_id", "session_id", "turn_id", "sequence", "required_owner", "expected_revision", "code", "execution_state")) {
                if (field == "id" && index >= 3) continue
                val value = when (field) {
                    "sequence", "expected_revision" -> com.google.gson.JsonPrimitive(0)
                    "code" -> com.google.gson.JsonPrimitive("unauthorized")
                    "execution_state" -> com.google.gson.JsonPrimitive("succeeded")
                    else -> com.google.gson.JsonPrimitive("foreign")
                }
                for (injected in listOf(com.google.gson.JsonNull.INSTANCE, value)) {
                    val bad = valid.deepCopy().apply { add(field, injected) }
                    assertTrue("$index:$field", runCatching { FrameCodec.decode(bad.toString().toByteArray()) }.isFailure)
                    val output = ByteArrayOutputStream()
                    assertTrue(runCatching { FrameCodec.write(output, bad) }.isFailure)
                    assertEquals(0, output.size())
                }
            }
        }
    }
    @Test fun challengeNonceHasExactCanonicalThirtyTwoByteEncoding() {
        val alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"
        for (last in 0..255) {
            val bytes = ByteArray(32).apply { this[31] = last.toByte() }
            val canonical = java.util.Base64.getUrlEncoder().withoutPadding().encodeToString(bytes)
            val frame = com.google.gson.JsonParser.parseString("""{"schema_version":1,"channel":"control","payload":{"type":"challenge","nonce":"$canonical","host_fingerprint":"${"a".repeat(64)}"}}""").asJsonObject
            assertEquals(frame, FrameCodec.decode(frame.toString().toByteArray()))
            val index = alphabet.indexOf(canonical.last())
            for (padding in 1..3) {
                val bad = frame.deepCopy().apply { getAsJsonObject("payload").addProperty("nonce", canonical.dropLast(1) + alphabet[index + padding]) }
                assertTrue(runCatching { FrameCodec.decode(bad.toString().toByteArray()) }.isFailure)
                assertTrue(runCatching { FrameCodec.encode(bad) }.isFailure)
            }
        }
    }
    @Test fun revokeStatusProofIsStrictAndCannotCarryProjectAuthority() {
        val value = com.google.gson.JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"query","payload":{"type":"revoke_status","device_id":"${"a".repeat(64)}","request_id":"original","signature":"proof"}}""").asJsonObject
        assertNotNull(FrameCodec.decode(value.toString().toByteArray()))
        for (field in listOf("project_id", "session_id", "turn_id", "required_owner", "expected_revision")) {
            val bad = value.deepCopy(); bad.addProperty(field, "foreign")
            assertTrue(runCatching { FrameCodec.decode(bad.toString().toByteArray()) }.isFailure)
        }
        val bad=value.deepCopy(); bad["payload"].asJsonObject.addProperty("unexpected", true)
        assertTrue(runCatching { FrameCodec.decode(bad.toString().toByteArray()) }.isFailure)
    }
    @Test fun subtabListIsAnExplicitReadWithoutWriteOwnerFields() {
        assertNotNull(FrameCodec.decode("""{"schema_version":1,"channel":"control","id":"read","project_id":"project","payload":{"type":"request","operation":"subtab_list","arguments":{}}}""".toByteArray()))
    }
    private val hello = """{"schema_version":1,"channel":"control","payload":{"type":"hello","supported_versions":[1]}}"""
    private fun decode(text: String) = FrameCodec.decode(text.toByteArray())
    @Test fun roundtripNetworkOrderAndCleanEof() {
        val envelope = decode(hello); val output = ByteArrayOutputStream()
        FrameCodec.write(output, envelope); FrameCodec.write(output, envelope)
        val input = ByteArrayInputStream(output.toByteArray())
        assertEquals(hello.length, java.nio.ByteBuffer.wrap(output.toByteArray()).int)
        assertEquals(envelope, FrameCodec.read(input)); assertEquals(envelope, FrameCodec.read(input)); assertNull(FrameCodec.read(input))
    }
    @Test fun everyTruncationAndMalformedLengthRefused() {
        val output = ByteArrayOutputStream(); FrameCodec.write(output, decode(hello)); val bytes = output.toByteArray()
        for (length in 1 until bytes.size) assertTrue(runCatching { FrameCodec.read(ByteArrayInputStream(bytes.copyOf(length))) }.isFailure)
        for (length in listOf(0, -1, FrameCodec.MAX_FRAME + 1)) {
            assertTrue(runCatching { FrameCodec.read(ByteArrayInputStream(java.nio.ByteBuffer.allocate(4).putInt(length).array())) }.isFailure)
        }
    }
    @Test fun strictJsonVersionAndFields() {
        for (bad in listOf(hello.replace("\"schema_version\":1", "\"schema_version\":4294967297"),
            hello.replace("\"schema_version\":1", "\"schema_version\":\"1\""), hello.replace("\"schema_version\":1", "\"schema_version\":1,\"schema_version\":1"),
            hello.replace("\"channel\":\"control\"", "\"channel\":\"bulk\""), hello.replace("\"type\":\"hello\"", "\"type\":\"hello\",\"admin\":true"),
            hello + "{}", hello.replace("[1]", "[]"), hello.replace("[1]", "[2]"), "null")) {
            assertTrue(bad, runCatching { decode(bad) }.isFailure)
        }
        assertTrue(runCatching { FrameCodec.decode(byteArrayOf(0xff.toByte())) }.isFailure)
    }
    @Test fun duplicatePayloadMembersIncludingEscapedKeysAreRefused() {
        for (payload in listOf(
            """{"type":"request","operation":"inspect","arguments":{"path":"one","path":"two"}}""",
            """{"type":"request","operation":"inspect","arguments":{"nested":[{"path":"one","pa\u0074h":"two"}]}}""",
            """{"type":"response","result":{"access":false,"access":true}}""",
            """{"type":"event","name":"turn","data":{"text":"one","text":"two"}}""",
            """{"type":"snapshot","cursor":{"session_id":"s","next_sequence":0},"state":{"nested":{"value":1,"value":2}}}"""
        )) {
            val frame = """{"schema_version":1,"channel":"control","id":"r","payload":$payload}"""
            val error = runCatching { decode(frame) }.exceptionOrNull()
            assertNotNull(error); assertEquals("Duplicate JSON field", error?.message)
        }
        assertNotNull(decode("""{"schema_version":1,"channel":"control","id":"r","payload":{"type":"request","operation":"inspect","arguments":{"items":[{"name":"one"},{"name":"two"}],"name":"root"}}}"""))
    }

    @Test fun injectedReadTimeoutAndOutputFailureAreNeverSuccess() {
        val output = ByteArrayOutputStream(); FrameCodec.write(output, decode(hello)); val bytes = output.toByteArray()
        for (stop in listOf(0, 2, 4, bytes.size - 1)) {
            val input = object : InputStream() {
                var position = 0
                override fun read(): Int {
                    if (position >= stop) throw java.net.SocketTimeoutException("fixture deadline")
                    return bytes[position++].toInt() and 255
                }
                override fun read(target: ByteArray, offset: Int, length: Int): Int {
                    target[offset] = read().toByte(); return 1
                }
            }
            val failure = runCatching { FrameCodec.read(input) }.exceptionOrNull()
            assertTrue(failure is java.net.SocketTimeoutException)
        }
        val rejected = object : OutputStream() { override fun write(value: Int) { throw IOException("fixture broken pipe") } }
        assertTrue(runCatching { FrameCodec.write(rejected, decode(hello)) }.exceptionOrNull() is IOException)
    }
    @Test fun outputLimitRefusesBeforePrefixAndResponseMustHaveOutcome() {
        val envelope = decode("""{"schema_version":1,"channel":"control","id":"r1","execution_state":"unknown_outcome","payload":{"type":"response","result":{}}}""")
        val overhead = envelope.toString().toByteArray().size + "\"text\":\"\"".length
        envelope["payload"].asJsonObject["result"].asJsonObject.addProperty("text", "x".repeat(FrameCodec.MAX_CONTROL - overhead))
        val output = ByteArrayOutputStream(); FrameCodec.write(output, envelope)
        assertEquals(FrameCodec.MAX_CONTROL + 4, output.size())
        envelope["payload"].asJsonObject["result"].asJsonObject.addProperty("text", "x".repeat(FrameCodec.MAX_CONTROL))
        val rejected = ByteArrayOutputStream()
        assertTrue(runCatching { FrameCodec.write(rejected, envelope) }.isFailure); assertEquals(0, rejected.size())
        envelope.remove("execution_state")
        assertTrue(runCatching { FrameCodec.write(rejected, envelope) }.isFailure)
    }
    @Test fun programmaticInvalidUnicodeIsNotSilentlyReplacedOnWrite() {
        for(text in listOf("\uD800","\uDC00","prefix\uD800suffix")) {
            val envelope=decode("""{"schema_version":1,"channel":"control","id":"r","execution_state":"succeeded","payload":{"type":"response","result":{}}}""")
            envelope.getAsJsonObject("payload").getAsJsonObject("result").addProperty("text",text)
            val output=ByteArrayOutputStream()
            assertTrue(runCatching { FrameCodec.encode(envelope) }.isFailure)
            assertTrue(runCatching{FrameCodec.write(output,envelope)}.isFailure)
            assertEquals(0,output.size())
        }
    }
}
