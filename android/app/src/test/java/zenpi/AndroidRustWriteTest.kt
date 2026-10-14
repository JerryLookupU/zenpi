package zenpi

import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Assume.assumeNotNull
import org.junit.Test
import zenpi.transport.*

/** Invoked by the Rust integration lane with its live fixture-local owner. */
class AndroidRustWriteTest {
    @Test fun generatedRequestAndActualHostReceiptsPreserveMutationIdentity() = runBlocking {
        val phase = System.getProperty("zenpi.write.phase")
        assumeNotNull(phase)
        fun file(property: String) = java.io.File(requireNotNull(System.getProperty(property)))
        val seed = JsonParser.parseString(file("zenpi.write.seed").readText()).asJsonObject
        if (phase == "generate") {
            val listing = seed.getAsJsonObject("listing")
            val projects = HostProjectionCodec.projects(listing); val selected = projects.single()
            val projection = HostProjectionCodec.workspace(HostProjectionCodec.snapshot(projects, selected, seed.getAsJsonObject("snapshot")), listing)
                .copy(subtabs = SubtabProjectionCodec.decode(seed.getAsJsonObject("subtabs"), selected, listing))
            assertTrue(selected.sessionControl)
            val command = SessionCommandCodec.rename(projection, "Android development")
            file("zenpi.write.command").outputStream().use { FrameCodec.write(it, command.envelope) }
        } else {
            assertEquals("validate", phase)
            val results = JsonParser.parseString(file("zenpi.write.results").readText()).asJsonArray
            assertEquals(6, results.size())
            for ((index, entry) in results.take(5).withIndex()) {
                val value = entry.asJsonObject; val request = value.getAsJsonObject("request")
                val wire = java.util.Base64.getDecoder().decode(value["wire_base64"].asString)
                var confirmed = false
                val error = runCatching {
                    val command = WorkspaceCommand.restore(request)
                    val response = ResponseReader.read(java.io.ByteArrayInputStream(wire), request, {
                        if (FrameCodec.string(it, "execution_state") == "failed") throw HostRefusedException(FrameCodec.string(it, "code"))
                        assertEquals("succeeded", FrameCodec.string(it, "execution_state"))
                        WorkspaceReceiptCodec.validate(command, it)
                    })
                    SessionCommandCodec.renameReceipt(request, response.result)
                    confirmed = true
                }.exceptionOrNull()
                if (index < 2) { assertNull(error); assertTrue(confirmed) }
                else {
                    assertFalse(confirmed); assertTrue(error is HostRefusedException)
                    assertEquals(listOf("revision_conflict", "request_id_conflict", "unauthorized")[index - 2], (error as HostRefusedException).code)
                }
            }
            assertEquals(results[0].asJsonObject["wire_base64"], results[1].asJsonObject["wire_base64"])
            val listing = results[5].asJsonObject
            val response = ResponseReader.read(java.io.ByteArrayInputStream(java.util.Base64.getDecoder().decode(listing["wire_base64"].asString)),
                listing.getAsJsonObject("request"), { assertEquals("succeeded", FrameCodec.string(it, "execution_state")) })
            val current = HostProjectionCodec.projects(response.result).single()
            assertEquals("Desktop newer name", current.sessionName)
            assertFalse(current.sessionControl)
        }
    }
}
