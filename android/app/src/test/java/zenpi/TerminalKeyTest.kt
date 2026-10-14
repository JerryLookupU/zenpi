package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.pairing.*

class TerminalKeyTest {
    private val tab = "b".repeat(64)
    private val terminal = RemoteTerminal("project", "session", tab, "1", "screen", false, "c".repeat(64))
    private val project = RemoteProject("project", "Workspace", "session", terminal = terminal, terminalControl = true)
    private val projection = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false, "0",
        workspaceOwner = "a".repeat(64), workspaceRevision = "3", subtabs = RemoteSubtabs("project", "1", tab,
            listOf(RemoteSubtab(tab, "Main", "main", 1)), true))

    @Test fun fixedKeysBuildExactDurableTerminalInputWithoutNewExecutionAuthority() {
        val expected = listOf("\u007f", "\u001b[3~", "\u001b[H", "\u001b[F", "\u001b[5~", "\u001b[6~",
            "\u001b[2~", "\u0004", "\u001a", "\u0001", "\u0005", "\u000c")
        assertEquals(expected, TerminalKey.entries.map { it.bytes })
        assertEquals(TerminalKey.entries.size, TerminalKey.entries.map { it.label }.distinct().size)
        for (key in TerminalKey.entries) {
            val command = TerminalCommandCodec.create(projection, key.bytes)
            assertEquals(key.bytes, command.envelope["payload"].asJsonObject["arguments"].asJsonObject["text"].asString)
            assertEquals("terminal_input", command.envelope["payload"].asJsonObject["operation"].asString)
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection)
            assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(state, false, true, command))
            assertFalse(WorkspaceDispatchPolicy.allows(state, true, false, command))
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(phase = PairingPhase.OFFLINE), true, true, command))
            val replaced = projection.copy(projects = listOf(project.copy(terminal = terminal.copy(terminalId = "d".repeat(64)))))
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = replaced), true, true, command))
        }
    }
}
