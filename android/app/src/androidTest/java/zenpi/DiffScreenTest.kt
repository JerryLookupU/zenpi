package zenpi

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.*
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class DiffScreenTest {
    @get:Rule val compose = createComposeRule()
    private val projection = HostProjection(listOf(RemoteProject("project", "Project", "session", diffAvailable = true)),
        "project", "session", "idle", emptyList(), "1", false,
        subtabs = RemoteSubtabs("project", "1", "b".repeat(64), listOf(RemoteSubtab("b".repeat(64), "main", "main", 1)), false))
    @Test fun previousMessagePageUsesAnIconAndIsFencedDuringLoadingOfflineOrUnknownWrite() {
        val first = TranscriptPage("project", "session", "turn", "assistant", "a".repeat(64), 0, 3, 6, true, "abc")
        val second = first.copy(offset = 3, nextOffset = 6, hasMore = false, text = "def")
        var state by mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection.copy(turns = listOf(RemoteTurn("turn", "assistant", "abc", true))),
            transcriptTurn = "turn", transcriptPage = second, transcriptPreviousPages = listOf(first)))
        var backwards = 0
        compose.setContent { MaterialTheme { PairingScreen(state, {}, {}, {}, onPreviousTranscript = { backwards++ }) } }
        compose.onNodeWithContentDescription("Previous message page").assertIsEnabled().performClick()
        assertEquals(1, backwards)
        compose.onNodeWithContentDescription("Next message page").assertIsNotEnabled()
        for (blocked in listOf(state.copy(transcriptLoading = true), state.copy(phase = PairingPhase.OFFLINE),
            state.copy(pendingRevoke = "unknown"), state.copy(transcriptError = "Message page unavailable"))) {
            compose.runOnIdle { state = blocked }
            compose.onNodeWithContentDescription("Previous message page").assertIsNotEnabled()
        }
        compose.runOnIdle { state = state.copy(phase = PairingPhase.CONNECTED, transcriptLoading = false, transcriptError = null,
            pendingRevoke = null, transcriptPreviousPages = emptyList(), transcriptPage = first) }
        compose.onNodeWithContentDescription("Previous message page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Next message page").assertIsEnabled()
        val valid = state
        for (invalid in listOf(valid.copy(resetting = true), valid.copy(resetRequired = true), valid.copy(scanInProgress = true),
            valid.copy(transcriptPage = first.copy(sessionId = "foreign")),
            valid.copy(projection = valid.projection?.copy(turns = emptyList())),
            valid.copy(transcriptPreviousPages = listOf(first)))) {
            compose.runOnIdle { state = invalid }
            compose.onNodeWithContentDescription("Previous message page").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Next message page").assertIsNotEnabled()
        }
        compose.runOnIdle { state = valid.copy(transcriptTurn = null, transcriptPage = null, scanInProgress = true) }
        compose.onNodeWithContentDescription("Read full message turn").performScrollTo().assertIsNotEnabled()
        compose.runOnIdle { state = state.copy(scanInProgress = false) }
        compose.onNodeWithContentDescription("Read full message turn").assertIsEnabled()
    }
    @Test fun readOnlyDiffControlIsFencedOfflineAndDuringPendingWrites() {
        var state by mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection))
        val requests = mutableListOf<String>()
        compose.setContent { MaterialTheme { PairingScreen(state, {}, {}, {}, onDiff = { requests.add(it) }) } }
        compose.onNodeWithContentDescription("Review diff").performScrollTo().performClick()
        assertEquals(listOf("."), requests)
        compose.runOnIdle { state = state.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithContentDescription("Review diff").assertIsNotEnabled()
        compose.runOnIdle { state = state.copy(phase = PairingPhase.CONNECTED, pendingRevoke = "pending") }
        compose.onNodeWithContentDescription("Review diff").assertIsNotEnabled()
        val valid = state.copy(pendingRevoke = null)
        for (blocked in listOf(valid.copy(scanInProgress = true), valid.copy(resetting = true), valid.copy(resetRequired = true),
            valid.copy(projection = projection.copy(subtabs = null)), valid.copy(projection = projection.copy(sessionId = "foreign")))) {
            compose.runOnIdle { state = blocked }
            compose.onNodeWithContentDescription("Review diff").assertIsNotEnabled()
        }
    }
    @Test fun boundedPatchFlagsPathAndCloseAreVisibleOnNarrowDisplay() {
        val diff = RemoteDiff("project", "session", "b".repeat(64), "1", "file.txt", true, false, true, "+changed\n", " M file.txt", DiffPage("a".repeat(64),0,9,18,true))
        var state by mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection, diffOpen = true, diff = diff))
        val requests = mutableListOf<String>(); var closed = false; var next = 0; var previous = 0
        compose.setContent { MaterialTheme { PairingScreen(state, {}, {}, {}, onDiff = { requests.add(it) }, onCloseDiff = { closed = true },
            onNextDiff = { next++ }, onPreviousDiff = { previous++ }) } }
        compose.onNodeWithText("Diff truncated").assertIsDisplayed()
        compose.onNodeWithText("+changed\n").assertIsDisplayed()
        compose.onNodeWithText("Path").performTextReplacement("file.txt")
        compose.onNodeWithContentDescription("Refresh diff").performClick()
        compose.onNodeWithText("Path").assertIsNotFocused()
        assertEquals(listOf("file.txt"), requests)
        compose.onNodeWithContentDescription("Previous diff page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Next diff page").performClick(); assertEquals(1, next)
        val tail = diff.copy(diff = "+ending!\n", page = DiffPage("a".repeat(64),9,18,18,false))
        compose.runOnIdle { state = state.copy(diff = tail, diffPrevious = listOf(diff)) }
        compose.onNodeWithContentDescription("Previous diff page").performClick(); assertEquals(1, previous)
        compose.onNodeWithContentDescription("Next diff page").assertIsNotEnabled()
        val healthy = state
        val tabs = requireNotNull(projection.subtabs)
        val inactive = RemoteSubtab("e".repeat(64), "Other", "in_place", 1)
        compose.runOnIdle { state = healthy.copy(projection = projection.copy(
            subtabs = tabs.copy(tabs = tabs.tabs + listOf(inactive, inactive)))) }
        compose.onNodeWithContentDescription("Previous diff page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Refresh diff").assertIsNotEnabled()
        compose.onNodeWithText("Path").assertIsNotEnabled()
        compose.runOnIdle { state = healthy.copy(diff = diff, diffPrevious = listOf(diff.copy(diff = "a"))) }
        compose.onNodeWithContentDescription("Next diff page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Refresh diff").assertIsEnabled()
        compose.runOnIdle { state = healthy.copy(projection = projection.copy(projects = projection.projects + projection.projects.single().copy(sessionId = "foreign"))) }
        compose.onNodeWithContentDescription("Previous diff page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Refresh diff").assertIsNotEnabled()
        compose.runOnIdle { state = healthy.copy(scanInProgress = true) }
        compose.onNodeWithContentDescription("Previous diff page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Refresh diff").assertIsNotEnabled()
        compose.onNodeWithText("Path").assertIsNotEnabled()
        compose.runOnIdle { state = healthy.copy(diffError = "Changed") }
        compose.onNodeWithContentDescription("Refresh diff").assertIsEnabled()
        compose.onNodeWithContentDescription("Previous diff page").assertIsNotEnabled()
        compose.runOnIdle { state = healthy.copy(projection = projection.copy(projects = listOf(projection.projects.single().copy(diffAvailable = false)))) }
        compose.onNodeWithContentDescription("Refresh diff").assertIsNotEnabled()
        compose.runOnIdle { state = healthy }
        for (blocked in listOf(state.copy(diff = tail.copy(sessionId = "foreign")), state.copy(diffError = "Changed"),
            state.copy(phase = PairingPhase.OFFLINE), state.copy(resetting = true), state.copy(diffPrevious = listOf(diff.copy(status = "different"))))) {
            compose.runOnIdle { state = blocked }
            compose.onNodeWithContentDescription("Previous diff page").assertIsNotEnabled()
        }
        compose.onNodeWithContentDescription("Close diff").performClick(); assertTrue(closed)
    }
}
