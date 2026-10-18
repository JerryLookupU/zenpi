package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Assume.assumeNotNull
import org.junit.Test
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.pairing.WorkspaceDispatchPolicy
import zenpi.transport.*
import java.io.ByteArrayInputStream
import java.io.File
import java.util.Base64

/** Current Kotlin commands and reducers against a fixture-local Rust/Git owner. */
class AndroidRustWorktreeTest {
    private fun projection(seed: JsonObject): HostProjection {
        val listing = seed.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing)
        val selected = projects.single()
        return HostProjectionCodec.workspace(HostProjectionCodec.snapshot(projects, selected, seed.getAsJsonObject("snapshot")), listing)
            .copy(subtabs = SubtabProjectionCodec.decode(seed.getAsJsonObject("subtabs"), selected, listing))
    }

    @Test fun commandsReceiptsAndRefusalsPreserveHostOwnedWorktrees() = runBlocking {
        val phase = System.getProperty("zenpi.worktree.phase")
        assumeNotNull(phase)
        fun file(name: String) = File(requireNotNull(System.getProperty("zenpi.worktree.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val before = projection(seed)
        val tabs = requireNotNull(before.subtabs)
        val main = tabs.tabs.first().id
        val directory = tabs.tabs.singleOrNull { it.kind == "in_place" }?.id
        val worktree = tabs.tabs.singleOrNull { it.kind == "worktree" }?.id
        val step = seed["step"].asString
        if (phase == "generate") {
            val command = when (step) {
                "directory_add" -> WorkspaceCommand.directoryTab(before, "Source files", "src")
                "worktree_add" -> WorkspaceCommand.worktreeTab(before, "android-feature")
                "rename" -> WorkspaceCommand.subtab(before, "subtab_rename", requireNotNull(worktree), "Android feature")
                "move" -> WorkspaceCommand.subtab(before, "subtab_move", requireNotNull(worktree), requireNotNull(directory))
                "quota64" -> WorkspaceCommand.concurrency(before, requireNotNull(worktree), 64)
                "quota1" -> WorkspaceCommand.concurrency(before, requireNotNull(worktree), 1)
                "select_worktree" -> WorkspaceCommand.subtab(before, "subtab_select", requireNotNull(worktree))
                "dirty_close", "clean_close" -> WorkspaceCommand.subtab(before, "subtab_close", requireNotNull(worktree))
                "select_directory" -> WorkspaceCommand.subtab(before, "subtab_select", requireNotNull(directory))
                "close_directory" -> WorkspaceCommand.subtab(before, "subtab_close", requireNotNull(directory))
                "select_main" -> WorkspaceCommand.subtab(before, "subtab_select", main)
                else -> error("Unknown fixture step")
            }
            assertTrue(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.CONNECTED, projection = before), true, true, command))
            for (invalid in listOf(before.copy(subtabs = tabs.copy(activeId = "f".repeat(64))),
                before.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs.first())),
                before.copy(subtabs = tabs.copy(revision = "01")),
                before.copy(subtabs = tabs.copy(tabs = tabs.tabs.mapIndexed { index, tab ->
                    if (index == tabs.tabs.lastIndex) tab.copy(concurrency = 65) else tab })),
                before.copy(subtabs = tabs.copy(tabs = tabs.tabs.mapIndexed { index, tab ->
                    if (index == tabs.tabs.lastIndex) tab.copy(name = "bad\u001b") else tab })),
                before.copy(sessionId = "foreign"), before.copy(subtabs = tabs.copy(projectId = "foreign")))) {
                assertNull(SubtabProjectionCodec.current(invalid))
                assertTrue(runCatching { WorkspaceCommand.subtab(invalid, "subtab_select", main) }.isFailure)
                assertFalse(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.CONNECTED,
                    projection = invalid), true, true, command))
                assertTrue(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.OFFLINE,
                    projection = invalid, pendingWorkspace = command), true, true, command))
            }
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            file("command").outputStream().use { FrameCodec.write(it, command.envelope) }
        } else {
            assertEquals("validate", phase)
            val results = JsonParser.parseString(file("results").readText()).asJsonObject
            val request = results.getAsJsonObject("request")
            val command = WorkspaceCommand.restore(request)
            val exchanges = results.getAsJsonArray("exchanges")
            for (entry in exchanges) {
                val value = entry.asJsonObject
                val sent = value.getAsJsonObject("request")
                val sentCommand = WorkspaceCommand.restore(sent)
                val expected = value["code"]?.takeUnless { it.isJsonNull }?.asString
                val bytes = Base64.getDecoder().decode(value["wire_base64"].asString)
                // Fragmentation exercises production length-prefixed response handling.
                val input = object : ByteArrayInputStream(bytes) {
                    override fun read(target: ByteArray, offset: Int, length: Int): Int = super.read(target, offset, minOf(length, 7))
                }
                val checked = runCatching {
                    ResponseReader.read(input, sent, {
                        if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
                        WorkspaceReceiptCodec.validate(sentCommand, it)
                    })
                }
                if (expected == null) assertTrue(checked.exceptionOrNull().toString(), checked.isSuccess)
                else {
                    assertTrue("$step: ${checked.exceptionOrNull()}", checked.exceptionOrNull() is HostRefusedException)
                    assertEquals(expected, (checked.exceptionOrNull() as HostRefusedException).code)
                }
            }
            val after = projection(results.getAsJsonObject("after"))
            val current = requireNotNull(after.subtabs)
            if (step == "dirty_close") {
                assertEquals(before, after)
                assertEquals("invalid_request", exchanges.single().asJsonObject["code"].asString)
            } else {
                assertEquals(java.math.BigInteger(tabs.revision) + java.math.BigInteger.ONE, java.math.BigInteger(current.revision))
                assertEquals(java.math.BigInteger(requireNotNull(before.workspaceRevision)) + java.math.BigInteger.ONE,
                    java.math.BigInteger(requireNotNull(after.workspaceRevision)))
                assertEquals(exchanges[0].asJsonObject["wire_base64"], exchanges[1].asJsonObject["wire_base64"])
            }
            when (step) {
                "directory_add" -> assertEquals("Source files", current.tabs.single { it.kind == "in_place" }.name)
                "worktree_add" -> assertEquals("android-feature", current.tabs.single { it.kind == "worktree" }.name)
                "rename" -> assertEquals("Android feature", current.tabs.single { it.id == worktree }.name)
                "move" -> assertEquals(listOf(main, worktree, directory), current.tabs.map { it.id })
                "quota64", "quota1" -> assertEquals(if (step == "quota64") 64 else 1, current.tabs.single { it.id == worktree }.concurrency)
                "select_worktree" -> assertEquals(worktree, current.activeId)
                "clean_close" -> assertFalse(current.tabs.any { it.id == worktree })
                "select_directory" -> assertEquals(directory, current.activeId)
                "close_directory" -> assertEquals(listOf(main), current.tabs.map { it.id })
                "select_main" -> assertEquals(main, current.activeId)
            }
            // An old projection cannot admit a new write after a host revision changes.
            if (step != "dirty_close") assertFalse(WorkspaceDispatchPolicy.allows(
                PairingState(phase = PairingPhase.CONNECTED, projection = after), true, true, command))
        }
    }
}
