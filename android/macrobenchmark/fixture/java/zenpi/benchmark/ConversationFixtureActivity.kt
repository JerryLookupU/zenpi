package zenpi.benchmark

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.security.PairedHost
import zenpi.transport.HostProjection
import zenpi.transport.RemoteProject
import zenpi.transport.RemotePromptStatus
import zenpi.transport.RemoteToolCall
import zenpi.transport.RemoteTurn
import zenpi.ui.AppShell
import zenpi.ui.PairingActions
import zenpi.ui.theme.ZenpiTheme

/**
 * ZE-591 Macrobenchmark fixture (benchmark build type only): the production [AppShell] and theme over
 * a connected session whose conversation holds [TURNS] turns (the ZE-530 jank fixture's markdown,
 * code blocks, collapsed tool rows and a streaming row). The live app keeps a 64-turn display tail,
 * so a 10,000-turn list only exists here.
 */
class ConversationFixtureActivity : ComponentActivity() {
    companion object { const val TURNS = 10_000 }

    private fun turn(index: Int): RemoteTurn = if (index % 2 == 0) RemoteTurn("t$index", "user", "Step $index: check `module_$index` and report back.")
    else RemoteTurn("t$index", "assistant", "Turn $index found:\n- `src/file_$index.rs` changed\n- tests ${index % 7} pending\n" +
        "```rust\nfn check_$index(value: u32) -> bool { value % $index == 0 } // a long line that scrolls sideways\n```\nDone.",
        toolCalls = listOf(RemoteToolCall("c$index", "bash", "{\"command\":\"cargo test case_$index\"}", false)))

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val state = PairingState(phase = PairingPhase.CONNECTED,
            host = PairedHost("192.168.1.20", 7443, "e".repeat(64), "d".repeat(64)), registered = true,
            projection = HostProjection(listOf(RemoteProject("p", "zenpi", "s", workspaceControl = true, promptControl = true,
                promptStatuses = listOf(RemotePromptStatus("r", "p", "s", "running", streamText = "Streaming `partial` answer…")))),
                "p", "s", "running", (0 until TURNS).map(::turn), "$TURNS", false, "$TURNS", workspaceOwner = "f".repeat(64), workspaceRevision = "1"))
        setContent { ZenpiTheme { AppShell(state, object : PairingActions {}) } }
    }
}
