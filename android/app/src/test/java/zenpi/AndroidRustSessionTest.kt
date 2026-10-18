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

/** Rust supplies current owner/catalog state and executes the actual Kotlin frames. */
class AndroidRustSessionTest {
    private fun projection(value: JsonObject): HostProjection {
        val listing = value.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing)
        val selected = projects.single()
        return HostProjectionCodec.workspace(HostProjectionCodec.snapshot(projects, selected, value.getAsJsonObject("snapshot")), listing)
            .copy(subtabs = SubtabProjectionCodec.decode(value.getAsJsonObject("subtabs"), selected, listing))
    }
    @Test fun replacementAndHistoryResumeBindActualOwnersAndDurableReceipts() = runBlocking {
        val phase = System.getProperty("zenpi.session.phase")
        assumeNotNull(phase)
        fun file(name: String) = File(requireNotNull(System.getProperty("zenpi.session.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val before = projection(seed)
        val catalog = SessionCatalogCodec.decode(seed.getAsJsonObject("catalog"), before, null, 0)
        assertTrue(catalog.entries.any { it.id == "history:session/\u4e2d" })
        val searchedWire = seed.getAsJsonObject("searched_catalog")
        val searched = SessionCatalogCodec.decode(searchedWire, before, null, 0, "desktop")
        assertEquals(catalog.snapshotId, searched.snapshotId)
        assertEquals(2, searched.entries.size)
        assertTrue(searched.entries.all { it.matches == 1 && it.snippets.size == 1
            && it.snippets.single().contains("desktop", ignoreCase = true) })
        assertEquals(setOf("Original desktop conversation", "Historical desktop answer"),
            searched.entries.flatMap { it.snippets }.toSet())
        for (fields in listOf(listOf("matches"), listOf("snippets"))) {
            val altered = searchedWire.deepCopy().apply {
                val entry = getAsJsonArray("entries")[0].asJsonObject
                fields.forEach { entry.remove(it) }
            }
            assertTrue(runCatching { SessionCatalogCodec.decode(altered, before, null, 0, "desktop") }.isFailure)
        }
        val legacy = searchedWire.deepCopy().apply { getAsJsonArray("entries").forEach {
            it.asJsonObject.apply { remove("matches"); remove("snippets") }
        } }
        assertTrue(SessionCatalogCodec.decode(legacy, before, null, 0, "desktop").entries.all {
            it.matches == 0 && it.snippets.isEmpty()
        })
        assertEquals(1, catalog.entries.count { it.current })
        val invalidCatalog = catalog.copy(snapshotId = "invalid")
        assertFalse(SessionCatalogCodec.sameScope(invalidCatalog, before))
        val currentEntry = catalog.entries.single { it.current }
        assertFalse(SessionCatalogCodec.sameScope(catalog.copy(entries = listOf(currentEntry.copy(current = false))), before))
        val ambiguous = before.copy(projects = before.projects + before.projects.single().copy(sessionId = "foreign"))
        assertFalse(SessionCatalogCodec.sameScope(catalog, ambiguous))
        assertTrue(runCatching { SessionCatalogCodec.request(ambiguous, null, 0) }.isFailure)
        assertTrue(runCatching { SessionCatalogCodec.decode(seed.getAsJsonObject("catalog"), ambiguous, null, 0) }.isFailure)
        assertTrue(runCatching { SessionCommandCodec.newSession(ambiguous) }.isFailure)
        assertTrue(runCatching { SessionCommandCodec.rename(ambiguous, "Renamed") }.isFailure)
        val tabs = requireNotNull(before.subtabs)
        val duplicated = before.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs.first()))
        assertFalse(SessionCommandCodec.mainSelected(duplicated))
        assertTrue(runCatching { SessionCommandCodec.newSession(duplicated) }.isFailure)
        assertTrue(runCatching { SessionCommandCodec.rename(duplicated, "Renamed") }.isFailure)
        val scopeCatalog = catalog.copy(entries = listOf(RemoteSession("scope-check", "1", 0, false)))
        assertTrue(SessionCatalogCodec.resumable(scopeCatalog, before, "scope-check"))
        assertTrue(runCatching { SessionCommandCodec.resume(duplicated, scopeCatalog, "scope-check") }.isFailure)
        val step = seed["step"].asString
        if (phase == "generate") {
            val command = if (step.startsWith("new")) SessionCommandCodec.newSession(before)
                else SessionCommandCodec.resume(before, catalog, seed["target"].asString)
            assertTrue(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.CONNECTED, projection = before, sessions = catalog), true, true, command))
            if (!step.startsWith("new")) assertFalse(WorkspaceDispatchPolicy.allows(
                PairingState(phase = PairingPhase.CONNECTED, projection = before, sessions = invalidCatalog), true, true, command))
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            file("command").outputStream().use { FrameCodec.write(it, command.envelope) }
        } else {
            assertEquals("validate", phase)
            val result = JsonParser.parseString(file("results").readText()).asJsonObject
            val command = WorkspaceCommand.restore(result.getAsJsonObject("request"))
            val exchanges = result.getAsJsonArray("exchanges")
            for (entry in exchanges) {
                val exchange = entry.asJsonObject
                val request = exchange.getAsJsonObject("request")
                val sent = WorkspaceCommand.restore(request)
                val bytes = Base64.getDecoder().decode(exchange["wire_base64"].asString)
                val input = object : ByteArrayInputStream(bytes) {
                    override fun read(target: ByteArray, offset: Int, length: Int): Int = super.read(target, offset, minOf(length, 5))
                }
                val checked = runCatching {
                    ResponseReader.read(input, request, {
                        if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
                        WorkspaceReceiptCodec.validate(sent, it)
                    })
                }
                val code = exchange["code"]?.takeUnless { it.isJsonNull }?.asString
                if (code == null) assertTrue("$step: ${checked.exceptionOrNull()}", checked.isSuccess)
                else {
                    assertTrue("$step: ${checked.exceptionOrNull()}", checked.exceptionOrNull() is HostRefusedException)
                    assertEquals(code, (checked.exceptionOrNull() as HostRefusedException).code)
                }
            }
            val after = projection(result.getAsJsonObject("after"))
            val currentCatalog = SessionCatalogCodec.decode(result.getAsJsonObject("after").getAsJsonObject("catalog"), after, null, 0)
            assertEquals(1, currentCatalog.entries.count { it.current })
            if (step == "stale_resume") {
                assertEquals(before, after)
                assertEquals("revision_conflict", exchanges.single().asJsonObject["code"].asString)
                assertNotEquals(catalog.snapshotId, currentCatalog.snapshotId)
                assertFalse(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.CONNECTED,
                    projection = after, sessions = currentCatalog), true, true, command))
                assertTrue(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.OFFLINE,
                    projection = after, sessions = currentCatalog, pendingWorkspace = command), true, true,
                    WorkspaceCommand.restore(command.envelope)))
            } else {
                assertNotEquals(before.sessionId, after.sessionId)
                assertEquals(before.selectedId, after.selectedId)
                assertNotEquals(before.workspaceOwner, after.workspaceOwner)
                assertEquals(before.workspaceOwner, result["pre_restart_owner"].asString)
                assertEquals(before.subtabs, after.subtabs)
                assertEquals(java.math.BigInteger(requireNotNull(before.workspaceRevision)) + java.math.BigInteger.ONE,
                    java.math.BigInteger(requireNotNull(after.workspaceRevision)))
                val successful = exchanges.filter { it.asJsonObject["code"].isJsonNull }
                assertEquals(3, successful.size)
                assertEquals(1, successful.map { it.asJsonObject["wire_base64"] }.distinct().size)
                assertFalse(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.CONNECTED, projection = after), true, true, command))
                assertFalse(SessionCatalogCodec.sameScope(catalog, after))
                assertTrue(runCatching { SessionCommandCodec.resume(after, catalog, requireNotNull(before.sessionId)) }.isFailure)
                if (step.startsWith("new")) assertTrue(after.turns.isEmpty())
                else {
                    assertEquals(seed["target"].asString, after.sessionId)
                    assertEquals(seed["expected_content"].asString, after.turns.single().content)
                }
            }
        }
    }
}
