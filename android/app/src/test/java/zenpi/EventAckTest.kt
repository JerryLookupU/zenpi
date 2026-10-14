package zenpi

import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.EventAckCodec
import zenpi.transport.HostRefusedException

class EventAckTest {
    private fun receipt() = JsonParser.parseString("""{"device_id":"device","project_id":"project","cursor_scope":"transcript_events_v1","cursor":{"session_id":"session","next_sequence":2}}""").asJsonObject
    @Test fun matchingAndMonotonicDuplicateAreAccepted() {
        EventAckCodec.validate(receipt(), "device", "project", "session", "2")
        EventAckCodec.validate(receipt(), "device", "project", "session", "1")
    }
    @Test fun wrongOwnerScopeSessionAndFieldsAreRefused() {
        for (field in listOf("device_id", "project_id", "cursor_scope")) {
            val value = receipt().apply { addProperty(field, "wrong") }
            assertTrue(runCatching { EventAckCodec.validate(value, "device", "project", "session", "2") }.isFailure)
        }
        val wrong = receipt().apply { getAsJsonObject("cursor").addProperty("session_id", "foreign") }
        assertTrue(runCatching { EventAckCodec.validate(wrong, "device", "project", "session", "2") }.isFailure)
        val extra = receipt().apply { addProperty("extra", true) }
        assertTrue(runCatching { EventAckCodec.validate(extra, "device", "project", "session", "2") }.isFailure)
    }
    @Test fun regressedStringFractionNegativeAndOverflowSequencesAreRefused() {
        for (number in listOf("1", "\"2\"", "2.0", "-1", "18446744073709551616", "null")) {
            val value = receipt().apply { getAsJsonObject("cursor").add("next_sequence", JsonParser.parseString(number)) }
            assertTrue(number, runCatching { EventAckCodec.validate(value, "device", "project", "session", "2") }.isFailure)
        }
    }
    @Test fun busyAckDefersWithoutRetryOrCursorChangeAndLaterReceiptConfirms() = kotlinx.coroutines.runBlocking {
        var calls = 0
        val request = EventAckCodec.request("project", "session", "2")
        assertFalse(EventAckCodec.acknowledge(request, "device", "project", "session", "2") {
            calls++; throw HostRefusedException("busy")
        })
        assertEquals(1, calls)
        assertTrue(EventAckCodec.acknowledge(request, "device", "project", "session", "2") {
            calls++; receipt()
        })
        assertEquals(2, calls)
    }
    @Test fun onlyTypedBusyRefusalCanDeferAnAck() = kotlinx.coroutines.runBlocking {
        val failures = listOf(HostRefusedException("unauthorized"), HostRefusedException("owner_mismatch"),
            HostRefusedException("invalid_request"), zenpi.transport.WireException("Host request refused: busy"),
            java.net.SocketTimeoutException("busy"), kotlinx.coroutines.CancellationException("busy"))
        for (failure in failures) {
            assertSame(failure, runCatching {
                EventAckCodec.acknowledge(EventAckCodec.request("project", "session", "2"), "device", "project", "session", "2") { throw failure }
            }.exceptionOrNull())
        }
        assertTrue(runCatching {
            EventAckCodec.acknowledge(EventAckCodec.request("project", "session", "2"), "device", "project", "session", "2") { receipt().apply {
                addProperty("device_id", "foreign")
            } }
        }.isFailure)
    }
    @Test fun correlatedBusyFrameLeavesFollowingReadAvailableOnSameStream() = kotlinx.coroutines.runBlocking {
        val output = java.io.ByteArrayOutputStream()
        val refused = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"ack","code":"busy","execution_state":"failed","payload":{"type":"response","result":null}}""").asJsonObject
        val next = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"read","execution_state":"succeeded","payload":{"type":"response","result":{"available":true}}}""").asJsonObject
        zenpi.transport.FrameCodec.write(output, refused); zenpi.transport.FrameCodec.write(output, next)
        val input = java.io.ByteArrayInputStream(output.toByteArray())
        val ack = EventAckCodec.request("project", "session", "2").apply { addProperty("id", "ack") }
        assertFalse(EventAckCodec.acknowledge(ack, "device", "project", "session", "2") { request ->
            zenpi.transport.ResponseReader.read(input, request, { response ->
                if (response["execution_state"].asString == "failed") throw HostRefusedException(response["code"].asString)
            }).result
        })
        val read = JsonParser.parseString("""{"id":"read","payload":{"type":"request"}}""").asJsonObject
        assertTrue(zenpi.transport.ResponseReader.read(input, read, {}).result["available"].asBoolean)
        assertNull(zenpi.transport.FrameCodec.read(input))
    }
    @Test fun malformedRetainedScopeOrSequenceCannotConstructOrConfirmAnAck() {
        for (number in listOf("", "01", "+1", "-1", "1.0", "1e0", " 1", "18446744073709551616")) {
            assertTrue(number, runCatching { EventAckCodec.request("project", "session", number) }.exceptionOrNull() is zenpi.transport.WireException)
            assertTrue(number, runCatching { EventAckCodec.validate(receipt(), "device", "project", "session", number) }.exceptionOrNull() is zenpi.transport.WireException)
        }
        for (scope in listOf("", "bad\u0000scope", "x".repeat(129))) {
            assertTrue(runCatching { EventAckCodec.request(scope, "session", "0") }.isFailure)
            assertTrue(runCatching { EventAckCodec.request("project", scope, "0") }.isFailure)
        }
        val maximum = "18446744073709551615"
        assertEquals(maximum, EventAckCodec.request("project", "session", maximum)
            .getAsJsonObject("payload").getAsJsonObject("cursor")["next_sequence"].toString())
    }
    @Test fun mismatchedAckRequestCannotReachExchangeEvenIfHostWouldReturnAValidReceipt() = kotlinx.coroutines.runBlocking {
        for (change in listOf<(com.google.gson.JsonObject) -> Unit>(
            { it.addProperty("project_id", "foreign") }, { it.addProperty("session_id", "foreign") },
            { it.getAsJsonObject("payload").getAsJsonObject("cursor").addProperty("next_sequence", 1) },
            { it.getAsJsonObject("payload").getAsJsonObject("cursor").addProperty("session_id", "foreign") },
            { it.getAsJsonObject("payload").addProperty("type", "resume") },
            { it.addProperty("expected_revision", 0) }, { it.add("id", com.google.gson.JsonNull.INSTANCE) })) {
            var exchanges = 0
            assertTrue(runCatching {
                EventAckCodec.acknowledge(EventAckCodec.request("project", "session", "2").apply(change),
                    "device", "project", "session", "2") { exchanges++; receipt() }
            }.exceptionOrNull() is zenpi.transport.WireException)
            assertEquals(0, exchanges)
        }
    }
}
