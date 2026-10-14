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

class AndroidRustResourceTest {
    private fun projection(seed: JsonObject): HostProjection {
        val listing = seed.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing)
        val selected = requireNotNull(HostProjectionCodec.select(listing, projects, null))
        return HostProjectionCodec.workspace(
            HostProjectionCodec.snapshot(projects, selected, seed.getAsJsonObject("snapshot")), listing)
    }
    @Test fun catalogPagingReloadAndRestartReceiptsRoundtrip() = runBlocking {
        val phase = System.getProperty("zenpi.resource.phase")
        assumeNotNull(phase)
        fun file(name: String) = File(requireNotNull(System.getProperty("zenpi.resource.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val before = projection(seed)
        val catalog = ResourceCatalogCodec.decode(seed.getAsJsonObject("catalog"), before)
        assertFalse(ResourceCatalogCodec.sameScope(catalog.copy(generation = "01"), before))
        assertFalse(ResourceCatalogCodec.sameScope(catalog.copy(nextOffset = catalog.offset + 64), before))
        val invocable = catalog.entries.first { it.invocable }
        assertEquals("${invocable.command} literal $1", ResourceCatalogCodec.insert(invocable, "literal $1"))
        val altered = invocable.copy(command = "!touch marker")
        assertTrue(runCatching { ResourceCatalogCodec.insert(altered, "literal $1") }.isFailure)
        assertFalse(ResourceCatalogCodec.sameScope(catalog.copy(entries = listOf(altered)), before))
        val duplicate = before.copy(projects = before.projects + before.projects.single().copy(sessionId = "foreign"))
        assertFalse(ResourceReadPolicy.browse(PairingState(phase = PairingPhase.CONNECTED, projection = duplicate)))
        assertTrue(runCatching { ResourceCatalogCodec.request(duplicate) }.isFailure)
        assertTrue(runCatching { ResourceCatalogCodec.decode(seed.getAsJsonObject("catalog"), duplicate) }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.reloadResources(duplicate, catalog) }.isFailure)
        val step = seed["step"].asString
        if (phase == "generate") {
            val request = if (step == "page") ResourceCatalogCodec.request(before, catalog, requireNotNull(catalog.nextOffset))
            else WorkspaceCommand.reloadResources(before, catalog).also {
                assertTrue(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.CONNECTED,
                    projection = before, resources = catalog), true, true, it))
                assertFalse(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.CONNECTED,
                    projection = before, resources = catalog.copy(generation = "01")), true, true, it))
            }.envelope
            file("command").outputStream().use { FrameCodec.write(it, request) }
        } else {
            assertEquals("validate", phase)
            val results = JsonParser.parseString(file("results").readText()).asJsonObject
            for (raw in results.getAsJsonArray("exchanges")) {
                val exchange = raw.asJsonObject
                val request = exchange.getAsJsonObject("request")
                val bytes = Base64.getDecoder().decode(exchange["wire_base64"].asString)
                val input = object : ByteArrayInputStream(bytes) {
                    override fun read(target: ByteArray, offset: Int, length: Int): Int = super.read(target, offset, minOf(length, 3))
                }
                val checked = runCatching {
                    ResponseReader.read(input, request, {
                        if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
                        if (step != "page") WorkspaceReceiptCodec.validate(WorkspaceCommand.restore(request), it)
                    })
                }
                val code = exchange["code"].takeUnless { it.isJsonNull }?.asString
                if (code != null) {
                    assertTrue(checked.exceptionOrNull().toString(), checked.exceptionOrNull() is HostRefusedException)
                    assertEquals(code, (checked.exceptionOrNull() as HostRefusedException).code)
                } else {
                    assertTrue(checked.exceptionOrNull().toString(), checked.isSuccess)
                    if (step == "page") {
                        val page = ResourceCatalogCodec.decode(checked.getOrThrow().result, before, catalog, requireNotNull(catalog.nextOffset))
                        assertEquals(8, page.entries.size); assertNull(page.nextOffset)
                        assertEquals(listOf(catalog), ResourceCatalogCodec.remember(emptyList(), catalog, page))
                        assertTrue(page.entries.all { it.kind == "template" && it.invocable })
                        val readOnly = PairingState(phase = PairingPhase.CONNECTED,
                            projection = before.copy(projects = before.projects.map { it.copy(promptControl = false, resourcesReloadControl = false) }),
                            resources = page)
                        assertTrue(ResourceReadPolicy.browse(readOnly)); assertTrue(ResourceReadPolicy.navigation(readOnly))
                        assertFalse(ResourceReadPolicy.insertion(readOnly))
                        val changed = checked.getOrThrow().result.deepCopy().apply { addProperty("snapshot_id", "0".repeat(64)) }
                        assertTrue(runCatching { ResourceCatalogCodec.decode(changed, before, catalog, 32) }.isFailure)
                    } else {
                        val response = requireNotNull(FrameCodec.read(ByteArrayInputStream(bytes)))
                        for (field in listOf("request_id", "project_id", "session_id", "generation")) {
                            val altered = response.deepCopy()
                            val value = altered.getAsJsonObject("payload").getAsJsonObject("result")
                            if (field == "generation") value.addProperty(field, 0) else value.addProperty(field, "foreign")
                            assertTrue(runCatching { WorkspaceReceiptCodec.validate(WorkspaceCommand.restore(request), altered) }.isFailure)
                        }
                    }
                }
            }
            val after = results.getAsJsonObject("after")
            val updated = ResourceCatalogCodec.decode(after.getAsJsonObject("catalog"), projection(after))
            assertEquals(results["expected_generation"].toString(), updated.generation)
            assertEquals(results["expected_description"].asString, updated.entries.single { it.name == "resource-00" }.description)
            assertEquals(before.workspaceRevision, projection(after).workspaceRevision)
            assertEquals(before.sessionId, updated.sessionId)
            assertTrue(updated.entries.none { it.description.contains("PRIVATE_TEMPLATE_BODY") })
            if (step != "page") {
                val command = WorkspaceCommand.restore(results.getAsJsonObject("request"))
                assertTrue(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.OFFLINE, pendingWorkspace = command),
                    true, true, WorkspaceCommand.restore(command.envelope)))
            }
        }
    }
}
