package zenpi

import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.pairing.*

class SessionCommandTest {
    @Test fun allSessionWritesRequireAnUnambiguousCurrentMainSubtab() {
        val catalog = SessionCatalog(project.id, project.sessionId, "d".repeat(64), 0, null,
            listOf(RemoteSession("history", "1", 2, false)))
        val commands = listOf(SessionCommandCodec.newSession(projection), SessionCommandCodec.rename(projection, "Development"),
            SessionCommandCodec.resume(projection, catalog, "history"))
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, sessions = catalog)
        val tabs = requireNotNull(projection.subtabs)
        val main = tabs.tabs.single()
        val other = RemoteSubtab("e".repeat(64), "Other", "in_place", 1)
        for (invalid in listOf(null, tabs.copy(projectId = "foreign"), tabs.copy(tabs = emptyList()),
            tabs.copy(tabs = listOf(main, main)), tabs.copy(tabs = listOf(main, other, other)),
            tabs.copy(tabs = listOf(main.copy(kind = "in_place"))), tabs.copy(tabs = listOf(main, other.copy(kind = "main"))),
            tabs.copy(activeId = other.id, tabs = listOf(main, other)),
            tabs.copy(tabs = listOf(main) + (1..32).map { other.copy(id = "%064x".format(it)) }))) {
            val changed = projection.copy(subtabs = invalid)
            assertFalse(SessionCommandCodec.mainSelected(changed))
            assertTrue(runCatching { SessionCommandCodec.newSession(changed) }.isFailure)
            assertTrue(runCatching { SessionCommandCodec.rename(changed, "Development") }.isFailure)
            assertTrue(runCatching { SessionCommandCodec.resume(changed, catalog, "history") }.isFailure)
            for (command in commands) {
                val blocked = state.copy(projection = changed)
                assertFalse(WorkspaceDispatchPolicy.allows(blocked, true, true, command))
                val pending = blocked.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = command)
                assertTrue(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.restore(command.envelope)))
                assertFalse(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.restore(command.envelope.apply {
                    addProperty("id", "different-request")
                })))
            }
        }
        val healthy = projection.copy(subtabs = tabs.copy(tabs = listOf(main, other)))
        assertTrue(SessionCommandCodec.mainSelected(healthy))
        for (command in commands) assertTrue(WorkspaceDispatchPolicy.allows(state.copy(projection = healthy), true, true, command))
    }

    @Test fun newResumeRequiresCurrentCatalogSnapshotTargetAndSuccessfulRead() {
        val catalog = SessionCatalog(project.id, project.sessionId, "d".repeat(64), 0, null,
            listOf(RemoteSession("history", "1", 2, false)))
        val command = SessionCommandCodec.resume(projection, catalog, "history")
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, sessions = catalog)
        assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
        for (changed in listOf(state.copy(sessions = null), state.copy(sessionsLoading = true), state.copy(sessionsError = "changed"),
            state.copy(sessions = catalog.copy(projectId = "foreign")), state.copy(sessions = catalog.copy(sessionId = "other")),
            state.copy(sessions = catalog.copy(snapshotId = "e".repeat(64))), state.copy(sessions = catalog.copy(entries = emptyList())),
            state.copy(sessions = catalog.copy(entries = catalog.entries.map { it.copy(current = true) })))) {
            assertFalse(WorkspaceDispatchPolicy.allows(changed, true, true, command))
            assertTrue(WorkspaceDispatchPolicy.allows(changed.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = command),
                true, true, WorkspaceCommand.restore(command.envelope)))
        }
        val next = catalog.copy(snapshotId = "e".repeat(64))
        assertTrue(WorkspaceDispatchPolicy.allows(state.copy(sessions = next), true, true,
            SessionCommandCodec.resume(projection, next, "history")))
    }
    @Test fun sessionCatalogCannotAuthorizeResumeFromAnInconsistentProjectListing() {
        val catalog = SessionCatalog(project.id, project.sessionId, "d".repeat(64), 0, null,
            listOf(RemoteSession("history", "1", 2, false)))
        for (changed in listOf(projection.copy(projects = emptyList()),
            projection.copy(projects = listOf(project.copy(sessionId = "other"))),
            projection.copy(projects = listOf(project, project)))) {
            assertFalse(SessionCatalogCodec.sameScope(catalog, changed))
            assertFalse(SessionCatalogCodec.resumable(catalog, changed, "history"))
            assertTrue(runCatching { SessionCommandCodec.resume(changed, catalog, "history") }.isFailure)
        }
        assertFalse(SessionCatalogCodec.resumable(catalog.copy(entries = catalog.entries + catalog.entries), projection, "history"))
        assertFalse(SessionCatalogCodec.resumable(catalog.copy(entries = catalog.entries + catalog.entries.map { it.copy(current = true) }), projection, "history"))
        assertFalse(SessionCatalogCodec.resumable(catalog.copy(entries = listOf(catalog.entries.single().copy(id = project.sessionId))),
            projection, project.sessionId))
        assertTrue(SessionCatalogCodec.resumable(catalog, projection, "history"))
    }
    private val project = RemoteProject("a".repeat(64), "Workspace", "old-session", sessionControl = true)
    private val tab = "b".repeat(64)
    private val projection = HostProjection(listOf(project), project.id, project.sessionId, "idle", emptyList(), "1", false, "0",
        workspaceOwner = "c".repeat(64), workspaceRevision = "3", subtabs = RemoteSubtabs(project.id, "1", tab,
            listOf(RemoteSubtab(tab, "Main", "main", 1)), true))
    private fun response(command: WorkspaceCommand) = JsonObject().apply {
        addProperty("schema_version", 1); addProperty("channel", "control"); add("id", command.envelope["id"])
        addProperty("execution_state", "succeeded"); addProperty("session_id", "old-session")
        add("payload", JsonObject().apply { addProperty("type", "response"); add("result", JsonObject().apply {
            add("request_id", command.envelope["id"]); addProperty("project_id", project.id); addProperty("old_session_id", "old-session")
            addProperty("new_session_id", "new-session"); addProperty("required_owner", "c".repeat(64))
            addProperty("workspace_revision", 4); addProperty("active_project_id", project.id)
        }) })
    }
    @Test fun newSessionIsPersistedExactlyAndReceiptBindsOldAndNewSessions() {
        val command = SessionCommandCodec.newSession(projection)
        assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        assertEquals("session_new", command.envelope["payload"].asJsonObject["operation"].asString)
        WorkspaceReceiptCodec.validate(command, response(command))
        for ((field, value) in listOf("old_session_id" to "other", "new_session_id" to "old-session", "new_session_id" to "bad\nsession",
            "request_id" to "other", "required_owner" to "d".repeat(64), "project_id" to "other")) {
            val bad = response(command).apply { getAsJsonObject("payload").getAsJsonObject("result").addProperty(field, value) }
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, bad) }.isFailure)
        }
        for (revision in listOf("3", "4.0", "5", "18446744073709551616")) {
            val bad = response(command).apply { getAsJsonObject("payload").getAsJsonObject("result").add("workspace_revision", com.google.gson.JsonParser.parseString(revision)) }
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, bad) }.isFailure)
        }
    }
    @Test fun missingAuthorityNonMainTabAndExtraArgumentsNeverBuildANewSession() {
        assertTrue(runCatching { SessionCommandCodec.newSession(projection.copy(projects = listOf(project.copy(sessionControl = false)))) }.isFailure)
        assertTrue(runCatching { SessionCommandCodec.newSession(projection.copy(subtabs = projection.subtabs!!.copy(activeId = "d".repeat(64)))) }.isFailure)
        val command = SessionCommandCodec.newSession(projection)
        assertTrue(runCatching { WorkspaceCommand.restore(command.envelope.apply { getAsJsonObject("payload").getAsJsonObject("arguments").addProperty("cwd", "/private") }) }.isFailure)
    }
    @Test fun freshWritesRequireForegroundConnectedAndCapabilityIsNeverCached() {
        val command = SessionCommandCodec.newSession(projection)
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection)
        assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state, false, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state, true, false, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state.copy(phase = PairingPhase.ERROR), true, true, command))
        assertTrue(WorkspaceDispatchPolicy.allows(state.copy(phase = PairingPhase.ERROR, pendingWorkspace = command), true, true, command))
        val cached = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(projection))
        assertFalse(cached.projects.single().sessionControl)
        assertTrue(runCatching { SessionCommandCodec.newSession(cached) }.isFailure)
    }
    @Test fun capturedSessionWritesCannotCrossMainTabCapabilityOrJournalChanges() {
        val fresh = SessionCommandCodec.newSession(projection)
        val rename = SessionCommandCodec.rename(projection, "Development")
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection)
        val tabs = requireNotNull(projection.subtabs)
        for (changed in listOf(projection.copy(subtabs = null), projection.copy(subtabs = tabs.copy(projectId = "foreign")),
            projection.copy(subtabs = tabs.copy(activeId = "d".repeat(64), tabs = tabs.tabs + RemoteSubtab("d".repeat(64), "Other", "in_place", 1))),
            projection.copy(projects = listOf(project.copy(sessionControl = false))))) {
            for (command in listOf(fresh, rename)) {
                assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = changed), true, true, command))
            }
        }
        val advanced = state.copy(projection = projection.copy(journalSequence = "2"))
        assertFalse(WorkspaceDispatchPolicy.allows(advanced, true, true, rename))
        assertTrue(WorkspaceDispatchPolicy.allows(advanced, true, true, fresh))
        assertTrue(WorkspaceDispatchPolicy.allows(advanced.copy(pendingWorkspace = rename), true, true, rename))
    }
}
