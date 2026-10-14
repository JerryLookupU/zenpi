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

class AndroidRustDiffTest {
    @Test fun readOnlyDiffPagesRoundtripThroughDesktopOwner() = runBlocking {
        val phase = System.getProperty("zenpi.diff.phase")
        assumeNotNull(phase)
        fun file(name: String) = File(requireNotNull(System.getProperty("zenpi.diff.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val listing = seed.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing)
        val selected = requireNotNull(HostProjectionCodec.select(listing, projects, null))
        val projection = HostProjectionCodec.workspace(
            HostProjectionCodec.snapshot(projects, selected, seed.getAsJsonObject("snapshot")), listing)
            .copy(subtabs = SubtabProjectionCodec.decode(seed.getAsJsonObject("subtabs"), selected, listing))
        assertFalse(projects.single().promptControl)
        assertTrue(projects.single().diffAvailable)
        val ambiguous = projection.copy(projects = projects + selected.copy(sessionId = "foreign"))
        assertFalse(DiffProjectionCodec.readable(ambiguous))
        assertTrue(runCatching { DiffProjectionCodec.request(ambiguous, "file.txt", paged = true) }.isFailure)
        val tabs = requireNotNull(projection.subtabs)
        val inactive = RemoteSubtab("e".repeat(64), "Other", "in_place", 1)
        val invalidTabs = projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + listOf(inactive, inactive)))
        assertFalse(DiffProjectionCodec.readable(invalidTabs))
        assertTrue(runCatching { DiffProjectionCodec.request(invalidTabs, "file.txt", paged = true) }.isFailure)
        var previous: RemoteDiff? = null
        var history = emptyList<RemoteDiff>()
        for (raw in seed.getAsJsonArray("pages")) {
            val page = DiffProjectionCodec.decode(raw.asJsonObject, projection, "file.txt", previous, paged = true)
            assertTrue(runCatching { DiffProjectionCodec.decode(raw.asJsonObject, ambiguous, "file.txt", previous, true) }.isFailure)
            assertTrue(runCatching { DiffProjectionCodec.decode(raw.asJsonObject, invalidTabs, "file.txt", previous, true) }.isFailure)
            history = DiffProjectionCodec.remember(history, previous, page)
            previous = page
        }
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, diff = previous, diffOpen = true, diffPrevious = history)
        assertTrue(DiffReadPolicy.browse(state))
        if (previous != null) assertTrue(DiffReadPolicy.navigation(state))
        val request = DiffProjectionCodec.request(projection, "file.txt", previous, paged = true)
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
                    val metadata = response.getOrThrow().result
                    val page = DiffProjectionCodec.decode(metadata, projection, "file.txt", previous, paged = true)
                    assertFalse(DiffProjectionCodec.sameSelection(page, invalidTabs))
                    assertTrue(runCatching { DiffProjectionCodec.decode(metadata, invalidTabs, "file.txt", previous, true) }.isFailure)
                    assertFalse(page.binary); assertTrue(page.changed); assertFalse(page.truncated)
                    assertTrue(page.diff.toByteArray().size <= 8192)
                    assertFalse(page.diff.contains('\ufffd'))
                    val retained = DiffProjectionCodec.remember(history, previous, page)
                    assertTrue(DiffReadPolicy.navigation(state.copy(diff = page, diffPrevious = retained)))
                    assertFalse(DiffReadPolicy.navigation(state.copy(diff = page, diffPrevious = retained, projection = ambiguous)))
                    assertEquals(previous, DiffProjectionCodec.previous(retained, page))
                    for (field in listOf("project_id", "session_id", "subtab_id", "path", "subtab_revision")) {
                        val altered = metadata.deepCopy().apply { addProperty(field, "foreign") }
                        assertTrue(runCatching { DiffProjectionCodec.decode(altered, projection, "file.txt", previous, true) }.isFailure)
                    }
                    if (previous != null) {
                        val altered = metadata.deepCopy().apply { getAsJsonObject("page").addProperty("digest", "0".repeat(64)) }
                        assertTrue(runCatching { DiffProjectionCodec.decode(altered, projection, "file.txt", previous, true) }.isFailure)
                    }
                }
            }
        }
    }
}
