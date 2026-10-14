package zenpi

import zenpi.ui.PairingScreen
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.dp
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.state.ProjectionCacheCodec
import zenpi.transport.*

class PromptScreenTest {
    @Test fun terminalFailureShowsTypedInputDiagnosisOfflineWithoutSendingOrStopping() {
        val failed=projection.copy(projects=projection.projects.map{it.copy(promptStatuses=listOf(
            RemotePromptStatus(target,"project","session","failed",failureCode="invalid_request")))})
        val cached=ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(failed))
        var writes=0
        compose.setContent{MaterialTheme{Box(Modifier.width(320.dp)){PairingScreen(PairingState(phase=PairingPhase.OFFLINE,projection=cached),{},{},{},
            onPrompt={writes++},onCancelPrompt={writes++})}}}
        compose.onNodeWithText("Message input rejected").performScrollTo().assertIsDisplayed()
        compose.onNodeWithContentDescription("Cancel message $target").assertDoesNotExist()
        assertEquals(0,writes)
    }
    @Test fun liveComposerSteersExactTargetAndOfflineCacheRemovesWriteControls() {
        val state = mutableStateOf(PairingState(phase = PairingPhase.ERROR, projection = projection))
        val steered = mutableListOf<Pair<String, String>>()
        var prompts = 0
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onPrompt = { prompts++ }, onSteer = { target, text -> steered.add(target to text) })
        } } }
        compose.onNodeWithText("Steer").performScrollTo().performTextInput("change direction")
        val send = compose.onNodeWithContentDescription("Send steering input")
        send.performScrollTo().assertIsDisplayed().assertIsEnabled().performClick()
        assertEquals(listOf(target to "change direction"), steered); assertEquals(0, prompts)
        val healthy = state.value
        for (blocked in listOf(healthy.copy(scanInProgress = true), healthy.copy(resetting = true), healthy.copy(resetRequired = true))) {
            compose.runOnUiThread { state.value = blocked }
            send.assertIsNotEnabled()
            compose.onNodeWithText("Steer").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Cancel message $target").assertIsNotEnabled()
        }
        compose.runOnUiThread { state.value = healthy.copy(projection = projection.copy(
            projects = projection.projects + projection.projects.single().copy(sessionId = "foreign"))) }
        send.assertDoesNotExist()
        compose.onNodeWithText("Steer").assertDoesNotExist()
        compose.onNodeWithContentDescription("Cancel message $target").assertDoesNotExist()
        compose.onNodeWithContentDescription("Send message").assertIsNotEnabled()
        compose.runOnIdle { assertEquals(listOf(target to "change direction"), steered); assertEquals(0, prompts) }
        compose.runOnUiThread { state.value = healthy }
        compose.runOnUiThread { state.value = state.value.copy(pendingWorkspace = WorkspaceCommand.steer(projection, target, "change direction")) }
        send.assertIsNotEnabled()
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = null,
            projection = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(projection))) }
        send.assertDoesNotExist()
        assertEquals(1, steered.size)
    }
    @get:Rule val compose = createComposeRule()
    @Test fun steeringCommandsAndFileReferencesRetainDraftWithoutSending() {
        val steered = mutableListOf<String>()
        compose.setContent { MaterialTheme { PairingScreen(PairingState(phase = PairingPhase.CONNECTED, projection = projection), {}, {}, {},
            onSteer = { _, text -> steered.add(text) }) } }
        for (text in listOf("/skill:research", " /review literal", "use @\"file name.txt\"", "@\"unfinished", "\u0085/unknown")) {
            compose.onNode(hasSetTextAction()).performScrollTo().performTextReplacement(text)
            compose.onNodeWithContentDescription("Send steering input").assertIsNotEnabled()
            compose.onNode(hasSetTextAction()).assertTextContains(text, substring = true)
        }
        assertTrue(steered.isEmpty())
        compose.onNode(hasSetTextAction()).performTextReplacement("use a@example.invalid")
        compose.onNodeWithContentDescription("Send steering input").assertIsEnabled().performClick()
        assertEquals(listOf("use a@example.invalid"), steered)
    }
    @Test fun committedTruncatedMessageIsExplicitInOfflineDisplay() {
        val cached = projection.copy(turns = listOf(RemoteTurn("large", "assistant", "Durable prefix", true)), truncated = true)
        compose.setContent { MaterialTheme { PairingScreen(PairingState(phase = PairingPhase.OFFLINE, projection = cached), {}, {}, {}) } }
        compose.onNodeWithText("Durable prefix").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Message truncated").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Transcript truncated").performScrollTo().assertIsDisplayed()
        compose.onNodeWithContentDescription("Read full message large").performScrollTo().assertIsNotEnabled()
    }
    @Test fun messagePageShowsNextAndCloseWithoutAccumulatingTranscript() {
        val page = TranscriptPage("project", "session", "large", "assistant", "a".repeat(64), 0, 3, 6, true, "abc")
        var next = false; var closed = false
        compose.setContent { MaterialTheme { PairingScreen(PairingState(phase = PairingPhase.CONNECTED, projection = projection.copy(turns = listOf(RemoteTurn("large", "assistant", "prefix", true))),
            transcriptTurn = "large", transcriptPage = page), {}, {}, {}, onTranscript = { id, advance -> next = id == "large" && advance }, onCloseTranscript = { closed = true }) } }
        compose.onNodeWithTag("transcript-page").assertTextEquals("abc")
        compose.onNodeWithContentDescription("Next message page").performClick(); assertTrue(next)
        compose.onNodeWithContentDescription("Close message").performClick(); assertTrue(closed)
        assertTrue(projection.turns.isEmpty())
    }
    @Test fun terminalProjectionRequiresLiveMatchingSubtabAndNeverCachesScreen() {
        val tab = "a".repeat(64)
        val live = projection.copy(projects = projection.projects.map { it.copy(terminal = RemoteTerminal("project", "session", tab, "7", "PTY_VISIBLE", false)) },
            subtabs = RemoteSubtabs("project", "7", tab, listOf(RemoteSubtab(tab, "main", "main", 1)), false))
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = live))
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}) } }
        compose.onNodeWithTag("desktop-terminal").performScrollTo().assertTextEquals("PTY_VISIBLE")
        val encoded = ProjectionCacheCodec.encode(live)
        assertFalse(encoded.toString().contains("PTY_VISIBLE"))
        val cached = ProjectionCacheCodec.decode(encoded)
        assertNull(cached.projects.single().terminal); assertEquals(live.eventSequence, cached.eventSequence)
        compose.runOnUiThread { state.value = state.value.copy(projection = live.copy(subtabs = live.subtabs?.copy(revision = "8"))) }
        compose.onNodeWithTag("desktop-terminal").assertDoesNotExist()
        compose.runOnUiThread { state.value = PairingState(phase = PairingPhase.OFFLINE, projection = cached) }
        compose.onNodeWithTag("desktop-terminal").assertDoesNotExist()
    }
    @Test fun localDesktopPreviewIsReadOnlyTransientAndHiddenOffline() {
        val project = projection.projects.single().copy(promptControl = false, promptStatuses = emptyList(),
            desktopStreamText = "Local desktop partial")
        val live = projection.copy(projects = listOf(project))
        val state = mutableStateOf(PairingState(phase = PairingPhase.ERROR, projection = live))
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}) } }
        compose.onNodeWithTag("desktop-live-text").performScrollTo().assertTextEquals("Local desktop partial")
        compose.onNodeWithContentDescription("Cancel message $target").assertDoesNotExist()
        compose.onNodeWithContentDescription("Send steering input").assertDoesNotExist()
        val encoded = ProjectionCacheCodec.encode(live)
        assertFalse(encoded.toString().contains("Local desktop partial"))
        val cached = ProjectionCacheCodec.decode(encoded)
        assertNull(cached.projects.single().desktopStreamText)
        assertEquals(live.eventSequence, cached.eventSequence)
        compose.runOnUiThread { state.value = PairingState(phase = PairingPhase.OFFLINE, projection = cached) }
        compose.onNodeWithTag("desktop-live-text").assertDoesNotExist()
    }
    @Test fun partialTextIsDisplayedLiveButNeverInOfflineCursorCache() {
        val live = projection.copy(projects = projection.projects.map { project ->
            project.copy(promptStatuses = project.promptStatuses.map { it.copy(streamText = "Visible partial response", streamTruncated = true) })
        })
        val state = mutableStateOf(PairingState(phase = PairingPhase.ERROR, projection = live))
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}) } }
        compose.onNodeWithTag("live-prompt-$target").performScrollTo().assertTextEquals("Visible partial response")
        compose.onNodeWithTag("live-prompt-truncated-$target").performScrollTo().assertTextEquals("Preview truncated")
        val encoded = ProjectionCacheCodec.encode(live)
        assertFalse(encoded.toString().contains("Visible partial response"))
        val cached = ProjectionCacheCodec.decode(encoded)
        assertNull(cached.projects.single().promptStatuses.single().streamText)
        assertFalse(cached.projects.single().promptStatuses.single().streamTruncated)
        assertEquals(live.eventSequence, cached.eventSequence)
        compose.runOnUiThread { state.value = PairingState(phase = PairingPhase.OFFLINE, projection = cached) }
        compose.onNodeWithTag("live-prompt-$target").assertDoesNotExist()
        compose.onNodeWithTag("live-prompt-truncated-$target").assertDoesNotExist()
    }
    private val target = "12345678-request"
    private val projection = HostProjection(
        listOf(RemoteProject("project", "Workspace", "session", true, true,
            listOf(RemotePromptStatus(target, "project", "session", "running")))),
        "project", "session", "idle", emptyList(), "1", false, "0",
        workspaceOwner = "a".repeat(64), workspaceRevision = "1")

    @Test fun stopRoutesExactTargetAndIsFencedByPendingWritesOnNarrowScreen() {
        val state = mutableStateOf(PairingState(phase = PairingPhase.ERROR, projection = projection))
        val cancelled = mutableListOf<String>()
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onCancelPrompt = { cancelled.add(it) })
        } } }
        val stop = compose.onNodeWithContentDescription("Cancel message $target")
        stop.performScrollTo().assertIsDisplayed().assertIsEnabled()
        val label = compose.onNodeWithText("Message running").getUnclippedBoundsInRoot()
        val icon = stop.getUnclippedBoundsInRoot()
        val id = compose.onNodeWithText("12345678").getUnclippedBoundsInRoot()
        assertTrue(id.right <= label.left)
        assertTrue(label.right <= icon.left)
        stop.performClick()
        assertEquals(listOf(target), cancelled)
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val bitmap = instrumentation.uiAutomation.takeScreenshot()
        java.io.File(instrumentation.targetContext.filesDir, "prompt-stop-narrow.png").outputStream().use {
            bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
        }
        bitmap.recycle()
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.CONNECTING) }
        stop.assertIsNotEnabled()
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.ERROR,
            pendingWorkspace = WorkspaceCommand.cancelPrompt(projection, target)) }
        stop.assertIsNotEnabled()
        compose.runOnUiThread { state.value = state.value.copy(pendingWorkspace = null, pendingRevoke = "unresolved-revoke") }
        stop.assertIsNotEnabled()
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.OFFLINE, pendingRevoke = null) }
        stop.assertIsNotEnabled()
        assertEquals(listOf(target), cancelled)
    }

    @Test fun cachedOrTerminalStatusNeverOffersCancellation() {
        val state = mutableStateOf(PairingState(phase = PairingPhase.OFFLINE,
            projection = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(projection))))
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}) } }
        compose.onNodeWithText("Message result unknown").performScrollTo().assertIsDisplayed()
        compose.onNodeWithContentDescription("Cancel message $target").assertDoesNotExist()
        compose.runOnUiThread { state.value = PairingState(phase = PairingPhase.CONNECTED,
            projection = projection.copy(projects = projection.projects.map {
                it.copy(promptStatuses = it.promptStatuses.map { status -> status.copy(state = "cancelled") })
            })) }
        compose.onNodeWithText("Message cancelled").performScrollTo().assertIsDisplayed()
        compose.onNodeWithContentDescription("Cancel message $target").assertDoesNotExist()
    }
}
