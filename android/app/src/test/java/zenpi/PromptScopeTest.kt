package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class PromptScopeTest {
    private val project = RemoteProject("project", "Workspace", "session", promptControl = true)
    private val projection = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false, "0",
        workspaceOwner = "a".repeat(64), workspaceRevision = "3")
    @Test fun messageEnvelopeCannotBorrowControlFromAnotherSession() {
        for (changed in listOf(projection.copy(sessionId = "foreign"), projection.copy(selectedId = "foreign"),
            projection.copy(projects = listOf(project.copy(sessionId = "foreign"))))) {
            assertTrue(runCatching { WorkspaceCommand.prompt(changed, "draft") }.isFailure)
        }
        val command = WorkspaceCommand.prompt(projection, "use @\"file.txt\"")
        assertEquals("project", command.envelope["project_id"].asString)
        assertEquals("session", command.envelope["session_id"].asString)
        assertEquals("use @\"file.txt\"", command.envelope.getAsJsonObject("payload").getAsJsonObject("arguments")["text"].asString)
        assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
    }
}
