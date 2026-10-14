package zenpi

import com.google.gson.JsonNull
import com.google.gson.JsonObject
import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.pairing.AuthorityLossPolicy
import zenpi.lifecycle.ResumeRetryPolicy

class AuthenticationReceiptTest {
    private val device = "a".repeat(64)
    private fun receipt() = JsonParser.parseString("""{"authenticated":true,"device_id":"$device","device_revision":1,"control_available":false}""").asJsonObject
    @Test fun currentReceiptIsReadOnlyAndRevisionIsExactUnsignedIdentityFence() {
        assertEquals("1", AuthenticationReceipt.validate(receipt(), device))
        val maximum = receipt().apply { addProperty("device_revision", java.math.BigInteger("18446744073709551615")) }
        assertEquals("18446744073709551615", AuthenticationReceipt.validate(maximum, device))
        val legacy = receipt().apply { remove("device_revision") }
        assertNull(AuthenticationReceipt.validate(legacy, device))
    }
    @Test fun missingUnknownOrAuthorityFieldsNeverAdmitAWorkspaceRead() {
        val changes = listOf<(JsonObject) -> Unit>(
            { it.remove("authenticated") }, { it.remove("device_id") }, { it.remove("control_available") },
            { it.addProperty("control_available", true) }, { it.addProperty("authenticated", false) },
            { it.addProperty("project_id", "private") }, { it.addProperty("capabilities", "control") },
            { it.addProperty("token", "PRIVATE_TOKEN") }, { it.addProperty("private_key", "PRIVATE_KEY") })
        for (change in changes) {
            var workspaceReads = 0
            val error = runCatching {
                AuthenticationReceipt.validate(receipt().apply(change), device)
                workspaceReads++
            }.exceptionOrNull()
            assertTrue(error is WireException); assertEquals(0, workspaceReads)
            assertFalse(requireNotNull(error).message.orEmpty().contains("PRIVATE_"))
        }
    }
    @Test fun booleansCannotBeCoercedFromStringsNumbersNullOrContainers() {
        for (field in listOf("authenticated", "control_available")) {
            for (raw in listOf("\"true\"", "\"false\"", "1", "0", "null", "[]", "{}")) {
                val bad = receipt().apply { add(field, JsonParser.parseString(raw)) }
                assertTrue("$field=$raw", runCatching { AuthenticationReceipt.validate(bad, device) }.isFailure)
            }
        }
    }
    @Test fun identityMustMatchTheSavedSigningDeviceExactly() {
        for (id in listOf("b".repeat(64), device.uppercase(), "a".repeat(63), "a".repeat(65), "PRIVATE_DEVICE\n", "")) {
            assertTrue(runCatching { AuthenticationReceipt.validate(receipt().apply { addProperty("device_id", id) }, device) }.isFailure)
        }
        assertTrue(runCatching { AuthenticationReceipt.validate(receipt(), "invalid") }.isFailure)
        assertTrue(runCatching { AuthenticationReceipt.validate(receipt().apply { add("device_id", JsonNull.INSTANCE) }, device) }.isFailure)
    }
    @Test fun revisionCannotBeZeroCoercedFractionalNegativeOrOverflow() {
        for (raw in listOf("0", "-1", "1.0", "1e0", "\"1\"", "true", "null", "[]", "{}", "18446744073709551616")) {
            assertTrue(raw, runCatching { AuthenticationReceipt.validate(receipt().apply { add("device_revision", JsonParser.parseString(raw)) }, device) }.isFailure)
        }
    }
    @Test fun realFramedCorrelatedReceiptMustValidateBeforeAnyControlFollowup() = kotlinx.coroutines.runBlocking {
        for (valid in listOf(false, true)) {
            val result = receipt().apply { if (!valid) addProperty("authenticated", "true") }
            val envelope = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"auth","execution_state":"succeeded","payload":{"type":"response","result":{}}}""").asJsonObject
                .apply { getAsJsonObject("payload").add("result", result) }
            val bytes = java.io.ByteArrayOutputStream().also { FrameCodec.write(it, envelope) }.toByteArray()
            val request = JsonParser.parseString("""{"id":"auth","payload":{"type":"authenticate"}}""").asJsonObject
            var reads = 0
            val outcome = runCatching {
                val page = ResponseReader.read(java.io.ByteArrayInputStream(bytes), request, { AuthenticationReceipt.validateResponse(it, device) })
                AuthenticationReceipt.validate(page.result, device)
                reads++
            }
            assertEquals(valid, outcome.isSuccess); assertEquals(if (valid) 1 else 0, reads)
        }
    }
    @Test fun successEnvelopeCannotSmuggleRoutingOwnerOrFailureMetadata() {
        val envelope = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"auth","execution_state":"succeeded","payload":{"type":"response","result":{}}}""").asJsonObject
            .apply { getAsJsonObject("payload").add("result", receipt()) }
        AuthenticationReceipt.validateResponse(envelope, device)
        for (field in listOf("project_id", "session_id", "turn_id", "required_owner", "expected_revision", "sequence", "code", "extra")) {
            for (value in listOf(com.google.gson.JsonPrimitive("injected"), JsonNull.INSTANCE)) {
                val bad = envelope.deepCopy().apply { add(field, value) }
                assertTrue(field, runCatching { AuthenticationReceipt.validateResponse(bad, device) }.isFailure)
            }
        }
        assertTrue(runCatching { AuthenticationReceipt.validateResponse(envelope.deepCopy().apply {
            getAsJsonObject("payload").add("result", JsonNull.INSTANCE)
        }, device) }.isFailure)
    }
    @Test fun nonObjectAuthenticationResultUsesWireFailure() {
        val envelope = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"auth","execution_state":"succeeded","payload":{"type":"response","result":null}}""").asJsonObject
        val error = runCatching { AuthenticationReceipt.validateResponse(envelope, device) }.exceptionOrNull()
        assertTrue(error is WireException)
    }
    @Test fun rejectedReceiptsFenceAuthorityAndRetryByTypeNotDiagnosticText() {
        val changes = listOf<(JsonObject) -> Unit>(
            { it.addProperty("authenticated", false) }, { it.addProperty("authenticated", "true") },
            { it.addProperty("device_id", "b".repeat(64)) }, { it.remove("control_available") },
            { it.addProperty("control_available", true) }, { it.addProperty("device_revision", 0) },
            { it.addProperty("private_key", "PRIVATE_KEY") })
        for (change in changes) {
            val failure = runCatching { AuthenticationReceipt.validate(receipt().apply(change), device) }.exceptionOrNull() as Exception
            assertTrue(failure is HostAuthenticationException)
            assertEquals("Host device authentication unconfirmed", failure.message)
            for (error in listOf(failure, java.io.IOException("wrapper", failure))) {
                assertTrue(HostAuthenticationException.refused(error))
                assertTrue(AuthorityLossPolicy.refused(error))
                assertTrue(ResumeRetryPolicy.permanent(error))
            }
        }
        val cycle = java.io.IOException("cycle")
        val second = java.io.IOException("second", cycle); cycle.initCause(second)
        for (error in listOf(cycle, WireException("Host device authentication unconfirmed"),
            java.net.SocketTimeoutException("Host device authentication unconfirmed"))) {
            assertFalse(HostAuthenticationException.refused(error))
            assertFalse(AuthorityLossPolicy.refused(error))
            assertFalse(ResumeRetryPolicy.permanent(error))
        }
    }
    @Test fun fragmentedCorrelatedInvalidReceiptFencesBeforeFollowupOrPublication() = kotlinx.coroutines.runBlocking {
        val envelope = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"auth","execution_state":"succeeded","payload":{"type":"response","result":{}}}""").asJsonObject
            .apply { getAsJsonObject("payload").add("result", receipt().apply { addProperty("device_id", "b".repeat(64)) }) }
        val bytes = java.io.ByteArrayOutputStream().also { FrameCodec.write(it, envelope) }.toByteArray()
        val input = object : java.io.ByteArrayInputStream(bytes) {
            override fun read(target: ByteArray, offset: Int, length: Int): Int = super.read(target, offset, minOf(length, 2))
        }
        val request = JsonParser.parseString("""{"id":"auth","payload":{"type":"authenticate"}}""").asJsonObject
        var reads = 0
        val error = runCatching {
            ResponseReader.read(input, request, { AuthenticationReceipt.validateResponse(it, device) })
            reads++
        }.exceptionOrNull() as Exception
        assertEquals(0, reads)
        assertTrue(AuthorityLossPolicy.refused(error)); assertTrue(ResumeRetryPolicy.permanent(error))
    }
    @Test fun framingCorrelationAndIncompleteOutcomesCannotBypassAuthenticationFence() = kotlinx.coroutines.runBlocking {
        fun envelope() = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"auth","execution_state":"succeeded","payload":{"type":"response","result":{}}}""").asJsonObject
            .apply { getAsJsonObject("payload").add("result", receipt()) }
        fun raw(value: JsonObject) = java.io.ByteArrayOutputStream().also {
            val body = value.toString().toByteArray()
            java.io.DataOutputStream(it).apply { writeInt(body.size); write(body) }
        }.toByteArray()
        val bad = listOf(envelope().apply { addProperty("id", "foreign") },
            envelope().apply { addProperty("code", "busy") },
            envelope().apply { addProperty("turn_id", "foreign") },
            envelope().apply { addProperty("channel", "event") },
            envelope().apply { addProperty("schema_version", 2) },
            envelope().apply { addProperty("execution_state", "accepted") },
            envelope().apply { addProperty("execution_state", "unknown_outcome") },
            JsonParser.parseString("""{"schema_version":1,"channel":"control","payload":{"type":"welcome","selected_version":1}}""").asJsonObject,
            JsonParser.parseString("""{"schema_version":1,"channel":"event","session_id":"session","sequence":0,"payload":{"type":"event","name":"transcript_turn","data":{}}}""").asJsonObject)
        for (value in bad) {
            val request = JsonParser.parseString("""{"id":"auth","payload":{"type":"authenticate"}}""").asJsonObject
            var validated = 0; var followups = 0
            val error = runCatching {
                ResponseReader.read(java.io.ByteArrayInputStream(raw(value)), request, {
                    validated++; AuthenticationReceipt.validateResponse(it, device)
                })
                followups++
            }.exceptionOrNull() as Exception
            assertTrue(error is HostAuthenticationException)
            assertTrue(AuthorityLossPolicy.refused(error)); assertTrue(ResumeRetryPolicy.permanent(error))
            assertEquals(0, followups)
            assertTrue(validated <= 1)
        }
    }
    @Test fun authenticationTransportTruncationAndTimeoutRemainTransient() = kotlinx.coroutines.runBlocking {
        val response = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"auth","execution_state":"succeeded","payload":{"type":"response","result":{}}}""").asJsonObject
            .apply { getAsJsonObject("payload").add("result", receipt()) }
        val bytes = java.io.ByteArrayOutputStream().also { FrameCodec.write(it, response) }.toByteArray()
        val request = JsonParser.parseString("""{"id":"auth","payload":{"type":"authenticate"}}""").asJsonObject
        for (cut in 0 until bytes.size) {
            val error = runCatching {
                ResponseReader.read(java.io.ByteArrayInputStream(bytes.copyOf(cut)), request, { fail("incomplete receipt") })
            }.exceptionOrNull() as Exception
            assertFalse(AuthorityLossPolicy.refused(error)); assertFalse(ResumeRetryPolicy.permanent(error))
        }
        val timeout = java.net.SocketTimeoutException("fixture")
        val input = object : java.io.InputStream() { override fun read(): Int = throw timeout }
        assertSame(timeout, runCatching { ResponseReader.read(input, request, {}) }.exceptionOrNull())
    }
}
