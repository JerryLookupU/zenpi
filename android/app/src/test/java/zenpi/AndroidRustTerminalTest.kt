package zenpi

import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Assume.assumeNotNull
import org.junit.Test
import zenpi.transport.*
import zenpi.pairing.*

/** Host-owned PTY effects and retained screen, not an Android shell or LAN receipt. */
class AndroidRustTerminalTest {
    @Test fun commandsAndHistoryPagesUseActualHostTerminalIdentity() = runBlocking {
        val phase = System.getProperty("zenpi.terminal.phase")
        assumeNotNull(phase)
        fun file(name: String) = java.io.File(requireNotNull(System.getProperty("zenpi.terminal.$name")))
        val seed = JsonParser.parseString(file("seed").readText()).asJsonObject
        val listing = seed.getAsJsonObject("listing")
        val projects = HostProjectionCodec.projects(listing); val project = projects.single()
        val projection = HostProjectionCodec.workspace(HostProjectionCodec.snapshot(projects, project, seed.getAsJsonObject("snapshot")), listing)
            .copy(subtabs = SubtabProjectionCodec.decode(seed.getAsJsonObject("subtabs"), project, listing))
        val terminal = requireNotNull(TerminalPageCodec.terminal(projection))
        assertTrue(terminal.running)
        for (bad in listOf(terminal.copy(text = "bad\u001b"), terminal.copy(size = TerminalSize(0, 80)),
            terminal.copy(terminalId = "invalid"))) {
            val changed = projection.copy(projects = listOf(project.copy(terminal = bad)))
            assertNull(TerminalPageCodec.terminal(changed))
            assertFalse(TerminalPageCodec.sameScope(projection, changed))
            assertTrue(runCatching { TerminalPageCodec.request(changed, null) }.isFailure)
            assertTrue(runCatching { TerminalCommandCodec.create(changed, "x") }.isFailure)
        }
        val ambiguous = projection.copy(projects = projects + project.copy(sessionId = "foreign"))
        assertNull(TerminalPageCodec.terminal(ambiguous))
        assertFalse(TerminalPageCodec.sameScope(projection, ambiguous))
        assertTrue(runCatching { TerminalPageCodec.request(ambiguous, null) }.isFailure)
        assertTrue(runCatching { TerminalCommandCodec.open(ambiguous) }.isFailure)
        assertTrue(runCatching { TerminalCommandCodec.create(ambiguous, "x") }.isFailure)
        val tabs = requireNotNull(projection.subtabs)
        val other = RemoteSubtab("e".repeat(64), "Other", "in_place", 1)
        val invalidTabs = projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + listOf(other, other)))
        assertNull(TerminalPageCodec.terminal(invalidTabs))
        assertFalse(TerminalPageCodec.sameScope(projection, invalidTabs))
        assertTrue(runCatching { TerminalPageCodec.request(invalidTabs, null) }.isFailure)
        assertTrue(runCatching { TerminalCommandCodec.open(invalidTabs) }.isFailure)
        assertTrue(runCatching { TerminalCommandCodec.create(invalidTabs, "x") }.isFailure)
        if (phase == "generate") {
            assertTrue(project.terminalControl)
            val resize = TerminalCommandCodec.create(projection, rows = 33, cols = 91)
            val input = TerminalCommandCodec.create(projection,
                "printf '%s%s\\n' 'ANDROID_PTY_' 'ONCE' >> effect.txt; stty size > size.txt; i=0; while [ \$i -lt 240 ]; do printf 'android-history-%s abcdefghijklmnopqrstuvwxyz\\n' \"\$i\"; i=\$((i+1)); done; printf 'api_key=%s\\n' 'INTEROP_PTY_PRIVATE'; printf '%s%s\\n' 'ANDROID_PTY_' 'FINISHED'\r")
            file("commands").outputStream().use { output ->
                FrameCodec.write(output, resize.envelope); FrameCodec.write(output, input.envelope)
                FrameCodec.write(output, TerminalPageCodec.request(projection, null))
            }
            return@runBlocking
        }
        assertEquals("read", phase)
        assertFalse(project.terminalControl); assertFalse(project.promptControl)
        assertTrue(runCatching { TerminalCommandCodec.create(projection, "x") }.isFailure)
        assertTrue(runCatching { TerminalCommandCodec.create(projection, rows = 24, cols = 80) }.isFailure)
        val results = JsonParser.parseString(file("results").readText()).asJsonObject
        for (field in listOf("read_only_controls", "read_refusals")) {
            val exchanges = results.getAsJsonArray(field)
            assertTrue(exchanges.size() > 0)
            for (raw in exchanges) {
                val value = raw.asJsonObject
                val request = value.getAsJsonObject("request")
                val bytes = java.util.Base64.getDecoder().decode(value["wire_base64"].asString)
                val error = runCatching { ResponseReader.read(java.io.ByteArrayInputStream(bytes), request, {
                    if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
                }) }.exceptionOrNull()
                assertTrue(error.toString(), error is HostRefusedException)
                val expected = if (field == "read_only_controls" || FrameCodec.string(request, "session_id") != projection.sessionId) {
                    if (field == "read_only_controls") "unauthorized" else "owner_mismatch"
                } else "unauthorized"
                assertEquals(expected, (error as HostRefusedException).code)
            }
        }
        val commands = results["commands"].asJsonArray
        val codes = listOf("revision_conflict", "owner_mismatch", "unauthorized", null, null, null, "request_id_conflict", null, null, "invalid_request")
        assertEquals(codes.size, commands.size())
        for ((index, raw) in commands.withIndex()) {
            val value = raw.asJsonObject; val request = value.getAsJsonObject("request")
            val command = WorkspaceCommand.restore(request)
            val wire = java.util.Base64.getDecoder().decode(value["wire_base64"].asString)
            var confirmed = false
            val error = runCatching { ResponseReader.read(java.io.ByteArrayInputStream(wire), request, {
                if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
                WorkspaceReceiptCodec.validate(command, it); confirmed = true
            }) }.exceptionOrNull()
            if (codes[index] == null) { assertNull(error); assertTrue(confirmed) }
            else { assertFalse(confirmed); assertTrue(error is HostRefusedException); assertEquals(codes[index], (error as HostRefusedException).code) }
        }
        assertEquals(commands[4].asJsonObject["wire_base64"], commands[5].asJsonObject["wire_base64"])
        assertEquals(commands[4].asJsonObject["wire_base64"], commands[8].asJsonObject["wire_base64"])
        var current: TerminalPage? = null
        var history = emptyList<TerminalPage>()
        val text = StringBuilder()
        for (raw in results["pages"].asJsonArray) {
            val value = raw.asJsonObject; val request = value.getAsJsonObject("request")
            val expected = TerminalPageCodec.request(projection, current).apply { add("id", request["id"]) }
            assertEquals(expected, request)
            val bytes = java.util.Base64.getDecoder().decode(value["wire_base64"].asString)
            val page = ResponseReader.read(object : java.io.ByteArrayInputStream(bytes) {
                override fun read(bytes: ByteArray, offset: Int, length: Int) = super.read(bytes, offset, minOf(length, 3))
            }, request, { assertEquals("succeeded", FrameCodec.string(it, "execution_state")) })
            val next = TerminalPageCodec.decode(page.result, projection, current)
            assertTrue(runCatching { TerminalPageCodec.decode(page.result, invalidTabs, current) }.isFailure)
            assertFalse(TerminalPageCodec.matchesSelection(next, invalidTabs))
            assertTrue(runCatching { TerminalPageCodec.decode(page.result, ambiguous, current) }.isFailure)
            assertFalse(TerminalPageCodec.matchesSelection(next, ambiguous))
            assertTrue(runCatching { TerminalPageCodec.request(ambiguous, next) }.isFailure)
            history = TerminalPageCodec.remember(history, current, next); current = next
            assertTrue(TerminalPageCodec.validChain(history, next)); text.append(next.text)
            val readOnly = PairingState(phase = PairingPhase.CONNECTED, terminalOpen = true,
                projection = projection,
                terminalPage = next, terminalPrevious = history)
            assertTrue(TerminalReadPolicy.browse(readOnly)); assertTrue(TerminalReadPolicy.navigation(readOnly))
            assertFalse(TerminalReadPolicy.navigation(readOnly.copy(scanInProgress = true)))
            assertTrue(runCatching { TerminalCommandCodec.create(requireNotNull(readOnly.projection), "x") }.isFailure)
        }
        val last = requireNotNull(current)
        assertFalse(text.contains("INTEROP_PTY_PRIVATE"))
        if (last.hasMore) file("next").outputStream().use { FrameCodec.write(it, TerminalPageCodec.request(projection, last)) }
        else {
            assertTrue(results["pages"].asJsonArray.size() > 1)
            assertTrue(text.contains("android-history-0")); assertTrue(text.contains("android-history-239"))
            assertTrue(text.contains("ANDROID_PTY_FINISHED"))
            val digest = java.security.MessageDigest.getInstance("SHA-256").digest(text.toString().toByteArray())
                .joinToString("") { "%02x".format(it.toInt() and 255) }
            assertEquals(last.digest, digest)
            file("next").writeBytes(byteArrayOf())
        }
    }
}
