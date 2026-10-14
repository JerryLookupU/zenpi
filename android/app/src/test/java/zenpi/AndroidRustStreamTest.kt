package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Assume.assumeNotNull
import org.junit.Test
import zenpi.transport.*

/** Invoked by the Rust interop lane with actual subscription frames carrying live text deltas. */
class AndroidRustStreamTest {
    private fun wire(exchange: JsonObject) = java.util.Base64.getDecoder().decode(exchange["wire_base64"].asString)
    private suspend fun pages(exchange: JsonObject, accept: suspend (ResponsePage) -> Unit = {}): ResponsePage =
        ResponseReader.read(object : java.io.ByteArrayInputStream(wire(exchange)) {
            override fun read(bytes: ByteArray, offset: Int, length: Int): Int = super.read(bytes, offset, minOf(length, 3))
        }, exchange.getAsJsonObject("request"), { assertEquals("succeeded", FrameCodec.string(it, "execution_state")) }, accept)

    @Test fun actualRustDeltasRebuildLiveTextInOrderAndReplayWithoutDuplicatesOrGaps() = runBlocking {
        val path = System.getProperty("zenpi.stream.fixture")
        assumeNotNull(path)
        val fixture = JsonParser.parseString(java.io.File(path).readText()).asJsonObject
        val listing = pages(fixture.getAsJsonObject("listing")).result
        val projects = HostProjectionCodec.projects(listing)
        val selected = projects.single()
        val initial = HostProjectionCodec.snapshot(projects, selected, pages(fixture.getAsJsonObject("initial")).result)
        val target = fixture["request_id"].asString
        fun text(projection: HostProjection) = projection.projects.single().promptStatuses.singleOrNull { it.requestId == target }?.streamText

        val streaming = SubscriptionReducer(initial, selected)
        val seen = mutableListOf<String?>()
        pages(fixture.getAsJsonObject("stream")) { seen += text(streaming.accept(it.frames, it.result)) }
        assertTrue(streaming.done)
        val published = seen.filterNotNull().distinct()
        assertEquals(fixture["published"].asJsonArray.map { it.asString }, published)
        // Ordered: every page extends the previous text, until the final text rewrites its tail.
        for ((before, after) in published.dropLast(1).zipWithNext()) assertTrue("$before -> $after", after.startsWith(before))
        val final = fixture["final"].asString
        assertEquals(final, text(streaming.projection))
        assertTrue(seen.none { it?.contains("SECRET") == true })

        // Reconnect: a new subscription replays from offset 0 onto the held text.
        val replay = SubscriptionReducer(streaming.projection, selected)
        var replayed = 0
        pages(fixture.getAsJsonObject("replay")) { replay.accept(it.frames, it.result); replayed++ }
        assertTrue(replay.done); assertTrue(replayed >= 1)
        assertEquals(final, text(replay.projection))

        // A page the client cannot place is refused instead of guessed at.
        val tampered = mutableListOf<ResponsePage>()
        pages(fixture.getAsJsonObject("replay")) { tampered += it }
        val first = tampered.first()
        val status = first.result.getAsJsonObject("subscription_view").getAsJsonObject("listing")
            .getAsJsonArray("projects")[0].asJsonObject.getAsJsonArray("prompt_statuses")[0].asJsonObject
        for ((field, value) in listOf("stream_offset" to final.codePointCount(0, final.length) + 1, "stream_text" to final)) {
            val page = first.result.deepCopy()
            val item = page.getAsJsonObject("subscription_view").getAsJsonObject("listing")
                .getAsJsonArray("projects")[0].asJsonObject.getAsJsonArray("prompt_statuses")[0].asJsonObject
            if (value is Int) item.addProperty(field, value) else item.addProperty(field, value as String)
            val reducer = SubscriptionReducer(streaming.projection, selected)
            assertTrue(runCatching { reducer.accept(first.frames, page) }.exceptionOrNull() is WireException)
            assertEquals(streaming.projection, reducer.projection)
        }
        assertEquals(0, status["stream_offset"].asInt)
    }
}
