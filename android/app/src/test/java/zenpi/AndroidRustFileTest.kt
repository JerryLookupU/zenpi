package zenpi

import com.google.gson.JsonObject
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

class AndroidRustFileTest {
    @Test fun readOnlyFilesRoundtripThroughDesktopOwner() = runBlocking {
        val phase = System.getProperty("zenpi.files.phase")
        assumeNotNull(phase)
        fun file(name: String) = File(requireNotNull(System.getProperty("zenpi.files.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val listing = seed.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing)
        val selected = requireNotNull(HostProjectionCodec.select(listing, projects, null))
        val projection = HostProjectionCodec.workspace(
            HostProjectionCodec.snapshot(projects, selected, seed.getAsJsonObject("snapshot")), listing)
            .copy(subtabs = SubtabProjectionCodec.decode(seed.getAsJsonObject("subtabs"), selected, listing))
        assertFalse(projects.single().promptControl)
        val first = seed["first"]?.takeUnless { it.isJsonNull }?.asJsonObject?.let {
            WorkspaceFilesCodec.decode(it, projection, "docs")
        }
        val previous = seed["previous"]?.takeUnless { it.isJsonNull }?.asJsonObject?.let {
            val previousOffset = seed["previous_offset"].asInt
            if (previousOffset == 0) first else WorkspaceFilesCodec.decode(it, projection, "docs", first, previousOffset)
        }
        val path = seed["path"].asString
        val offset = seed["offset"].asInt
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, files = previous)
        assertTrue(FileReadPolicy.browse(state))
        assertFalse(FileReadPolicy.attachment(state))
        if (previous != null) assertTrue(FileReadPolicy.navigation(state))
        val duplicate = projection.copy(projects = projection.projects + projection.projects.single().copy(sessionId = "foreign"))
        assertTrue(runCatching { WorkspaceFilesCodec.request(duplicate, path) }.isFailure)
        val missingTab = projection.copy(subtabs = requireNotNull(projection.subtabs).copy(activeId = "f".repeat(64)))
        assertTrue(runCatching { WorkspaceFilesCodec.request(missingTab, path) }.isFailure)
        val tabs = requireNotNull(projection.subtabs)
        val inactive = RemoteSubtab("e".repeat(64), "Other", "in_place", 1)
        val invalidTabs = projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + listOf(inactive, inactive)))
        assertFalse(FileReadPolicy.browse(state.copy(projection = invalidTabs)))
        assertTrue(runCatching { WorkspaceFilesCodec.request(invalidTabs, path, previous, offset) }.isFailure)
        val request = WorkspaceFilesCodec.request(projection, path, previous, offset)
        if (phase == "generate") {
            file("command").outputStream().use { FrameCodec.write(it, request) }
        } else {
            assertEquals("validate", phase)
            val results = JsonParser.parseString(file("results").readText()).asJsonObject
            for (raw in results.getAsJsonArray("exchanges")) {
                val exchange = raw.asJsonObject
                val actualRequest = exchange.getAsJsonObject("request")
                val bytes = Base64.getDecoder().decode(exchange["wire_base64"].asString)
                val input = object : ByteArrayInputStream(bytes) {
                    override fun read(target: ByteArray, offset: Int, length: Int): Int = super.read(target, offset, minOf(length, 3))
                }
                val response = runCatching { ResponseReader.read(input, actualRequest, {
                    if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
                }) }
                val code = exchange["code"].takeUnless { it.isJsonNull }?.asString
                if (code != null) {
                    assertTrue(response.exceptionOrNull().toString(), response.exceptionOrNull() is HostRefusedException)
                    assertEquals(code, (response.exceptionOrNull() as HostRefusedException).code)
                } else {
                    assertTrue(response.exceptionOrNull().toString(), response.isSuccess)
                    val metadata = response.getOrThrow().result
                    val page = WorkspaceFilesCodec.decode(metadata, projection, path, previous, offset)
                    assertFalse(WorkspaceFilesCodec.matchesSelection(page.copy(page = page.page?.copy(snapshotId = "invalid")), projection))
                    assertFalse(WorkspaceFilesCodec.matchesSelection(page, invalidTabs))
                    assertTrue(runCatching { WorkspaceFilesCodec.decode(metadata, invalidTabs, path, previous, offset) }.isFailure)
                    assertEquals(seed["expected_count"].asInt, page.entries.size)
                    assertEquals(offset, requireNotNull(page.page).offset)
                    assertTrue(FileReadPolicy.navigation(state.copy(files = page)))
                    assertFalse(FileReadPolicy.attachment(state.copy(files = page)))
                    assertTrue(page.entries.none { WorkspaceFilesCodec.privatePath(it.path) || it.name.startsWith('.') })
                    for (field in listOf("project_id", "session_id", "subtab_id", "subtab_revision", "path")) {
                        val altered = metadata.deepCopy().apply { addProperty(field, if (field == "subtab_revision") "999" else "foreign") }
                        assertTrue(runCatching { WorkspaceFilesCodec.decode(altered, projection, path, previous, offset) }.isFailure)
                    }
                    if (previous != null && offset > previous.page!!.offset) {
                        assertEquals(listOf(previous), WorkspaceFilesCodec.remember(emptyList(), previous, page))
                        val altered = metadata.deepCopy().apply { getAsJsonObject("page").addProperty("snapshot_id", "0".repeat(64)) }
                        assertTrue(runCatching { WorkspaceFilesCodec.decode(altered, projection, path, previous, offset) }.isFailure)
                    }
                    if (path == "docs/nested") assertEquals("child.txt", page.entries.single().name)
                    assertFalse(metadata.toString().contains("PRIVATE_FILE_BODY"))
                }
            }
        }
    }
}
