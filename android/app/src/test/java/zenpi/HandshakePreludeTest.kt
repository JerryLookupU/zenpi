package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.DataOutputStream
import zenpi.transport.*
import zenpi.pairing.AuthorityLossPolicy
import zenpi.lifecycle.ResumeRetryPolicy

class HandshakePreludeTest {
    private val fingerprint = "a".repeat(64)
    private val nonce = "c".repeat(43)
    private fun welcome() = JsonParser.parseString("""{"schema_version":1,"channel":"control","payload":{"type":"welcome","selected_version":1}}""").asJsonObject
    private fun challenge() = JsonParser.parseString("""{"schema_version":1,"channel":"control","payload":{"type":"challenge","nonce":"$nonce","host_fingerprint":"$fingerprint"}}""").asJsonObject
    private fun wire(vararg frames: JsonObject) = ByteArrayOutputStream().also { output ->
        frames.forEach { frame ->
            val bytes = frame.toString().toByteArray(Charsets.UTF_8)
            DataOutputStream(output).apply { writeInt(bytes.size); write(bytes) }
        }
    }.toByteArray()
    private class Fragmented(bytes: ByteArray) : ByteArrayInputStream(bytes) {
        override fun read(bytes: ByteArray, offset: Int, length: Int) = super.read(bytes, offset, minOf(length, 2))
    }
    @Test fun exactConnectionNegotiationYieldsNonceWithoutConsumingFollowingResponse() {
        val input = Fragmented(wire(welcome(), challenge(), welcome()))
        HandshakePrelude.welcome(input)
        assertEquals(nonce, HandshakePrelude.challenge(input, fingerprint))
        assertEquals(welcome(), FrameCodec.read(input))
    }
    @Test fun nonCanonicalChallengeNeverReachesDeviceSigningAndFencesReconnect() {
        val canonical = java.util.Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32))
        for (suffix in listOf('B', 'C', 'D')) {
            val malformed = challenge().apply { getAsJsonObject("payload").addProperty("nonce", canonical.dropLast(1) + suffix) }
            var signs = 0
            val error = runCatching {
                val input = Fragmented(wire(welcome(), malformed))
                HandshakePrelude.welcome(input)
                HandshakePrelude.challenge(input, fingerprint)
                signs++
            }.exceptionOrNull() as Exception
            assertEquals(0, signs); assertTrue(error is HostNegotiationException)
            assertTrue(AuthorityLossPolicy.refused(error)); assertTrue(ResumeRetryPolicy.permanent(error))
        }
    }
    @Test fun reauthenticationRefusesRoutedPreludeBeforeAnyProofOrRequest() {
        for (field in listOf("id", "project_id", "session_id", "turn_id", "required_owner", "expected_revision", "sequence", "code", "execution_state")) {
            for (raw in listOf("null", "\"injected\"")) for (onWelcome in listOf(true, false)) {
                val first = welcome(); val second = challenge()
                (if (onWelcome) first else second).add(field, JsonParser.parseString(raw))
                var proofCalls = 0
                val error = runCatching {
                    val input = Fragmented(wire(first, second))
                    HandshakePrelude.welcome(input); HandshakePrelude.challenge(input, fingerprint)
                    proofCalls++
                }.exceptionOrNull()
                assertTrue(error is WireException); assertEquals(0, proofCalls)
            }
        }
    }
    @Test fun wrongOrderHostVersionChannelAndMalformedNonceFailClosed() {
        for (first in listOf(challenge(), welcome().apply { addProperty("channel", "bulk") },
            welcome().apply { getAsJsonObject("payload").addProperty("selected_version", "1") },
            welcome().apply { getAsJsonObject("payload").addProperty("selected_version", 2) })) {
            assertTrue(runCatching { HandshakePrelude.welcome(Fragmented(wire(first))) }.exceptionOrNull() is WireException)
        }
        for (second in listOf(welcome(), challenge().apply { addProperty("channel", "event") },
            challenge().apply { getAsJsonObject("payload").addProperty("host_fingerprint", "b".repeat(64)) },
            challenge().apply { getAsJsonObject("payload").addProperty("nonce", "bad") })) {
            assertTrue(runCatching { HandshakePrelude.challenge(Fragmented(wire(second)), fingerprint) }.exceptionOrNull() is WireException)
        }
    }
    @Test fun truncatedPreludeNeverReachesSigningBoundary() {
        val bytes = wire(welcome(), challenge())
        for (cut in 0 until bytes.size) {
            var proofCalls = 0
            assertTrue(runCatching {
                val input = Fragmented(bytes.copyOf(cut))
                HandshakePrelude.welcome(input); HandshakePrelude.challenge(input, fingerprint); proofCalls++
            }.isFailure)
            assertEquals(0, proofCalls)
        }
    }
    @Test fun invalidNegotiationFencesOldAuthorityAndIsNotABackgroundRetry() {
        val failures = mutableListOf<Throwable>()
        for (protocol in listOf(null, "", "h2", "zenpi/2", "zenpi/1\n")) {
            failures += requireNotNull(runCatching { HandshakePrelude.applicationProtocol(protocol) }.exceptionOrNull())
        }
        HandshakePrelude.applicationProtocol("zenpi/1")
        for (first in listOf(challenge(), welcome().apply { addProperty("id", "foreign") },
            welcome().apply { getAsJsonObject("payload").addProperty("selected_version", 2) },
            welcome().apply { getAsJsonObject("payload").addProperty("selected_version", "1") })) {
            failures += requireNotNull(runCatching { HandshakePrelude.welcome(Fragmented(wire(first))) }.exceptionOrNull())
        }
        failures += requireNotNull(runCatching {
            HandshakePrelude.challenge(Fragmented(wire(challenge().apply {
                getAsJsonObject("payload").addProperty("host_fingerprint", "b".repeat(64))
            })), fingerprint)
        }.exceptionOrNull())
        for (failure in failures) {
            assertTrue(failure is HostNegotiationException)
            assertEquals("Host protocol negotiation refused", failure.message)
            for (error in listOf(failure as Exception, java.io.IOException("wrapper", failure))) {
                assertTrue(HostNegotiationException.refused(error))
                assertTrue(AuthorityLossPolicy.refused(error))
                assertTrue(ResumeRetryPolicy.permanent(error))
            }
        }
    }
    @Test fun incompleteOrTimedOutNegotiationIsTransientAndDiagnosticTextCannotGrantClassification() {
        val failures = mutableListOf<Exception>(WireException("Host protocol negotiation refused"),
            java.net.SocketTimeoutException("timeout"), java.io.IOException("Host protocol negotiation refused"))
        val bytes = wire(welcome())
        for (cut in 0 until bytes.size) {
            failures += requireNotNull(runCatching { HandshakePrelude.welcome(Fragmented(bytes.copyOf(cut))) }.exceptionOrNull()) as Exception
        }
        val cycle = java.io.IOException("cycle")
        val second = java.io.IOException("second", cycle)
        cycle.initCause(second); failures += cycle
        for (error in failures) {
            assertFalse(HostNegotiationException.refused(error))
            assertFalse(AuthorityLossPolicy.refused(error))
            assertFalse(ResumeRetryPolicy.permanent(error))
        }
    }
}
