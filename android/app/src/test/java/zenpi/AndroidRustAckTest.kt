package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Assume.assumeNotNull
import org.junit.Test
import zenpi.state.ProjectionCacheCodec
import zenpi.transport.*
import java.io.ByteArrayInputStream
import java.io.File
import java.util.Base64

class AndroidRustAckTest {
    @Test fun restoredCursorAcknowledgesAndResumesAcrossActualRustRestart() = runBlocking {
        val phase = System.getProperty("zenpi.ack.phase")
        assumeNotNull(phase)
        fun file(name: String) = File(requireNotNull(System.getProperty("zenpi.ack.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val projects = HostProjectionCodec.projects(seed.getAsJsonObject("listing"))
        val selected = projects.single()
        val initial = HostProjectionCodec.snapshot(projects, selected, seed.getAsJsonObject("snapshot"))
        val cached = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(initial))
        assertEquals("1", cached.eventSequence)
        assertEquals(initial.turns, cached.turns)
        fun ack(sequence: String, id: String) = EventAckCodec.request(selected.id, selected.sessionId, sequence)
            .apply { addProperty("id", id) }
        if (phase == "generate") {
            val resume = ack(requireNotNull(cached.eventSequence), "android-resume").apply {
                getAsJsonObject("payload").apply { addProperty("type", "resume"); addProperty("limit", 128) }
            }
            file("commands").outputStream().use { output ->
                FrameCodec.write(output, ack(requireNotNull(cached.eventSequence), "android-ack"))
                FrameCodec.write(output, ack("0", "android-lower-ack"))
                FrameCodec.write(output, ack("4", "android-future-ack"))
                FrameCodec.write(output, resume)
            }
        } else {
            val results = JsonParser.parseString(file("results").readText()).asJsonObject
            assertTrue(phase == "validate" || phase == "final")
            var projection = cached
            for (entry in results.getAsJsonArray("exchanges")) {
                val exchange = entry.asJsonObject
                val request = exchange.getAsJsonObject("request")
                val type = FrameCodec.string(request.getAsJsonObject("payload"), "type")
                val sequence = request.getAsJsonObject("payload").getAsJsonObject("cursor")["next_sequence"].toString()
                val input = object : ByteArrayInputStream(Base64.getDecoder().decode(exchange["wire_base64"].asString)) {
                    override fun read(target: ByteArray, offset: Int, length: Int) = super.read(target, offset, minOf(length, 3))
                }
                val failure = exchange["code"].takeUnless { it.isJsonNull }?.asString
                var confirmed = false
                val checked = runCatching {
                    suspend fun response(sent: JsonObject) = ResponseReader.read(input, sent, {
                        if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
                        assertEquals("succeeded", FrameCodec.string(it, "execution_state"))
                    })
                    if (type == "ack") {
                        var admitted = 0
                        val wrong = request.deepCopy().apply { addProperty("project_id", "foreign") }
                        assertTrue(runCatching {
                            EventAckCodec.acknowledge(wrong, seed["device_id"].asString, selected.id,
                                selected.sessionId, sequence) { admitted++; response(it).result }
                        }.exceptionOrNull() is WireException)
                        assertEquals(0, admitted)
                        confirmed = EventAckCodec.acknowledge(request, seed["device_id"].asString, selected.id,
                            selected.sessionId, sequence) { response(it).result }
                    } else {
                        val page = response(request)
                        projection = HostProjectionCodec.replay(projection, projects, selected, page.frames, page.result)
                    }
                }
                if (failure == "busy") { assertTrue(checked.isSuccess); assertFalse(confirmed) }
                else if (failure != null) {
                    assertEquals(failure, (checked.exceptionOrNull() as HostRefusedException).code)
                    assertFalse(confirmed)
                } else {
                    assertNull(checked.exceptionOrNull())
                    if (type == "ack") assertTrue(confirmed)
                    assertEquals(0, input.available())
                }
            }
            assertEquals(cached, ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(cached)))
            if (phase == "validate") {
                assertEquals("2", projection.eventSequence)
                assertEquals(listOf("desktop seed", "desktop after restart"), projection.turns.map { it.content })
                assertEquals(1, projection.replayedEventCount)
                assertFalse(projection.hasMoreEvents)
                val restored = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(projection))
                assertEquals(projection.turns, restored.turns); assertEquals("2", restored.eventSequence)
                file("next").outputStream().use { FrameCodec.write(it, ack(requireNotNull(restored.eventSequence), "android-next-ack")) }
            }
        }
    }
}
