package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import java.io.*
import java.util.Base64
import zenpi.pairing.PairingInvite
import zenpi.transport.*

class PairingHandshakeTest {
    @Test fun nonCanonicalChallengeCannotSendOneUsePairingProof() {
        val output = ByteArrayOutputStream()
        val malformed = challenge().apply { getAsJsonObject("payload").addProperty("nonce", "A".repeat(42) + "B") }
        var signs = 0
        val error = runCatching {
            exchange(ByteArrayInputStream(rawWire(welcome(), malformed)), output, sign = { signs++; "proof" })
        }.exceptionOrNull()
        assertTrue(error is HostNegotiationException); assertEquals(0, signs)
        val sent = ByteArrayInputStream(output.toByteArray())
        assertEquals("hello", FrameCodec.read(sent)?.getAsJsonObject("payload")?.get("type")?.asString)
        assertNull(FrameCodec.read(sent))
    }
    private val fingerprint = "a".repeat(64)
    private val device = "b".repeat(64)
    private val token = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32))
    private val public = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(65).apply { this[0] = 4 })
    private val invite get() = PairingInvite.parse("zenpi://pair?v=1&address=192.168.1.20%3A7777&fingerprint=$fingerprint&token=$token&expires=1060", 1000) { 0 }
    private fun welcome() = JsonParser.parseString("""{"schema_version":1,"channel":"control","payload":{"type":"welcome","selected_version":1}}""").asJsonObject
    private fun challenge() = JsonParser.parseString("""{"schema_version":1,"channel":"control","payload":{"type":"challenge","nonce":"${"c".repeat(43)}","host_fingerprint":"$fingerprint"}}""").asJsonObject
    private fun response(id: String) = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"$id","execution_state":"succeeded","payload":{"type":"response","result":{"paired":true,"device_registered":true,"control_available":false,"device_id":"$device","device_revision":1}}}""").asJsonObject
    private fun wire(vararg values: JsonObject) = ByteArrayOutputStream().also { out -> values.forEach { FrameCodec.write(out, it) } }.toByteArray()
    /** Encodes hostile inbound bytes without running the outbound validator first. */
    private fun rawWire(vararg values: JsonObject) = ByteArrayOutputStream().also { out ->
        values.forEach {
            val bytes = it.toString().toByteArray(Charsets.UTF_8)
            DataOutputStream(out).apply { writeInt(bytes.size); write(bytes) }
        }
    }.toByteArray()
    private class Fixture(val prelude: ByteArray, val output: ByteArrayOutputStream,
        val makeResponse: (String) -> ByteArray) : InputStream() {
        private var bytes = ByteArrayInputStream(prelude)
        private var finalRead = false
        override fun read(): Int {
            val value = bytes.read()
            if (value != -1 || finalRead) return value
            finalRead = true
            val sent = ByteArrayInputStream(output.toByteArray())
            FrameCodec.read(sent)
            val pair = FrameCodec.read(sent) ?: error("Pair not sent")
            bytes = ByteArrayInputStream(makeResponse(FrameCodec.string(pair, "id")))
            return bytes.read()
        }
        override fun read(target: ByteArray, offset: Int, length: Int): Int {
            if (length == 0) return 0
            val value = read(); if (value == -1) return -1
            target[offset] = value.toByte(); return 1
        }
    }
    private fun sent(output: ByteArrayOutputStream): List<JsonObject> {
        val input = ByteArrayInputStream(output.toByteArray())
        val values = mutableListOf<JsonObject>()
        while (true) values.add(FrameCodec.read(input) ?: break)
        return values
    }
    private fun exchange(input: InputStream, output: ByteArrayOutputStream, now: () -> Long = { 1000 },
        sign: (String) -> String = { "proof" }, active: () -> Unit = {}): PairingResult =
        PairingHandshake.exchange(input, output, invite, "Phone", public, device, sign, active, now)
    @Test fun hostReportedControlAfterPairingIsPropagated() {
        val output = ByteArrayOutputStream()
        val result = exchange(Fixture(wire(welcome(), challenge()), output) {
            wire(response(it).apply { getAsJsonObject("payload").getAsJsonObject("result").addProperty("control_available", true) })
        }, output)
        assertTrue(result.paired); assertTrue(result.controlAvailable)
        assertFalse(sent(output)[1]["payload"].asJsonObject.has("safety_code"))
    }
    @Test fun fragmentedHandshakeSendsOneBoundPairAndConfirmsReadOnlyRegistration() {
        val output = ByteArrayOutputStream()
        var signed: String? = null
        val result = exchange(Fixture(wire(welcome(), challenge()), output) { wire(response(it)) }, output,
            sign = { signed = it; "proof" })
        assertTrue(result.paired); assertTrue(result.deviceRegistered); assertFalse(result.controlAvailable)
        val frames = sent(output); assertEquals(2, frames.size)
        assertEquals("hello", frames[0]["payload"].asJsonObject["type"].asString)
        val pair = frames[1]
        assertEquals(signed, FrameCodec.string(pair, "id")); assertEquals(token, pair["payload"].asJsonObject["token"].asString)
        assertEquals(public, pair["payload"].asJsonObject["public_key"].asString)
    }
    @Test fun pairingIdentityLossBeforeOrDuringSigningNeverSendsTheInvitation() {
        for (failAt in 1..4) {
            val output = ByteArrayOutputStream(); var checks = 0; var signs = 0
            val error = runCatching {
                PairingHandshake.exchange(ByteArrayInputStream(wire(welcome(), challenge())), output, invite,
                    "Phone", public, device, sign = { signs++; "proof" }, now = { 1000 },
                    checkIdentity = { if (++checks == failAt) throw HostAuthenticationException() })
            }.exceptionOrNull()
            assertTrue(error is HostAuthenticationException)
            assertEquals(if (failAt == 1) 0 else 1, sent(output).size)
            assertEquals(if (failAt == 4) 1 else 0, signs)
            assertFalse(output.toString("UTF-8").contains(token))
        }
    }
    @Test fun identityLossWhileAwaitingRegistrationReceiptCannotPublishPairingSuccess() {
        val output = ByteArrayOutputStream(); var changed = false; var published = false
        val input = Fixture(wire(welcome(), challenge()), output) { changed = true; wire(response(it)) }
        val error = runCatching {
            PairingHandshake.exchange(input, output, invite, "Phone", public, device,
                sign = { "proof" }, now = { 1000 }, checkIdentity = { if (changed) throw HostAuthenticationException() })
            published = true
        }.exceptionOrNull()
        assertTrue(error is HostAuthenticationException); assertFalse(published)
        assertEquals(2, sent(output).size)
    }
    @Test fun cancellationInsideEachIdentityReadRefusesTheNextHandshakeStage() {
        for (cancelAt in 1..5) {
            val output = ByteArrayOutputStream()
            val cancelled = kotlinx.coroutines.CancellationException("connection owner replaced")
            var active = true; var reads = 0; var signs = 0; var published = false
            val error = runCatching {
                PairingHandshake.exchange(Fixture(wire(welcome(), challenge()), output) { wire(response(it)) },
                    output, invite, "Phone", public, device,
                    sign = { signs++; "proof" }, now = { 1000 },
                    checkActive = { if (!active) throw cancelled },
                    checkIdentity = { if (++reads == cancelAt) active = false })
                published = true
            }.exceptionOrNull()
            assertSame("identity read $cancelAt", cancelled, error)
            assertFalse(published); assertEquals(cancelAt, reads)
            assertEquals(if (cancelAt >= 4) 1 else 0, signs)
            assertEquals(when (cancelAt) { 1 -> 0; 5 -> 2; else -> 1 }, sent(output).size)
            if (cancelAt < 5) assertFalse(output.toString("UTF-8").contains(token))
        }
    }
    @Test fun monotonicExpiryWhileSigningNeverSendsTokenEvenWithRolledBackWallClock() {
        var elapsed = 0L
        val captured = PairingInvite.parse("zenpi://pair?v=1&address=192.168.1.20%3A7777&fingerprint=$fingerprint&token=$token&expires=1060", 1000) { elapsed }
        val output = ByteArrayOutputStream()
        assertTrue(runCatching {
            PairingHandshake.exchange(ByteArrayInputStream(wire(welcome(), challenge())), output, captured,
                "Phone", public, device, sign = { elapsed = 60_000_000_000; "proof" }, now = { 990 })
        }.isFailure)
        assertEquals(1, sent(output).size)
        assertFalse(output.toString("UTF-8").contains(token))
        assertTrue(runCatching { captured.requireFresh(1000) }.isFailure)
    }
    @Test fun expiryDuringWelcomeChallengeOrSigningNeverSendsToken() {
        for (expireAt in 2..4) {
            val output = ByteArrayOutputStream()
            var checks = 0
            assertTrue(runCatching {
                exchange(ByteArrayInputStream(wire(welcome(), challenge())), output,
                    now = { checks++; if (checks >= expireAt) 1060 else 1000 })
            }.isFailure)
            assertEquals(1, sent(output).size); assertFalse(output.toString("UTF-8").contains(token))
        }
    }
    @Test fun fingerprintMismatchOrWrongWelcomeNeverSignsOrSendsPair() {
        for (prelude in listOf(wire(challenge()), wire(welcome(), challenge().apply {
            getAsJsonObject("payload").addProperty("host_fingerprint", "d".repeat(64))
        }))) {
            val output = ByteArrayOutputStream()
            assertTrue(runCatching { exchange(ByteArrayInputStream(prelude), output, sign = { fail("must not sign"); "proof" }) }.isFailure)
            assertEquals(1, sent(output).size)
        }
    }
    @Test fun unsupportedWelcomeVersionNeverSendsPair() {
        val output = ByteArrayOutputStream()
        val unsupported = welcome().apply { getAsJsonObject("payload").addProperty("selected_version", 2) }
        assertTrue(runCatching { exchange(ByteArrayInputStream(rawWire(unsupported, challenge())), output, sign = { fail("must not sign"); "proof" }) }.isFailure)
        assertEquals(1, sent(output).size)
    }
    @Test fun malformedChallengeNonceNeverSignsOrSendsPair() {
        for (nonce in listOf("", "c".repeat(42), "c".repeat(44), "c".repeat(42) + "!")) {
            val output = ByteArrayOutputStream()
            val malformed = challenge().apply { getAsJsonObject("payload").addProperty("nonce", nonce) }
            assertTrue(runCatching { exchange(ByteArrayInputStream(rawWire(welcome(), malformed)), output, sign = { fail("must not sign"); "proof" }) }.isFailure)
            assertEquals(1, sent(output).size)
        }
    }
    @Test fun routedOrIdentifiedNegotiationNeverSignsOrSendsAnInvitation() {
        for (field in listOf("id", "project_id", "session_id", "turn_id", "required_owner", "expected_revision", "sequence", "code", "execution_state")) {
            for (value in listOf(com.google.gson.JsonNull.INSTANCE, com.google.gson.JsonPrimitive("foreign"))) {
                for (onWelcome in listOf(true, false)) {
                    val prelude = if (onWelcome) rawWire(welcome().apply { add(field, value) }, challenge())
                        else rawWire(welcome(), challenge().apply { add(field, value) })
                    val output = ByteArrayOutputStream()
                    val error = runCatching { exchange(ByteArrayInputStream(prelude), output,
                        sign = { fail("Invalid negotiation must not reach signing"); "proof" }) }.exceptionOrNull()
                    assertTrue("$field welcome=$onWelcome", error is WireException)
                    assertEquals(1, sent(output).size)
                    assertFalse(output.toString("UTF-8").contains(token))
                }
            }
        }
    }
    @Test fun pairingResponseRoutingFieldsAreRejected() {
        val output = ByteArrayOutputStream()
        val fixture = Fixture(wire(welcome(), challenge()), output) { id -> wire(response(id).apply { addProperty("session_id", "foreign") }) }
        assertTrue(runCatching { exchange(fixture, output) }.isFailure)
        assertEquals(2, sent(output).size)
    }
    @Test fun refusedPairingWithProtocolErrorCodeIsHandledAsRefusal() {
        val output = ByteArrayOutputStream()
        val fixture = Fixture(wire(welcome(), challenge()), output) { id ->
            wire(response(id).apply {
                addProperty("execution_state", "failed")
                addProperty("code", "unauthorized")
            })
        }
        val error = runCatching { exchange(fixture, output) }.exceptionOrNull()
        assertTrue(error is WireException)
        assertEquals(2, sent(output).size)
    }
    @Test fun successfulPairingCannotSmuggleAnErrorCode() {
        val output = ByteArrayOutputStream()
        val fixture = Fixture(wire(welcome(), challenge()), output) { id -> wire(response(id).apply { addProperty("code", "busy") }) }
        assertTrue(runCatching { exchange(fixture, output) }.isFailure)
    }
    @Test fun pairingReceiptWithNonObjectResultFailsClosedBeforeSuccess() {
        val output = ByteArrayOutputStream()
        val fixture = Fixture(wire(welcome(), challenge()), output) { id ->
            val malformed = response(id)
            malformed["payload"].asJsonObject.add("result", com.google.gson.JsonNull.INSTANCE)
            wire(malformed)
        }
        val error = runCatching { exchange(fixture, output) }.exceptionOrNull()
        assertTrue(error is WireException)
        assertEquals(2, sent(output).size)
    }
    @Test fun malformedRefusedOrUnconfirmedReceiptsAreNotSuccessfulPairing() {
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("id", "foreign") },
            { it.addProperty("execution_state", "failed"); it.addProperty("code", "unauthorized") },
            { it.getAsJsonObject("payload").getAsJsonObject("result").addProperty("paired", false) },
            { it.getAsJsonObject("payload").getAsJsonObject("result").addProperty("device_registered", false) },
            { it.getAsJsonObject("payload").getAsJsonObject("result").addProperty("control_available", "false") },
            { it.getAsJsonObject("payload").getAsJsonObject("result").addProperty("device_id", "foreign") },
            { it.getAsJsonObject("payload").getAsJsonObject("result").addProperty("device_revision", "1") },
            { it.getAsJsonObject("payload").getAsJsonObject("result").addProperty("device_revision", 0) },
            { it.getAsJsonObject("payload").getAsJsonObject("result").addProperty("extra", true) })) {
            val output = ByteArrayOutputStream()
            assertTrue(runCatching { exchange(Fixture(wire(welcome(), challenge()), output) { wire(response(it).apply(change)) }, output) }.isFailure)
            assertEquals(2, sent(output).size)
        }
    }
    @Test fun everyResponseTruncationLeavesUnknownOutcomeWithoutResendingInvite() {
        val sample = wire(response("00000000-0000-0000-0000-000000000000"))
        for (cut in 0 until sample.size) {
            val output = ByteArrayOutputStream()
            assertTrue(runCatching {
                exchange(Fixture(wire(welcome(), challenge()), output) { wire(response(it)).copyOf(cut) }, output)
            }.isFailure)
            assertEquals(2, sent(output).size)
        }
    }
    @Test fun acceptedReceiptAfterExpiryRemainsValidButCancellationDoesNotPublishIt() {
        val output = ByteArrayOutputStream()
        var time = 1000L
        assertTrue(exchange(Fixture(wire(welcome(), challenge()), output) { time = 1061; wire(response(it)) }, output, now = { time }).paired)
        val cancelled = ByteArrayOutputStream()
        var receiptReceived = false
        val failure = kotlinx.coroutines.CancellationException("replaced")
        assertSame(failure, runCatching {
            exchange(Fixture(wire(welcome(), challenge()), cancelled) { receiptReceived = true; wire(response(it)) }, cancelled,
                active = { if (receiptReceived) throw failure })
        }.exceptionOrNull())
        assertEquals(2, sent(cancelled).size)
    }
    @Test fun legacyReceiptWithoutRevisionIsCompatibleAndInvalidNameNeverSendsHello() {
        val output = ByteArrayOutputStream()
        assertTrue(exchange(Fixture(wire(welcome(), challenge()), output) {
            wire(response(it).apply { getAsJsonObject("payload").getAsJsonObject("result").remove("device_revision") })
        }, output).paired)
        for (name in listOf("", "bad\nname", "x".repeat(129))) {
            val refused = ByteArrayOutputStream()
            assertTrue(runCatching {
                PairingHandshake.exchange(ByteArrayInputStream(byteArrayOf()), refused, invite,
                    name, public, device, { "proof" }, now = { 1000 })
            }.isFailure)
            assertEquals(0, refused.size())
        }
    }
    @Test fun cancellationAndSigningFailureBeforePairNeverLeakInvitation() {
        for (cancelAt in 1..8) {
            val output = ByteArrayOutputStream()
            var checks = 0
            val failure = kotlinx.coroutines.CancellationException("replaced")
            assertSame(failure, runCatching {
                exchange(ByteArrayInputStream(wire(welcome(), challenge())), output,
                    active = { if (++checks == cancelAt) throw failure })
            }.exceptionOrNull())
            assertEquals(if (cancelAt <= 2) 0 else 1, sent(output).size)
            assertFalse(output.toString("UTF-8").contains(token))
        }
        val output = ByteArrayOutputStream()
        val failure = IOException("key unavailable")
        assertSame(failure, runCatching {
            exchange(ByteArrayInputStream(wire(welcome(), challenge())), output, sign = { throw failure })
        }.exceptionOrNull())
        assertEquals(1, sent(output).size)
    }
}
