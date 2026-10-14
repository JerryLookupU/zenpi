package zenpi

import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class SubtabProjectionTest {
    @Test fun malformedRetainedViewCannotAuthorizeTabCommandsWorkerSubmissionOrScopedReads() {
        val main = RemoteSubtab(id, "main", "main", 1)
        val other = RemoteSubtab("c".repeat(64), "other", "in_place", 2)
        val tabs = RemoteSubtabs(project.id, "2", id, listOf(main, other), true, true, true, true)
        val selected = project.copy(workerControl = true, diffAvailable = true,
            terminal = RemoteTerminal(project.id, project.sessionId, id, "2", "screen", false, "e".repeat(64)), terminalControl = true)
        val projection = HostProjection(listOf(selected), project.id, project.sessionId, "idle", emptyList(), "1", false,
            workspaceOwner = "b".repeat(64), workspaceRevision = "4", subtabs = tabs)
        val commands = listOf(WorkspaceCommand.subtab(projection, "subtab_select", other.id),
            WorkspaceCommand.directoryTab(projection, "child", "child"), WorkspaceCommand.worktreeTab(projection, "feature"),
            WorkspaceCommand.concurrency(projection, id, 2), WorkspaceCommand.workerPrompt(projection, "review"),
            TerminalCommandCodec.create(projection, "pwd\r"))
        val mutations = listOf(tabs.copy(revision = "01"), tabs.copy(revision = "18446744073709551616"),
            tabs.copy(activeId = "invalid", tabs = listOf(main.copy(id = "invalid"), other)),
            tabs.copy(tabs = listOf(main, other.copy(id = "invalid"))),
            tabs.copy(tabs = listOf(main.copy(kind = "in_place"), other)),
            tabs.copy(tabs = listOf(main, other.copy(kind = "main"))),
            tabs.copy(tabs = listOf(main, other.copy(kind = "unknown"))),
            tabs.copy(tabs = listOf(main, other.copy(name = " "))),
            tabs.copy(tabs = listOf(main, other.copy(name = "x".repeat(129)))),
            tabs.copy(tabs = listOf(main, other.copy(name = "bad\u001b"))),
            tabs.copy(tabs = listOf(main, other.copy(concurrency = 0))),
            tabs.copy(tabs = listOf(main, other.copy(concurrency = 65))))
        for (bad in mutations) {
            val invalid = projection.copy(subtabs = bad)
            assertNull(SubtabProjectionCodec.current(invalid))
            val state = zenpi.pairing.PairingState(phase = zenpi.pairing.PairingPhase.CONNECTED, projection = invalid)
            assertFalse(zenpi.pairing.PromptControlPolicy.workerSend(state))
            assertTrue(runCatching { WorkspaceCommand.workerPrompt(invalid, "review") }.isFailure)
            assertTrue(runCatching { WorkspaceCommand.subtab(invalid, "subtab_select", other.id) }.isFailure)
            assertTrue(runCatching { WorkspaceCommand.directoryTab(invalid, "child", "child") }.isFailure)
            assertTrue(runCatching { WorkspaceCommand.worktreeTab(invalid, "feature") }.isFailure)
            assertTrue(runCatching { WorkspaceCommand.concurrency(invalid, id, 2) }.isFailure)
            assertTrue(runCatching { WorkspaceFilesCodec.request(invalid, ".") }.isFailure)
            assertTrue(runCatching { DiffProjectionCodec.request(invalid, ".") }.isFailure)
            assertTrue(runCatching { TerminalPageCodec.request(invalid, null) }.isFailure)
            for (command in commands) {
                assertFalse(zenpi.pairing.WorkspaceDispatchPolicy.allows(state, true, true, command))
                assertTrue(zenpi.pairing.WorkspaceDispatchPolicy.allows(state.copy(pendingWorkspace = command),
                    true, true, WorkspaceCommand.restore(command.envelope)))
            }
        }
        assertNotNull(SubtabProjectionCodec.current(projection.copy(subtabs = tabs.copy(revision = "18446744073709551615",
            tabs = listOf(main.copy(name = "x".repeat(128)), other.copy(concurrency = 64))))))
        assertNotNull(SubtabProjectionCodec.current(projection.copy(subtabs = tabs.copy(control = false,
            directoryControl = false, worktreeControl = false, concurrencyControl = false))))
    }

    @Test fun currentScopeRequiresUniqueProjectAndCompleteActiveTabIdentity() {
        val tabs = RemoteSubtabs("project", "2", id, listOf(RemoteSubtab(id, "main", "main", 1)), true,
            directoryControl = true, worktreeControl = true, concurrencyControl = true)
        val projection = HostProjection(listOf(project), project.id, project.sessionId, "idle", emptyList(), "1", false,
            workspaceOwner = "b".repeat(64), workspaceRevision = "4", subtabs = tabs)
        assertEquals(tabs, SubtabProjectionCodec.current(projection))
        val commands = listOf(WorkspaceCommand.subtab(projection, "subtab_select", id),
            WorkspaceCommand.directoryTab(projection, "child", "child"), WorkspaceCommand.worktreeTab(projection, "feature"),
            WorkspaceCommand.concurrency(projection, id, 2))
        for (invalid in listOf(projection.copy(sessionId = "foreign"),
            projection.copy(projects = listOf(project, project.copy(sessionId = "foreign", workspaceControl = false))),
            projection.copy(subtabs = tabs.copy(projectId = "foreign")), projection.copy(subtabs = tabs.copy(activeId = "c".repeat(64))),
            projection.copy(subtabs = tabs.copy(tabs = emptyList())), projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs)))) {
            assertNull(SubtabProjectionCodec.current(invalid))
            assertTrue(runCatching { WorkspaceCommand.subtab(invalid, "subtab_select", id) }.isFailure)
            assertTrue(runCatching { WorkspaceCommand.directoryTab(invalid, "child", "child") }.isFailure)
            assertTrue(runCatching { WorkspaceCommand.worktreeTab(invalid, "feature") }.isFailure)
            assertTrue(runCatching { WorkspaceCommand.concurrency(invalid, id, 2) }.isFailure)
            for (command in commands) {
                val state = zenpi.pairing.PairingState(phase = zenpi.pairing.PairingPhase.CONNECTED, projection = invalid)
                assertFalse(zenpi.pairing.WorkspaceDispatchPolicy.allows(state, true, true, command))
                assertTrue(zenpi.pairing.WorkspaceDispatchPolicy.allows(state.copy(pendingWorkspace = command), true, true, command))
            }
        }
        assertNotNull(SubtabProjectionCodec.current(projection.copy(subtabs = tabs.copy(control = false))))
    }
    private val id = "a".repeat(64)
    private val project = RemoteProject("project", "Workspace", "session", true)
    private fun fixture() = JsonParser.parseString("""{"project_id":"project","session_id":"session","schema_version":2,"required_owner":"${"b".repeat(64)}","workspace_revision":4,"subtab_revision":2,"active_subtab_id":"$id","tabs":[{"id":"$id","name":"main","kind":"main","concurrency":1}],"subtab_control":true,"directory_tab_control":true,"worktree_control":false,"concurrency_control":false}""").asJsonObject
    private val workspace = JsonParser.parseString("""{"required_owner":"${"b".repeat(64)}","workspace_revision":4}""").asJsonObject
    @Test fun terminalMustMatchCurrentProjectSessionActiveTabAndRevision() {
        val terminal = RemoteTerminal(project.id, project.sessionId, id, "2", "current output", false)
        assertEquals(id, SubtabProjectionCodec.decode(fixture(), project.copy(terminal = terminal), workspace).activeId)
        for (foreign in listOf(terminal.copy(projectId = "foreign"), terminal.copy(sessionId = "foreign"),
                terminal.copy(subtabId = "c".repeat(64)), terminal.copy(subtabRevision = "1"), terminal.copy(subtabRevision = "3"))) {
            assertTrue(runCatching { SubtabProjectionCodec.decode(fixture(), project.copy(terminal = foreign), workspace) }.exceptionOrNull() is WireException)
        }
        assertNotNull(SubtabProjectionCodec.decode(fixture(), project.copy(terminal = null), workspace))
    }
    @Test fun stableIdProjectionHasNoPathAndValidatesCapabilities() {
        val value = SubtabProjectionCodec.decode(fixture(), project, workspace)
        assertEquals(id, value.activeId); assertEquals("2", value.revision); assertTrue(value.control)
        assertEquals(1, value.tabs.single().concurrency)
        val worktree = fixture().apply { addProperty("worktree_control", true) }
        assertTrue(SubtabProjectionCodec.decode(worktree, project, workspace).worktreeControl)
        assertTrue(SubtabProjectionCodec.decode(fixture().apply { addProperty("concurrency_control", true) }, project, workspace).concurrencyControl)
        assertTrue(runCatching { SubtabProjectionCodec.decode(worktree, project.copy(workspaceControl = false), workspace) }.isFailure)
        assertTrue(runCatching { SubtabProjectionCodec.decode(fixture(), project.copy(workspaceControl = false), workspace) }.isFailure)
    }
    @Test fun malformedIdentityOwnerRevisionAndLimitsFailClosed() {
        for (field in listOf("project_id", "session_id", "required_owner", "active_subtab_id", "workspace_revision", "schema_version", "subtab_revision", "worktree_control", "concurrency_control")) {
            val value = fixture()
            when (field) {
                "workspace_revision" -> value.addProperty(field, 5)
                "schema_version" -> value.addProperty(field, 3)
                "subtab_revision" -> value.addProperty(field, "2")
                "worktree_control" -> value.addProperty(field, "true")
                "concurrency_control" -> value.addProperty(field, "true")
                else -> value.addProperty(field, "wrong")
            }
            assertTrue(field, runCatching { SubtabProjectionCodec.decode(value, project, workspace) }.isFailure)
        }
        for (count in listOf(0, 65)) {
            val value = fixture(); value["tabs"].asJsonArray[0].asJsonObject.addProperty("concurrency", count)
            assertTrue(runCatching { SubtabProjectionCodec.decode(value, project, workspace) }.isFailure)
        }
        val duplicate = fixture(); duplicate["tabs"].asJsonArray.add(duplicate["tabs"].asJsonArray[0].deepCopy())
        assertTrue(runCatching { SubtabProjectionCodec.decode(duplicate, project, workspace) }.isFailure)
        val leaked = fixture(); leaked["tabs"].asJsonArray[0].asJsonObject.addProperty("root", "/private/workspace")
        assertTrue(runCatching { SubtabProjectionCodec.decode(leaked, project, workspace) }.isFailure)
    }
}
