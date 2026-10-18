package zenpi

import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Assume.assumeNotNull
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*
import java.io.ByteArrayInputStream
import java.io.File
import java.util.Base64

class AndroidRustHistoryTest {
    @Test fun readOnlyConversationPagesUseActualDesktopJournal() = runBlocking {
        val phase = System.getProperty("zenpi.history.phase")
        val firstId = "history:0/\u4e2d"
        assumeNotNull(phase)
        fun file(name: String) = File(requireNotNull(System.getProperty("zenpi.history.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val listing = seed.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing)
        val selected = requireNotNull(HostProjectionCodec.select(listing, projects, null))
        val projection = HostProjectionCodec.workspace(
            HostProjectionCodec.snapshot(projects, selected, seed.getAsJsonObject("snapshot")), listing)
        assertFalse(projects.single().promptControl)
        assertFalse(projection.turns.any { it.id == firstId })
        val ambiguous = projection.copy(projects = projects + selected.copy(sessionId = "foreign"))
        assertFalse(SessionHistoryCodec.readable(ambiguous))
        assertTrue(runCatching { SessionHistoryCodec.request(ambiguous, null) }.isFailure)
        if (phase == "generate_detail" || phase == "validate_detail") {
            val first = SessionHistoryCodec.decode(seed.getAsJsonObject("first"), projection, null)
            assertFalse(SessionHistoryCodec.sameScope(first, ambiguous))
            assertTrue(runCatching { SessionHistoryCodec.decode(seed.getAsJsonObject("first"), ambiguous, null) }.isFailure)
            val turn = first.turns.single { it.id == firstId }
            assertTrue(turn.contentTruncated)
            var state = PairingState(phase = PairingPhase.CONNECTED, projection = projection,
                sessionHistoryOpen = true, sessionHistory = first, transcriptTurn = turn.id, transcriptHistoryTurn = turn)
            val readProjection = requireNotNull(SessionHistoryReadPolicy.projection(state, projection))
            var current: TranscriptPage? = null
            var retained = emptyList<TranscriptPage>()
            val joined = StringBuilder()
            for (raw in seed.getAsJsonArray("details")) {
                val next = TranscriptPageCodec.decode(raw.asJsonObject, readProjection, turn.id, current,
                    journalRevision = first.journalRevision)
                assertFalse(TranscriptNavigation.validChain(emptyList(), next.copy(journalRevision = "01")))
                assertFalse(TranscriptNavigation.validChain(emptyList(), next.copy(toolName = "unpaired-tool")))
                retained = TranscriptNavigation.remember(retained, current, next); current = next; joined.append(next.text)
            }
            if (phase == "generate_detail") {
                file("command").outputStream().use { FrameCodec.write(it, TranscriptPageCodec.request(readProjection, turn.id, current,
                    journalRevision = first.journalRevision)) }
            } else {
                val results = JsonParser.parseString(file("results").readText()).asJsonObject
                for (raw in results.getAsJsonArray("exchanges")) {
                    val exchange = raw.asJsonObject
                    val bytes = Base64.getDecoder().decode(exchange["wire_base64"].asString)
                    val input = object : ByteArrayInputStream(bytes) {
                        override fun read(target: ByteArray, offset: Int, length: Int): Int = super.read(target, offset, minOf(length, 3))
                    }
                    val response = runCatching { ResponseReader.read(input, exchange.getAsJsonObject("request"), {
                        if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
                    }) }
                    val code = exchange["code"].takeUnless { it.isJsonNull }?.asString
                    if (code != null) {
                        assertTrue(response.exceptionOrNull() is HostRefusedException)
                        assertEquals(code, (response.exceptionOrNull() as HostRefusedException).code)
                    } else {
                        val value = response.getOrThrow().result
                        val next = TranscriptPageCodec.decode(value, readProjection, turn.id, current, journalRevision = first.journalRevision)
                        val chain = TranscriptNavigation.remember(retained, current, next)
                        state = state.copy(transcriptPage = next, transcriptPreviousPages = chain)
                        assertTrue(TranscriptReadPolicy.navigation(state))
                        assertFalse(TranscriptReadPolicy.navigation(state.copy(scanInProgress = true)))
                        assertFalse(TranscriptReadPolicy.navigation(state.copy(projection = projection.copy(journalSequence = "999"))))
                        if (current != null) assertEquals(current, TranscriptReadPolicy.previous(state))
                        for (field in listOf("turn_id", "journal_revision", "session_id", "digest")) {
                            val changed = value.deepCopy().apply { addProperty(field, "foreign") }
                            assertTrue(runCatching { TranscriptPageCodec.decode(changed, readProjection, turn.id, current,
                                journalRevision = first.journalRevision) }.isFailure)
                        }
                        joined.append(next.text)
                        if (!next.hasMore) {
                            assertEquals(next.totalBytes, joined.toString().toByteArray().size)
                            val digest = java.security.MessageDigest.getInstance("SHA-256").digest(joined.toString().toByteArray())
                                .joinToString("") { "%02x".format(it.toInt() and 255) }
                            assertEquals(next.digest, digest)
                            assertTrue(joined.endsWith("end of historical detail"))
                        }
                        assertFalse(joined.contains("PRIVATE_HISTORY_CREDENTIAL"))
                    }
                }
            }
            return@runBlocking
        }
        var previous: SessionHistoryPage? = null
        var history = emptyList<SessionHistoryPage>()
        for (raw in seed.getAsJsonArray("pages")) {
            val page = SessionHistoryCodec.decode(raw.asJsonObject, projection, previous)
            history = SessionHistoryCodec.remember(history, previous, page)
            previous = page
        }
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection,
            sessionHistoryOpen = true, sessionHistory = previous, sessionHistoryPrevious = history)
        assertTrue(SessionHistoryReadPolicy.browse(state))
        if (previous != null) assertTrue(SessionHistoryReadPolicy.navigation(state))
        val request = SessionHistoryCodec.request(projection, previous)
        if (phase == "generate") {
            file("command").outputStream().use { FrameCodec.write(it, request) }
        } else {
            assertEquals("validate", phase)
            val results = JsonParser.parseString(file("results").readText()).asJsonObject
            for (raw in results.getAsJsonArray("exchanges")) {
                val exchange = raw.asJsonObject
                val bytes = Base64.getDecoder().decode(exchange["wire_base64"].asString)
                val input = object : ByteArrayInputStream(bytes) {
                    override fun read(target: ByteArray, offset: Int, length: Int): Int = super.read(target, offset, minOf(length, 3))
                }
                val response = runCatching { ResponseReader.read(input, exchange.getAsJsonObject("request"), {
                    if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
                }) }
                val code = exchange["code"].takeUnless { it.isJsonNull }?.asString
                if (code != null) {
                    assertTrue(response.exceptionOrNull().toString(), response.exceptionOrNull() is HostRefusedException)
                    assertEquals(code, (response.exceptionOrNull() as HostRefusedException).code)
                } else {
                    assertTrue(response.exceptionOrNull().toString(), response.isSuccess)
                    val result = response.getOrThrow().result
                    val page = SessionHistoryCodec.decode(result, projection, previous)
                    assertEquals(70, page.totalTurns)
                    assertEquals(seed["offset"].asInt, page.offset)
                    assertEquals(seed["count"].asInt, page.turns.size)
                    val retained = SessionHistoryCodec.remember(history, previous, page)
                    assertTrue(SessionHistoryReadPolicy.navigation(state.copy(sessionHistory = page, sessionHistoryPrevious = retained)))
                    assertFalse(page.turns.any { it.content.contains("PRIVATE_HISTORY_CREDENTIAL") })
                    for (field in listOf("project_id", "session_id", "journal_revision", "digest")) {
                        val altered = result.deepCopy().apply { addProperty(field, "foreign") }
                        assertTrue(runCatching { SessionHistoryCodec.decode(altered, projection, previous) }.isFailure)
                    }
                    if (page.offset == 0) {
                        assertEquals(firstId, page.turns.first().id)
                        val chosen = state.copy(sessionHistory = page, transcriptTurn = firstId, transcriptHistoryTurn = page.turns.first())
                        val readProjection = requireNotNull(SessionHistoryReadPolicy.projection(chosen, projection))
                        assertTrue(readProjection.turns.any { it.id == firstId })
                        assertFalse(projection.turns.any { it.id == firstId })
                    }
                }
            }
        }
    }
}
