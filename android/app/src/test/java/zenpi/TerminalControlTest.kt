package zenpi

import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.pairing.*

class TerminalControlTest {
    @Test fun malformedRetainedTerminalCannotAuthorizeReadInputResizeOrHistoryRetention() {
        val input = TerminalCommandCodec.create(projection, "pwd\r")
        val resize = TerminalCommandCodec.create(projection, rows = 33, cols = 91)
        val page = TerminalPage("project", "session", requireNotNull(terminal.terminalId), tab, "1",
            "d".repeat(64), 0, 3, 6, true, "abc")
        for (bad in listOf(terminal.copy(terminalId = "bad"), terminal.copy(subtabId = "bad"),
            terminal.copy(subtabRevision = "01"), terminal.copy(subtabRevision = "18446744073709551616"),
            terminal.copy(text = "bad\u001b"), terminal.copy(text = "x".repeat(16385)),
            terminal.copy(size = TerminalSize(0, 80)), terminal.copy(size = TerminalSize(24, 513)),
            terminal.copy(terminalId = null, size = TerminalSize(24, 80)), terminal.copy(terminalId = null, running = false))) {
            assertFalse(TerminalProjectionCodec.valid(bad))
            val changed = projection.copy(projects = listOf(project.copy(terminal = bad)))
            assertNull(TerminalPageCodec.terminal(changed))
            assertFalse(TerminalPageCodec.sameScope(projection, changed))
            assertFalse(TerminalPageCodec.matchesSelection(page, changed))
            assertTrue(runCatching { TerminalPageCodec.request(changed, null) }.isFailure)
            assertTrue(runCatching { TerminalCommandCodec.create(changed, "pwd\r") }.isFailure)
            assertTrue(runCatching { TerminalCommandCodec.create(changed, rows = 33, cols = 91) }.isFailure)
            val state = PairingState(phase = PairingPhase.CONNECTED, projection = changed, terminalOpen = true, terminalPage = page)
            assertFalse(TerminalReadPolicy.browse(state)); assertFalse(TerminalReadPolicy.navigation(state))
            for (command in listOf(input, resize)) {
                assertFalse(WorkspaceDispatchPolicy.allows(state, true, true, command))
                assertTrue(WorkspaceDispatchPolicy.allows(state.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = command),
                    true, true, WorkspaceCommand.restore(command.envelope)))
            }
        }
        for (healthy in listOf(terminal.copy(size = TerminalSize(1, 512)), terminal.copy(running = false),
            terminal.copy(terminalId = null))) assertTrue(TerminalProjectionCodec.valid(healthy))
        assertNotNull(TerminalCommandCodec.open(projection.copy(projects = listOf(project.copy(
            terminal = terminal.copy(text = "bad\u001b"), terminalOpenControl = true)))))
    }

    @Test fun duplicateInactiveTabsAndCapacityOverflowCannotAuthorizeTerminalReadsOrWrites() {
        val live = projection.copy(projects = listOf(project.copy(terminalOpenControl = true)))
        val commands = listOf(TerminalCommandCodec.open(live), TerminalCommandCodec.create(live, "pwd\r"),
            TerminalCommandCodec.create(live, rows = 33, cols = 91))
        val tabs = requireNotNull(live.subtabs)
        val other = RemoteSubtab("d".repeat(64), "Other", "in_place", 1)
        val healthy = live.copy(subtabs = tabs.copy(tabs = tabs.tabs + other))
        assertNotNull(TerminalPageCodec.terminal(healthy))
        assertTrue(TerminalReadPolicy.browse(PairingState(phase = PairingPhase.CONNECTED, projection = healthy)))
        for (command in commands) assertTrue(WorkspaceDispatchPolicy.allows(
            PairingState(phase = PairingPhase.CONNECTED, projection = healthy), true, true, command))
        for (invalid in listOf(tabs.copy(tabs = tabs.tabs + listOf(other, other)),
            tabs.copy(tabs = tabs.tabs + (1..32).map { other.copy(id = "%064x".format(it)) }))) {
            val changed = live.copy(subtabs = invalid)
            assertNull(TerminalPageCodec.terminal(changed))
            assertFalse(TerminalPageCodec.sameScope(live, changed))
            assertFalse(TerminalPageCodec.sameScope(changed, live))
            assertTrue(runCatching { TerminalPageCodec.request(changed, null) }.isFailure)
            assertTrue(runCatching { TerminalCommandCodec.open(changed) }.isFailure)
            assertTrue(runCatching { TerminalCommandCodec.create(changed, "pwd\r") }.isFailure)
            assertTrue(runCatching { TerminalCommandCodec.create(changed, rows = 33, cols = 91) }.isFailure)
            val state = PairingState(phase = PairingPhase.CONNECTED, projection = changed)
            assertFalse(TerminalReadPolicy.browse(state))
            for (command in commands) {
                assertFalse(WorkspaceDispatchPolicy.allows(state, true, true, command))
                val pending = state.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = command)
                assertTrue(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.restore(command.envelope)))
                assertFalse(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.restore(command.envelope.apply {
                    addProperty("id", "another-request")
                })))
            }
        }
    }

    private val tab = "b".repeat(64)
    private val terminal = RemoteTerminal("project", "session", tab, "1", "screen", false, "c".repeat(64))
    private val project = RemoteProject("project", "Workspace", "session", terminal = terminal, terminalControl = true)
    private val projection = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false, "0",
        workspaceOwner = "a".repeat(64), workspaceRevision = "3", subtabs = RemoteSubtabs("project", "1", tab,
            listOf(RemoteSubtab(tab, "Main", "main", 1)), true))
    @Test fun ambiguousProjectAndActiveTabCannotReadWriteOpenOrRetainTerminalHistory() {
        val input = TerminalCommandCodec.create(projection, "pwd\r")
        val resize = TerminalCommandCodec.create(projection, rows = 33, cols = 91)
        val openProjection = projection.copy(projects = listOf(project.copy(terminalOpenControl = true)))
        val open = TerminalCommandCodec.open(openProjection)
        val tabs = requireNotNull(openProjection.subtabs)
        for (invalid in listOf(openProjection.copy(projects = openProjection.projects + project.copy(sessionId = "foreign")),
            openProjection.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs)),
            openProjection.copy(subtabs = tabs.copy(tabs = emptyList())))) {
            assertNull(TerminalPageCodec.terminal(invalid))
            assertFalse(TerminalPageCodec.sameScope(projection, invalid))
            assertFalse(TerminalPageCodec.sameScope(invalid, projection))
            assertTrue(runCatching { TerminalPageCodec.request(invalid, null) }.isFailure)
            assertTrue(runCatching { TerminalCommandCodec.create(invalid, "pwd\r") }.isFailure)
            assertTrue(runCatching { TerminalCommandCodec.create(invalid, rows = 33, cols = 91) }.isFailure)
            assertTrue(runCatching { TerminalCommandCodec.open(invalid) }.isFailure)
            val state = PairingState(phase = PairingPhase.CONNECTED, projection = invalid)
            for (command in listOf(input, resize, open)) {
                assertFalse(WorkspaceDispatchPolicy.allows(state, true, true, command))
                assertTrue(WorkspaceDispatchPolicy.allows(state.copy(pendingWorkspace = command), true, true,
                    WorkspaceCommand.restore(command.envelope)))
            }
        }
    }
    private fun receipt(command: WorkspaceCommand) = JsonObject().apply {
        val request = command.envelope
        val args = request["payload"].asJsonObject["arguments"].asJsonObject
        addProperty("schema_version", 1); addProperty("channel", "control"); add("id", request["id"])
        addProperty("execution_state", "succeeded")
        add("payload", JsonObject().apply {
            addProperty("type", "response")
            add("result", JsonObject().apply {
                add("request_id", request["id"]); add("project_id", request["project_id"]); add("session_id", request["session_id"])
                for (field in listOf("terminal_id", "subtab_id", "subtab_revision")) add(field, args[field])
                if (args.has("text")) addProperty("accepted_bytes", args["text"].asString.toByteArray().size)
                else { add("rows", args["rows"]); add("cols", args["cols"]) }
            })
        })
    }
    @Test fun inputAndResizeAreExactPersistableHostBoundWritesWithVerifiedReceipts() {
        for (command in listOf(TerminalCommandCodec.create(projection, "\u4e2d\r"), TerminalCommandCodec.create(projection, rows = 33, cols = 91))) {
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            WorkspaceReceiptCodec.validate(command, receipt(command))
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, receipt(command).apply {
                getAsJsonObject("payload").getAsJsonObject("result").addProperty("subtab_revision", 1.0)
            }) }.isFailure)
            for (field in listOf("request_id", "terminal_id", "subtab_id", "session_id", "project_id")) {
                val invalid = receipt(command).apply { getAsJsonObject("payload").getAsJsonObject("result").addProperty(field, "foreign") }
                assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, invalid) }.isFailure)
            }
        }
        val input = TerminalCommandCodec.create(projection, "\u4e2d\r")
        val invalid = receipt(input).apply { getAsJsonObject("payload").getAsJsonObject("result").addProperty("accepted_bytes", 2) }
        assertTrue(runCatching { WorkspaceReceiptCodec.validate(input, invalid) }.isFailure)
    }
    @Test fun missingControlInstanceScopeOrInvalidBoundsNeverBuildAWrite() {
        for (changed in listOf(project.copy(terminalControl = false), project.copy(terminal = terminal.copy(terminalId = null)),
            project.copy(terminal = terminal.copy(projectId = "foreign")), project.copy(terminal = terminal.copy(sessionId = "foreign")),
            project.copy(terminal = terminal.copy(subtabRevision = "2")), project.copy(terminal = terminal.copy(subtabId = "d".repeat(64))))) {
            assertTrue(runCatching { TerminalCommandCodec.create(projection.copy(projects = listOf(changed)), "pwd\r") }.isFailure)
        }
        for (text in listOf("", "x".repeat(4097), "\u0000")) assertTrue(runCatching { TerminalCommandCodec.create(projection, text) }.isFailure)
        assertTrue(runCatching { TerminalCommandCodec.create(projection, rows = 0) }.isFailure)
        assertTrue(runCatching { TerminalCommandCodec.create(projection, cols = 513) }.isFailure)
        TerminalCommandCodec.create(projection, "x".repeat(4096))
        TerminalCommandCodec.create(projection, rows = 512, cols = 1)
    }
    @Test fun authorityIsNeverCachedAndNewInputIsForegroundConnectedOnly() {
        val cached = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(projection))
        assertFalse(cached.projects.single().terminalControl); assertNull(cached.projects.single().terminal)
        val command = TerminalCommandCodec.create(projection, "pwd\r")
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection)
        assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
        for (phase in listOf(PairingPhase.ERROR, PairingPhase.OFFLINE, PairingPhase.CONNECTING))
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(phase = phase), true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state, false, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state, true, false, command))
        assertTrue(WorkspaceDispatchPolicy.allows(state.copy(phase = PairingPhase.ERROR, pendingWorkspace = command), true, true, command))
    }
    @Test fun connectedInputCannotTargetAnOldTerminalInstanceTabOrLostCapability() {
        val command = TerminalCommandCodec.create(projection, "pwd\r")
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection)
        for (changed in listOf(project.copy(terminalControl = false), project.copy(terminal = null),
            project.copy(terminal = terminal.copy(terminalId = "d".repeat(64))),
            project.copy(terminal = terminal.copy(subtabId = "d".repeat(64))),
            project.copy(terminal = terminal.copy(subtabRevision = "2")))) {
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = projection.copy(projects = listOf(changed))), true, true, command))
        }
        val tabs = requireNotNull(projection.subtabs)
        for (changed in listOf<RemoteSubtabs?>(null, tabs.copy(revision = "2"), tabs.copy(projectId = "foreign"),
            tabs.copy(activeId = "d".repeat(64), tabs = tabs.tabs + RemoteSubtab("d".repeat(64), "Other", "in_place", 1)))) {
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = projection.copy(subtabs = changed)), true, true, command))
        }
    }
    @Test fun foreignTerminalScopeCannotBecomeAnInputOrResizeEnvelopeEvenWithMatchingInstanceAndTab() {
        for (foreign in listOf(terminal.copy(projectId = "foreign"), terminal.copy(sessionId = "foreign"))) {
            val invalid = projection.copy(projects = listOf(project.copy(terminal = foreign)))
            assertNull(TerminalPageCodec.terminal(invalid))
            assertTrue(runCatching { TerminalCommandCodec.create(invalid, "pwd\r") }.isFailure)
            assertTrue(runCatching { TerminalCommandCodec.create(invalid, rows = 24, cols = 80) }.isFailure)
        }
        val input = TerminalCommandCodec.create(projection, "pwd\r")
        val args = input.envelope.getAsJsonObject("payload").getAsJsonObject("arguments")
        assertEquals(terminal.terminalId, args["terminal_id"].asString)
        assertEquals(terminal.subtabId, args["subtab_id"].asString)
        assertEquals(terminal.subtabRevision, args["subtab_revision"].toString())
        assertEquals("project", input.envelope["project_id"].asString)
        assertEquals("session", input.envelope["session_id"].asString)
    }
    @Test fun geometryUpdatesKeepInstanceScopeButNeverRestoreCachedControl() {
        val updated = projection.copy(projects = listOf(project.copy(terminal = terminal.copy(size = TerminalSize(33, 91)))))
        assertTrue(TerminalPageCodec.sameScope(projection, updated))
        val input = TerminalCommandCodec.create(projection, "pwd\r")
        assertTrue(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.CONNECTED, projection = updated), true, true, input))
        val cached = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(updated))
        assertNull(cached.projects.single().terminal)
        assertFalse(cached.projects.single().terminalControl)
    }
    @Test fun openCreatesAnExactPersistableWriteAndVerifiesReturnedHostInstance() {
        val live = projection.copy(projects = listOf(project.copy(terminal = null, terminalControl = false, terminalOpenControl = true)))
        val command = TerminalCommandCodec.open(live)
        val request = command.envelope
        assertEquals("terminal_open", request["payload"].asJsonObject["operation"].asString)
        assertEquals(command.envelope, WorkspaceCommand.restore(request).envelope)
        val response = JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); add("id", request["id"])
            addProperty("execution_state", "succeeded")
            add("payload", JsonObject().apply {
                addProperty("type", "response"); add("result", JsonObject().apply {
                    add("request_id", request["id"]); add("project_id", request["project_id"]); add("session_id", request["session_id"])
                    addProperty("terminal_id", "c".repeat(64)); addProperty("subtab_id", tab); addProperty("subtab_revision", 1)
                })
            })
        }
        WorkspaceReceiptCodec.validate(command, response)
        for (field in listOf("request_id", "project_id", "session_id", "terminal_id", "subtab_id", "subtab_revision")) {
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, response.deepCopy().apply {
                getAsJsonObject("payload").getAsJsonObject("result").addProperty(field, "foreign")
            }) }.isFailure)
        }
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = live)
        assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state.copy(phase = PairingPhase.OFFLINE), true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state, false, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(state, true, false, command))
        val cached = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(live))
        assertFalse(cached.projects.single().terminalOpenControl)
        for (bad in listOf(live.copy(projects = listOf(project.copy(terminalOpenControl = false))),
            live.copy(subtabs = live.subtabs?.copy(projectId = "foreign")), live.copy(subtabs = live.subtabs?.copy(activeId = "foreign")))) {
            assertTrue(runCatching { TerminalCommandCodec.open(bad) }.isFailure)
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = bad), true, true, command))
        }
        assertTrue(WorkspaceDispatchPolicy.allows(state.copy(pendingWorkspace = command, phase = PairingPhase.ERROR), true, true, command))
    }
    @Test fun exitedInstanceCannotReceiveFreshInputButItsReceiptCanStillBeResolved() {
        val command = TerminalCommandCodec.create(projection, "pwd\r")
        val exited = projection.copy(projects = listOf(project.copy(terminal = terminal.copy(running = false), terminalOpenControl = true)))
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = exited)
        assertTrue(runCatching { TerminalCommandCodec.create(exited, "pwd\r") }.isFailure)
        assertTrue(runCatching { TerminalCommandCodec.create(exited, rows = 24, cols = 80) }.isFailure)
        assertFalse(WorkspaceDispatchPolicy.allows(state, true, true, command))
        assertTrue(WorkspaceDispatchPolicy.allows(state.copy(pendingWorkspace = command), true, true, command))
        assertNotNull(TerminalCommandCodec.open(exited))
        assertNotNull(TerminalPageCodec.terminal(exited))
    }
}
