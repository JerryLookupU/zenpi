package zenpi

import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Assume.assumeNotNull
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*
import java.io.File
import java.io.ByteArrayInputStream
import java.util.Base64

class AndroidRustWorkerToolTest {
    @Test fun realToolLoopHistoryAndArgumentsRetainWorkerIdentityAndRedaction() = runBlocking {
        val phase = System.getProperty("zenpi.workertool.phase"); assumeNotNull(phase)
        fun file(name: String) = File(requireNotNull(System.getProperty("zenpi.workertool.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val listing = seed.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing)
        val selected = requireNotNull(HostProjectionCodec.select(listing, projects, null))
        val projection = HostProjectionCodec.workspace(HostProjectionCodec.snapshot(projects, selected, seed.getAsJsonObject("snapshot")), listing)
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection)
        val read = requireNotNull(WorkerReadPolicy.projection(state, "tool-loop"))
        var current: WorkerHistoryPage? = null
        var retained = emptyList<WorkerHistoryPage>()
        for (raw in seed.getAsJsonArray("histories")) {
            val next = WorkerHistoryCodec.decode(raw.asJsonObject, read, "tool-loop", current)
            retained = WorkerHistoryNavigation.remember(retained, current, next); current = next
        }
        val mode = seed["mode"].asString
        val turn = (retained + listOfNotNull(current)).flatMap { it.turns }.firstOrNull { it.toolCalls.isNotEmpty() }
        val call = turn?.toolCalls?.single()
        if (turn != null && call != null) {
            val toolPage = (retained + listOfNotNull(current)).single { page -> page.turns.any { it.id == turn.id } }
            val altered = toolPage.copy(turns = toolPage.turns.map { old ->
                if (old.id == turn.id) old.copy(toolCalls = listOf(call.copy(argumentsPreview = "x".repeat(513)))) else old })
            assertFalse(WorkerHistoryCodec.validPage(altered))
            assertFalse(WorkerHistoryNavigation.validChain(emptyList(), altered))
            val malformedTurn = altered.turns.single { it.id == turn.id }
            assertTrue(runCatching { TranscriptPageCodec.request(read, "tool-loop", null, true,
                malformedTurn, malformedTurn.toolCalls.single()) }.isFailure)
            if (altered.hasMore) assertTrue(runCatching { WorkerHistoryCodec.request(read, "tool-loop", altered) }.isFailure)
        }
        val request = if (mode == "history") WorkerHistoryCodec.request(read, "tool-loop", current)
            else TranscriptPageCodec.request(read, "tool-loop", null, true, requireNotNull(turn), requireNotNull(call))
        if (phase == "generate") {
            file("command").outputStream().use { FrameCodec.write(it, request) }; return@runBlocking
        }
        val result = JsonParser.parseString(file("results").readText()).asJsonObject
        for (raw in result.getAsJsonArray("exchanges")) {
            val exchange = raw.asJsonObject
            val bytes = Base64.getDecoder().decode(exchange["wire_base64"].asString)
            val input = object : ByteArrayInputStream(bytes) {
                override fun read(buffer: ByteArray, offset: Int, length: Int) = super.read(buffer, offset, minOf(length, 3))
            }
            val response = runCatching { ResponseReader.read(input, exchange.getAsJsonObject("request"), {
                if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
            }) }
            val code = exchange["code"].takeUnless { it.isJsonNull }?.asString
            if (code != null) {
                assertTrue(response.exceptionOrNull() is HostRefusedException)
                assertEquals(code, (response.exceptionOrNull() as HostRefusedException).code); continue
            }
            val value = response.getOrThrow().result
            assertFalse(value.toString().contains("PRIVATE_TOOL_LOOP_SECRET"))
            if (mode == "history") {
                val page = WorkerHistoryCodec.decode(value, read, "tool-loop", current)
                assertEquals(20, page.totalTurns)
                val history = WorkerHistoryNavigation.remember(retained, current, page)
                val opened = state.copy(workerHistoryRequest = "tool-loop", workerHistoryPage = page, workerHistoryPrevious = history)
                assertTrue(WorkerHistoryNavigation.navigation(opened))
                assertFalse(WorkerHistoryNavigation.navigation(opened.copy(scanInProgress = true)))
                if (!page.hasMore) {
                    val turns = (history + page).flatMap { it.turns }
                    assertEquals(9, turns.count { it.role == "tool" })
                    assertTrue(turns.filter { it.role == "tool" }.all { it.content.contains("Tool content") })
                    assertEquals(9, turns.sumOf { it.toolCalls.size })
                    assertEquals(20, turns.map { it.id }.distinct().size)
                    assertEquals("Final tool-backed answer", turns.last().content)
                    assertEquals(current, WorkerHistoryNavigation.previous(opened).workerHistoryPage)
                }
            } else {
                val page = TranscriptPageCodec.decode(value, read, "tool-loop", null, true, turn, call)
                assertEquals("read_file", page.toolName)
                assertEquals(seed["arguments"].asString, page.text)
                assertTrue(requireNotNull(call).argumentsTruncated)
                val opened = state.copy(transcriptTurn = "tool-loop", transcriptWorker = true,
                    transcriptHistoryTurn = turn, transcriptToolCall = call, transcriptPage = page)
                assertTrue(TranscriptReadPolicy.navigation(opened))
                assertFalse(TranscriptReadPolicy.navigation(opened.copy(scanInProgress = true)))
                for (field in listOf("request_id", "subtab_id", "turn_id", "tool_call_id", "tool_name")) assertTrue(runCatching {
                    TranscriptPageCodec.decode(value.deepCopy().apply { addProperty(field, "foreign") }, read, "tool-loop", null, true, turn, call)
                }.isFailure)
            }
        }
    }
}
