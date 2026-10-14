package zenpi

import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class SessionRenameTest {
    private val project = RemoteProject("a".repeat(64), "Workspace", "session", sessionControl = true)
    private val tab = "b".repeat(64)
    private val projection = HostProjection(listOf(project), project.id, "session", "idle", emptyList(), "10", false, "0",
        workspaceOwner = "c".repeat(64), workspaceRevision = "3", subtabs = RemoteSubtabs(project.id, "1", tab,
            listOf(RemoteSubtab(tab, "Main", "main", 1)), true))
    @Test fun renameBindsCurrentJournalAndSavedRequestAndExactReceipt() {
        val command = SessionCommandCodec.rename(projection, "Development")
        assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        val request = command.envelope
        val args = request.getAsJsonObject("payload").getAsJsonObject("arguments")
        assertEquals("10", args["journal_revision"].toString())
        val result = JsonObject().apply {
            add("request_id", request["id"]); add("project_id", request["project_id"]); add("session_id", request["session_id"])
            addProperty("name", "Development"); addProperty("journal_revision", 12)
        }
        SessionCommandCodec.renameReceipt(request, result)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("name", "Other") }, { it.addProperty("session_id", "other") },
            { it.addProperty("journal_revision", 10) }, { it.addProperty("journal_revision", "12") },
            { it.addProperty("journal_revision", -1) }, { it.addProperty("path", "/host") })) {
            assertTrue(runCatching { SessionCommandCodec.renameReceipt(request, result.deepCopy().apply(change)) }.isFailure)
        }
    }
    @Test fun invalidNamesMissingAuthorityAndChangedSubtabCannotWrite() {
        for (name in listOf("", " padded ", "bad/name", "bad\\name", "bad\nname", "x".repeat(65), "\u6c49".repeat(22)))
            assertTrue(runCatching { SessionCommandCodec.rename(projection, name) }.isFailure)
        assertEquals("\u6c49".repeat(21), SessionCommandCodec.validateName("\u6c49".repeat(21)))
        assertTrue(runCatching { SessionCommandCodec.rename(projection.copy(projects = listOf(project.copy(sessionControl = false))), "Valid") }.isFailure)
        assertTrue(runCatching { SessionCommandCodec.rename(projection.copy(subtabs = projection.subtabs!!.copy(activeId = "d".repeat(64))), "Valid") }.isFailure)
    }
    @Test fun projectedNamesAreOptionalBoundedAndNeverCachedAuthority() {
        val listing = JsonObject().apply {
            addProperty("mutations_available", false)
            add("projects", com.google.gson.JsonArray().apply { add(JsonObject().apply {
                addProperty("id", project.id); addProperty("title", project.title); addProperty("session_id", project.sessionId)
                addProperty("session_name", "Development")
            }) })
        }
        assertEquals("Development", HostProjectionCodec.projects(listing).single().sessionName)
        val cached = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(projection.copy(projects = listOf(project.copy(sessionName = "Development")))))
        assertNull(cached.projects.single().sessionName)
        listing.getAsJsonArray("projects")[0].asJsonObject.addProperty("session_name", "bad\nname")
        assertTrue(runCatching { HostProjectionCodec.projects(listing) }.isFailure)
    }
}
