package zenpi

import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class WorkerTranscriptTest {
    private val tab = "b".repeat(64)
    private val status = RemoteWorkerStatus("task", "project", "session", tab, "succeeded", outputText = "preview", outputTruncated = true)
    private val projection = HostProjection(listOf(RemoteProject("project", "Project", "session", workerStatuses = listOf(status))),
        "project", "session", "idle", emptyList(), "1", true, "1")
    private fun page(offset: Int = 0, text: String = "abc") = JsonObject().apply {
        addProperty("project_id", "project"); addProperty("session_id", "session"); addProperty("turn_id", "answer")
        addProperty("request_id", "task"); addProperty("subtab_id", tab); addProperty("role", "assistant")
        addProperty("digest", "a".repeat(64)); addProperty("offset", offset); addProperty("next_offset", offset + text.toByteArray().size)
        addProperty("total_bytes", 6); addProperty("has_more", offset + text.toByteArray().size < 6); addProperty("text", text)
    }
    @Test fun workerPagesBindRequestSubtabAndActualTurnWithoutMasterTranscriptMembership() {
        val first = TranscriptPageCodec.decode(page(), projection, "task", null, true)
        assertEquals("answer", first.turnId); assertEquals("task", first.workerRequestId); assertEquals(tab, first.subtabId)
        val request = TranscriptPageCodec.request(projection, "task", first, true)
        FrameCodec.decode(request.toString().toByteArray())
        val args = request["payload"].asJsonObject["arguments"].asJsonObject
        assertEquals(setOf("request_id", "offset", "digest"), args.keySet())
        assertFalse(request.has("required_owner")); assertFalse(request.has("expected_revision"))
        val second = TranscriptPageCodec.decode(page(3, "def"), projection, "task", first, true)
        assertFalse(second.hasMore)
        assertEquals(first, TranscriptNavigation.previous(TranscriptNavigation.remember(emptyList(), first, second), second))
    }
    @Test fun foreignTaskSubtabTurnDigestAndRoleNeverBecomeWorkerPages() {
        val first = TranscriptPageCodec.decode(page(), projection, "task", null, true)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("request_id", "foreign") }, { it.addProperty("subtab_id", "c".repeat(64)) },
            { it.addProperty("turn_id", "foreign") }, { it.addProperty("role", "user") },
            { it.addProperty("digest", "c".repeat(64)) }, { it.remove("request_id") },
            { it.addProperty("extra", true) }, { it.addProperty("session_id", "foreign") })) {
            assertTrue(runCatching { TranscriptPageCodec.decode(page(3, "def").apply(change), projection, "task", first, true) }.isFailure)
        }
        assertTrue(runCatching { TranscriptPageCodec.request(projection, "task", first) }.isFailure)
        assertTrue(runCatching { TranscriptNavigation.previous(listOf(first), first.copy(offset = 3, workerRequestId = "foreign")) }.isFailure)
    }
    @Test fun missingChangedOrUnconfirmedTasksInvalidateOpenPages() {
        assertTrue(TranscriptPageCodec.retainsTurn(projection, projection, "task", true))
        for (next in listOf(null, projection.copy(sessionId = "foreign"), projection.copy(selectedId = "foreign"),
            projection.copy(projects = emptyList()), projection.copy(projects = projection.projects.map { it.copy(workerStatuses = emptyList()) }),
            projection.copy(projects = projection.projects.map { it.copy(workerStatuses = listOf(status.copy(state = "running"))) }),
            projection.copy(projects = projection.projects.map { it.copy(workerStatuses = listOf(status.copy(subtabId = "c".repeat(64)))) }))) {
            assertFalse(TranscriptPageCodec.retainsTurn(projection, next, "task", true))
        }
        assertTrue(runCatching { TranscriptPageCodec.request(projection, "unknown", null, true) }.isFailure)
        val fenced = zenpi.pairing.AuthorityLossPolicy.displayOnly(zenpi.pairing.PairingState(
            projection = projection, transcriptTurn = "task", transcriptWorker = true,
            transcriptPage = TranscriptPageCodec.decode(page(), projection, "task", null, true)))
        assertNull(fenced.transcriptTurn); assertNull(fenced.transcriptPage); assertFalse(fenced.transcriptWorker)
    }
    @Test fun continuationCannotReuseAPageFromAnotherSubtabOrMalformedPriorContent() {
        val first = TranscriptPageCodec.decode(page(), projection, "task", null, true)
        val changed = projection.copy(projects = projection.projects.map { it.copy(workerStatuses = listOf(status.copy(subtabId = "c".repeat(64)))) })
        assertTrue(runCatching { TranscriptPageCodec.request(changed, "task", first, true) }.isFailure)
        for (bad in listOf(first.copy(subtabId = null), first.copy(role = "user"), first.copy(digest = "bad"),
            first.copy(text = "different"), first.copy(nextOffset = 2), first.copy(totalBytes = 2))) {
            assertTrue(runCatching { TranscriptPageCodec.request(projection, "task", bad, true) }.isFailure)
            assertTrue(runCatching { TranscriptPageCodec.decode(page(3, "def"), projection, "task", bad, true) }.isFailure)
        }
    }
}
