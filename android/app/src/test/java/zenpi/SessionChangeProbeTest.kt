package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class SessionChangeProbeTest {
    private fun request(type: String, operation: String? = null) = JsonObject().apply {
        addProperty("schema_version", 1); addProperty("channel", "control")
        addProperty("id", "original"); addProperty("project_id", "project"); addProperty("session_id", "old")
        add("payload", JsonObject().apply {
            addProperty("type", type)
            if (type in setOf("ack", "resume", "subscribe")) {
                add("cursor", JsonParser.parseString("""{"session_id":"old","next_sequence":0}"""))
                if (type != "ack") addProperty("limit", 128)
                if (type == "subscribe") addProperty("duration_ms", 1000)
            }
            operation?.let { addProperty("operation", it); add("arguments", JsonObject()) }
        })
    }
    @Test fun malformedOrAuthorityBearingRefusedRequestNeverStartsRecoveryProbe() = runBlocking {
        for (change in listOf<(JsonObject) -> Unit>(
            { it.remove("schema_version") }, { it.addProperty("schema_version", 2) },
            { it.addProperty("required_owner", "a".repeat(64)) }, { it.addProperty("expected_revision", 0) },
            { it.addProperty("turn_id", "turn") }, { it.add("project_id", com.google.gson.JsonNull.INSTANCE) },
            { it.addProperty("project_id", 123) }, { it.addProperty("session_id", "bad\n") },
            { it.add("id", com.google.gson.JsonNull.INSTANCE) },
            { it.getAsJsonObject("payload").getAsJsonObject("cursor").addProperty("session_id", "foreign") },
            { it.getAsJsonObject("payload").getAsJsonObject("cursor").addProperty("next_sequence", -1) })) {
            val original = HostRefusedException("owner_mismatch"); var probes = 0
            assertSame(original, SessionChangeProbe.resolve(request("resume").apply(change), original, true) { probes++; listing() })
            assertEquals(0, probes)
        }
        val original = HostRefusedException("owner_mismatch")
        val explicit = request("request", "inspect").apply { getAsJsonObject("payload").getAsJsonObject("arguments").addProperty("path", "file") }
        assertSame(original, SessionChangeProbe.resolve(explicit, original, true) { error("Explicit read must not probe") })
    }
    private fun listing(session: String = "new") = JsonParser.parseString("""{"mutations_available":false,"active_project_id":"project","projects":[{"id":"project","title":"Workspace","session_id":"$session","workspace_control":false}]}""").asJsonObject
    @Test fun ackResumeAndInitialReadRacesUseOneNewListingWithoutRetryingTheRefusedRequest() = runBlocking {
        for (request in listOf(request("ack"), request("resume"), request("request", "inspect"), request("request", "subtab_list"))) {
            val before = request.deepCopy(); var probes = 0
            val error = SessionChangeProbe.resolve(request, HostRefusedException("owner_mismatch"), true) { probe ->
                probes++
                assertFalse(probe.has("project_id")); assertFalse(probe.has("session_id"))
                assertNotEquals(request["id"], probe["id"])
                assertEquals("workspace_list", probe.getAsJsonObject("payload")["operation"].asString)
                listing()
            }
            assertTrue(error is SessionChangedException)
            assertEquals("project", (error as SessionChangedException).projectId); assertEquals("old", error.previousSessionId)
            assertEquals(1, probes); assertEquals(before, request)
        }
    }
    @Test fun mutationExplicitReadAndNonOwnershipRefusalsNeverProbe() = runBlocking {
        for ((request, code, automatic) in listOf(
            Triple(request("request", "prompt"), "owner_mismatch", true),
            Triple(request("request", "session_new"), "owner_mismatch", true),
            Triple(request("request", "workspace_list"), "owner_mismatch", true),
            Triple(request("subscribe"), "owner_mismatch", true),
            Triple(request("resume"), "owner_mismatch", false),
            Triple(request("ack"), "unauthorized", true), Triple(request("resume"), "revoked_device", true))) {
            val original = HostRefusedException(code)
            assertSame(original, SessionChangeProbe.resolve(request, original, automatic) { error("Must not probe") })
        }
    }
    @Test fun missingOrUnchangedAuthorizedProjectKeepsOriginalAuthorityRefusal() = runBlocking {
        val original = HostRefusedException("owner_mismatch")
        for (value in listOf(listing("old"), JsonParser.parseString("""{"mutations_available":false,"active_project_id":null,"projects":[]}""").asJsonObject))
            assertSame(original, SessionChangeProbe.resolve(request("resume"), original, true) { value })
    }
    @Test fun revokedMalformedOrMismatchedProbeCannotTriggerSessionRecovery() = runBlocking {
        val revoked = HostRefusedException("revoked_device")
        val result = runCatching { SessionChangeProbe.resolve(request("ack"), HostRefusedException("owner_mismatch"), true) { throw revoked } }
        assertSame(revoked, result.exceptionOrNull())
        for (value in listOf(listing().apply { addProperty("active_project_id", "foreign") },
            listing().apply { getAsJsonArray("projects").add(getAsJsonArray("projects")[0].deepCopy()) },
            listing().apply { getAsJsonArray("projects")[0].asJsonObject.addProperty("session_id", "bad\n") })) {
            assertTrue(runCatching { SessionChangeProbe.resolve(request("resume"), HostRefusedException("owner_mismatch"), true) { value } }.isFailure)
        }
    }
    @Test fun refusedReplayFramesStayUnpublishedWhileFreshListingClassifiesTheRace() = runBlocking {
        val request = request("resume")
        val failure = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"original","project_id":"project","session_id":"old","code":"owner_mismatch","execution_state":"failed","payload":{"type":"response","result":null}}""").asJsonObject
        val event = JsonParser.parseString("""{"schema_version":1,"channel":"event","project_id":"project","session_id":"old","turn_id":"late","sequence":0,"payload":{"type":"event","name":"transcript_turn","data":{"id":"late","role":"assistant","content":"unconfirmed"}}}""").asJsonObject
        val bytes = java.io.ByteArrayOutputStream().also { FrameCodec.write(it, event); FrameCodec.write(it, failure) }.toByteArray()
        var publications = 0; var probes = 0
        val error = runCatching {
            try {
                ResponseReader.read(java.io.ByteArrayInputStream(bytes), request,
                    { throw HostRefusedException(FrameCodec.string(it, "code")) }) { publications++ }
            } catch (refused: HostRefusedException) {
                throw SessionChangeProbe.resolve(request, refused, true) { probes++; listing() }
            }
        }.exceptionOrNull()
        assertTrue(error is SessionChangedException); assertEquals(0, publications); assertEquals(1, probes)
    }
}
