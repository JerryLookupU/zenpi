package zenpi

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.dp
import org.junit.Rule
import org.junit.Test
import org.junit.Assert.assertEquals
import zenpi.pairing.*
import zenpi.transport.*

class TranscriptToolScreenTest {
    @get:Rule val compose = createComposeRule()

    @Test fun ordinaryArgumentsOpenPageAndDisableReadsWhileOffline() {
        val call = RemoteToolCall("call", "read_file", "preview", true)
        val turn = RemoteTurn("turn", "assistant", "", toolCalls = listOf(call))
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session")),
            "project", "session", "idle", listOf(turn), "3", false, "1")
        val page = TranscriptPage("project", "session", "turn", "assistant", "a".repeat(64), 0, 3, 6, true, "abc",
            toolCallId = "call", toolName = "read_file")
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection))
        val opened = mutableListOf<Pair<String, String>>()
        val next = mutableListOf<Pair<String, Boolean>>()
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onToolArguments = { id, tool -> opened += id to tool },
                onTranscript = { id, forward -> next += id to forward })
        } } }
        compose.onNodeWithContentDescription("Read tool arguments call").performScrollTo().performClick()
        compose.runOnIdle {
            assertEquals(listOf("turn" to "call"), opened)
            state.value = state.value.copy(transcriptTurn = "turn", transcriptToolCall = call, transcriptPage = page)
        }
        compose.onNodeWithTag("transcript-page").assertTextEquals("abc")
        compose.onNodeWithContentDescription("Next message page").performClick()
        compose.runOnIdle { assertEquals(listOf("turn" to true), next) }
        val valid = state.value
        for (invalid in listOf(valid.copy(transcriptToolCall = call.copy(id = "other")), valid.copy(resetting = true),
            valid.copy(transcriptPage = page.copy(toolName = "foreign")),
            valid.copy(projection = projection.copy(turns = listOf(turn.copy(toolCalls = emptyList())))))) {
            compose.runOnIdle { state.value = invalid }
            compose.onNodeWithContentDescription("Next message page").assertIsNotEnabled()
        }
        compose.runOnIdle { state.value = valid.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithContentDescription("Next message page").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(transcriptTurn = null, transcriptToolCall = null, transcriptPage = null) }
        compose.onNodeWithContentDescription("Read tool arguments call").performScrollTo().assertIsNotEnabled()
        for (bad in listOf(turn.copy(toolCallsTruncated = true),
            turn.copy(toolCalls = listOf(call.copy(argumentsPreview = "x".repeat(513)))))) {
            compose.runOnIdle { state.value = PairingState(phase = PairingPhase.CONNECTED,
                projection = projection.copy(turns = listOf(bad))) }
            compose.onNodeWithContentDescription("Read tool arguments call").performScrollTo().assertIsNotEnabled()
        }
    }

    @Test fun ordinaryTranscriptShowsToolPreviewAndRemovesItOnCacheReplacement() {
        val project = RemoteProject("project", "Workspace", "session")
        val call = RemoteToolCall("call", "read_file", "{\"path\":\"note.txt\"}", true)
        val turn = RemoteTurn("turn", "assistant", "", toolCalls = listOf(call) + (1..7).map {
            call.copy(id = "call-$it", argumentsPreview = "{}", argumentsTruncated = false) }, toolCallsTruncated = true)
        val projection = HostProjection(listOf(project), "project", "session", "idle", listOf(turn), "3", false, "1")
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection))
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {})
        } } }
        compose.onNodeWithTag("transcript-tool-call").performScrollTo().assertTextEquals("read_file")
        compose.onNodeWithText("{\"path\":\"note.txt\"}").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Arguments truncated").assertExists()
        compose.onNodeWithText("Tool list truncated").assertExists()
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.OFFLINE,
            projection = projection.copy(turns = listOf(turn.copy(toolCalls = emptyList(), toolCallsTruncated = false)))) }
        compose.onNodeWithTag("transcript-tool-call").assertDoesNotExist()
        compose.onNodeWithText("Arguments truncated").assertDoesNotExist()
    }
}
