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

class AndroidRustWorkerReadTest {
    @Test fun taskPagesAndHistoryReadActualDesktopWorkerWithoutPublishingWriteAuthority() = runBlocking {
        val phase = System.getProperty("zenpi.workerread.phase")
        assumeNotNull(phase)
        fun file(name: String) = File(requireNotNull(System.getProperty("zenpi.workerread.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val listing = seed.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing)
        val selected = requireNotNull(HostProjectionCodec.select(listing, projects, null))
        val projection = HostProjectionCodec.workspace(HostProjectionCodec.snapshot(projects, selected,
            seed.getAsJsonObject("snapshot")), listing)
        assertFalse(projects.single().workerControl)
        val mode = seed["mode"].asString
        val target = seed["target"]?.asString
        var previousList: WorkerStatusPage? = null
        var listHistory = emptyList<WorkerStatusPage>()
        for (raw in seed.getAsJsonArray("lists")) {
            val page = WorkerStatusPageCodec.decode(raw.asJsonObject, projection, previousList, previousList?.nextOffset ?: 0)
            listHistory = WorkerStatusPageCodec.remember(listHistory, previousList, page); previousList = page
        }
        var state = PairingState(phase = PairingPhase.CONNECTED, projection = projection,
            workerStatusesOpen = true, workerStatusesPage = previousList, workerStatusesPrevious = listHistory)
        val cache = zenpi.state.ProjectionCacheCodec.encode(projection)
        val read = if (target != null) {
            val chosen = (listHistory + listOfNotNull(previousList)).single { page -> page.tasks.any { it.requestId == target } }
            state = state.copy(workerStatusesPage = chosen,
                workerStatusesPrevious = listHistory.takeWhile { it.offset < chosen.offset })
            assertFalse(projects.single().workerStatuses.any { it.requestId == target })
            requireNotNull(WorkerReadPolicy.projection(state, target))
        } else projection
        if (target != null) {
            val actual = read.projects.single { it.id == read.selectedId }.workerStatuses.single { it.requestId == target }
            for (bad in listOf(actual.copy(subtabId = "bad"), actual.copy(runtimeActive = true),
                actual.copy(failureCode = "internal_error"), actual.copy(outputTruncated = true),
                actual.copy(streamText = "invalid completed stream"))) {
                val malformed = read.copy(projects = read.projects.map { project ->
                    project.copy(workerStatuses = project.workerStatuses.map { if (it.requestId == target) bad else it })
                })
                assertFalse(TranscriptPageCodec.retainsTurn(malformed, malformed, target, true))
                assertTrue(runCatching { TranscriptPageCodec.request(malformed, target, null, true) }.isFailure)
                assertTrue(runCatching { WorkerHistoryCodec.request(malformed, target, null) }.isFailure)
                assertTrue(runCatching { WorkerReadPolicy.scope(malformed, target) }.isFailure)
            }
        }
        var historyPage: WorkerHistoryPage? = null
        for (raw in seed.getAsJsonArray("histories")) historyPage = WorkerHistoryCodec.decode(raw.asJsonObject, read, requireNotNull(target), historyPage)
        val turn = if (mode == "detail") requireNotNull(historyPage).turns.first { it.role == "user" } else null
        var previousText: TranscriptPage? = null
        var textHistory = emptyList<TranscriptPage>()
        val joined = StringBuilder()
        for (raw in seed.getAsJsonArray("texts")) {
            val page = TranscriptPageCodec.decode(raw.asJsonObject, read, requireNotNull(target), previousText, true, turn)
            textHistory = TranscriptNavigation.remember(textHistory, previousText, page); previousText = page; joined.append(page.text)
        }
        val request = when (mode) {
            "list" -> WorkerStatusPageCodec.request(projection, previousList, previousList?.nextOffset ?: 0)
            "history" -> WorkerHistoryCodec.request(read, requireNotNull(target), historyPage)
            "output", "detail" -> TranscriptPageCodec.request(read, requireNotNull(target), previousText, true, turn)
            else -> error("Unknown worker read mode")
        }
        assertFalse(request.has("required_owner")); assertFalse(request.has("expected_revision"))
        assertTrue(WorkerStatusReadPolicy.browse(state))
        assertFalse(WorkerStatusReadPolicy.browse(state.copy(scanInProgress = true)))
        if (target != null) {
            assertNull(WorkerReadPolicy.projection(state.copy(scanInProgress = true), target))
            assertEquals(cache, zenpi.state.ProjectionCacheCodec.encode(read))
        }
        if (phase == "generate") {
            file("command").outputStream().use { FrameCodec.write(it, request) }
            return@runBlocking
        }
        assertEquals("validate", phase)
        val results = JsonParser.parseString(file("results").readText()).asJsonObject
        for (raw in results.getAsJsonArray("exchanges")) {
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
                assertTrue(response.exceptionOrNull().toString(), response.exceptionOrNull() is HostRefusedException)
                assertEquals(code, (response.exceptionOrNull() as HostRefusedException).code)
                continue
            }
            val value = response.getOrThrow().result
            when (mode) {
                "list" -> {
                    val page = WorkerStatusPageCodec.decode(value, projection, previousList, previousList?.nextOffset ?: 0)
                    val contradictory = page.copy(tasks = page.tasks.map { it.copy(runtimeActive = true) })
                    assertFalse(WorkerStatusPageCodec.sameScope(contradictory, projection))
                    val alteredRuntime = value.deepCopy().apply { getAsJsonArray("tasks").forEach {
                        it.asJsonObject.addProperty("runtime_active", true)
                    } }
                    assertTrue(runCatching { WorkerStatusPageCodec.decode(alteredRuntime, projection, previousList,
                        previousList?.nextOffset ?: 0) }.isFailure)
                    assertEquals(18, page.totalTasks)
                    assertEquals(if (previousList == null) 16 else 2, page.tasks.size)
                    assertTrue(page.tasks.all { it.state == "succeeded" && it.outputText == null && it.streamText == null })
                    assertTrue(WorkerStatusReadPolicy.navigation(state.copy(workerStatusesPage = page)))
                    for (field in listOf("project_id", "session_id", "digest")) assertTrue(runCatching {
                        WorkerStatusPageCodec.decode(value.deepCopy().apply { addProperty(field, "foreign") }, projection, previousList, previousList?.nextOffset ?: 0)
                    }.isFailure)
                }
                "history" -> {
                    val page = WorkerHistoryCodec.decode(value, read, requireNotNull(target), historyPage)
                    assertEquals(listOf("user", "assistant"), page.turns.map { it.role })
                    assertTrue(page.turns.all { it.contentTruncated })
                    val opened = state.copy(workerStatusesOpen = false, workerStatusesPage = null, workerHistoryRequest = target,
                        workerHistoryPage = page, workerReadScope = WorkerReadPolicy.scope(read, target))
                    assertTrue(WorkerHistoryNavigation.navigation(opened))
                    assertFalse(WorkerHistoryNavigation.navigation(opened.copy(scanInProgress = true)))
                }
                else -> {
                    val page = TranscriptPageCodec.decode(value, read, requireNotNull(target), previousText, true, turn)
                    if (previousText == null) {
                        // Mutate a real host result to check opaque identity admission, not host dispatch.
                        for (opaque in listOf("worker:turn/\u4e2d", "\u754c".repeat(42) + "ab")) {
                            val altered = value.deepCopy().apply { addProperty("turn_id", opaque) }
                            val selected = turn?.copy(id = opaque)
                            val decoded = TranscriptPageCodec.decode(altered, read, target, null, true, selected)
                            assertEquals(opaque, decoded.turnId); assertEquals(page.text, decoded.text)
                            assertEquals(page.workerRequestId, decoded.workerRequestId); assertEquals(page.subtabId, decoded.subtabId)
                            if (selected != null) {
                                val exact = TranscriptPageCodec.request(read, target, null, true, selected)
                                assertEquals(opaque, exact.getAsJsonObject("payload").getAsJsonObject("arguments")["turn_id"].asString)
                            }
                        }
                        for (invalid in listOf("", " ", "bad\nturn", "\u754c".repeat(43))) {
                            assertTrue(runCatching { TranscriptPageCodec.decode(value.deepCopy().apply { addProperty("turn_id", invalid) },
                                read, target, null, true, turn?.copy(id = invalid)) }.isFailure)
                        }
                    }
                    val opened = state.copy(workerStatusesOpen = false, workerStatusesPage = null, transcriptTurn = target,
                        transcriptWorker = true, transcriptHistoryTurn = turn, transcriptPage = page,
                        transcriptPreviousPages = TranscriptNavigation.remember(textHistory, previousText, page),
                        workerReadScope = WorkerReadPolicy.scope(read, target))
                    assertTrue(TranscriptReadPolicy.navigation(opened))
                    assertFalse(TranscriptReadPolicy.navigation(opened.copy(scanInProgress = true)))
                    if (previousText != null) assertEquals(previousText, TranscriptReadPolicy.previous(opened))
                    joined.append(page.text)
                    if (!page.hasMore) {
                        assertEquals(page.totalBytes, joined.toString().toByteArray().size)
                        val digest = java.security.MessageDigest.getInstance("SHA-256").digest(joined.toString().toByteArray())
                            .joinToString("") { "%02x".format(it.toInt() and 255) }
                        assertEquals(page.digest, digest)
                        assertTrue(joined.contains("worker payload"))
                    }
                    val boundFields = listOf("request_id", "subtab_id", "digest") +
                        if (previousText != null || turn != null) listOf("turn_id") else emptyList()
                    for (field in boundFields) assertTrue(field, runCatching {
                        TranscriptPageCodec.decode(value.deepCopy().apply { addProperty(field, "foreign") }, read, target, previousText, true, turn)
                    }.isFailure)
                }
            }
        }
        assertEquals(cache, zenpi.state.ProjectionCacheCodec.encode(projection))
    }
}
