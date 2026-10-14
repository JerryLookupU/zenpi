package zenpi

import zenpi.ui.PairingScreen
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.collectAsState
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.security.*
import zenpi.transport.*
import zenpi.lifecycle.LanSocketRouting

class HeadlessHostTest {
    @get:Rule val compose = createComposeRule()
    private fun hostClient() = HostClient(LanSocketRouting(InstrumentationRegistry.getInstrumentation().targetContext)::bind)
    private fun pairingClient() = PairingClient(LanSocketRouting(InstrumentationRegistry.getInstrumentation().targetContext)::bind)
    private fun saveScreenshot(name: String) {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        compose.waitForIdle()
        // Dialog window animation and accessibility publication are outside Compose's idle clock.
        instrumentation.uiAutomation.waitForIdle(500, 5_000)
        java.io.File(instrumentation.targetContext.filesDir, name).outputStream().use {
            assertTrue(instrumentation.uiAutomation.takeScreenshot().compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it))
        }
    }

    @Test fun androidComposerUsesHeadlessOwnerAndRetainsCommittedPromptUntilProcessDeath(): Unit = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        val namespace = requireNotNull(args.getString("process_namespace"))
        val identity = DeviceIdentity.loadOrCreate("zenpi.headless-process.$namespace")
        val store = PairedHostStore(instrumentation.targetContext, namespace)
        val invite = PairingInvite.parse(requireNotNull(args.getString("invite")))
        val paired = pairingClient().pair(invite, "Android-headless-test", identity)
        // ZE-514: the headless host mounts a project, so pairing grants control at once.
        assertTrue(paired.paired); assertTrue(paired.controlAvailable)
        val host = PairedHost(invite.host, invite.port, invite.fingerprint, paired.deviceId)
        store.save(host)
        var projection = hostClient().inspect(host, identity = identity)
        repeat(60) {
            if (projection.projects.isEmpty()) {
                delay(100)
                try { projection = hostClient().inspect(host, identity = identity) }
                catch (error: WireException) { if (error.message != "Host request refused: unauthorized") throw error }
            }
        }
        assertEquals(requireNotNull(args.getString("expected_project")), projection.selectedId)
        assertTrue(projection.projects.single().promptControl)
        store.saveProjection(host, projection)
        val model = PairingViewModel(instrumentation.targetContext.applicationContext as android.app.Application, store, identity)
        var retain = false
        try {
            compose.setContent { MaterialTheme {
                PairingScreen(model.state.collectAsState().value, {}, {}, {}, onPrompt = model::prompt, onRetryWorkspace = model::retryWorkspace,
                    onDiff = model::reviewDiff, onCloseDiff = model::closeDiff, onFiles = model::browseFiles, onCloseFiles = model::closeFiles,
                    onFilePage=model::browseFilePage,
                    onResources = model::browseResources, onCloseResources = model::closeResources,
                    onModel = {model.inspectModel()}, onCloseModel = model::closeModel, onModelPage=model::inspectModel,
                    onSelectModel = model::selectModel, onSelectReasoning = model::selectReasoning)
            } }
            compose.runOnUiThread { model.setForeground(true) }
            repeat(60) { if (model.state.value.phase != PairingPhase.CONNECTED) delay(100) }
            assertEquals(PairingPhase.CONNECTED, model.state.value.phase)
            assertTrue(requireNotNull(model.state.value.projection).projects.single().diffAvailable)
            compose.onNodeWithContentDescription("Review diff").performScrollTo().performClick()
            repeat(60) { if (model.state.value.diff == null && model.state.value.diffError == null) delay(100) }
            assertNull(model.state.value.diffError)
            assertTrue(requireNotNull(model.state.value.diff).diff.contains("HEADLESS_DIFF_AFTER"))
            compose.onNodeWithText("Path").performTextReplacement("review.txt")
            compose.onNodeWithContentDescription("Refresh diff").performClick()
            repeat(60) { if (model.state.value.diff?.path != "review.txt" && model.state.value.diffError == null) delay(100) }
            assertNull(model.state.value.diffError)
            assertEquals("review.txt", requireNotNull(model.state.value.diff).path)
            compose.onNodeWithTag("diff-patch").assertTextContains("HEADLESS_DIFF_AFTER", substring = true).assertIsDisplayed()
            saveScreenshot("headless-diff.png")
            compose.onNodeWithContentDescription("Close diff").performClick()
            assertFalse(model.state.value.diffOpen); assertNull(model.state.value.diff)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "headless_diff_read_verified") })
            compose.onNodeWithText("Message").performScrollTo().performTextInput("STAGE2_HEADLESS_ANDROID_FIRST")
            val beforeFiles = requireNotNull(model.state.value.projection)
            val cachedBeforeFiles = store.loadState()?.projection
            compose.onNodeWithContentDescription("Attach host file").performScrollTo().performClick()
            repeat(60) { if (model.state.value.files == null && model.state.value.filesError == null) delay(100) }
            assertNull(model.state.value.filesError)
            compose.onNodeWithText("documents").performScrollTo().performClick()
            repeat(60) { if (model.state.value.files?.path != "documents" && model.state.value.filesError == null) delay(100) }
            assertNull(model.state.value.filesError)
            assertEquals("documents", requireNotNull(model.state.value.files).path)
            val firstFiles=requireNotNull(model.state.value.files)
            assertEquals(0,firstFiles.page?.offset);assertEquals(64,firstFiles.entries.size);assertEquals(64,firstFiles.page?.nextOffset)
            assertFalse(firstFiles.entries.any{it.name=="fixture note.txt"})
            compose.onNodeWithContentDescription("Next files page").assertIsDisplayed().assertIsEnabled().performClick()
            repeat(100){if(model.state.value.files?.page?.offset!=64 && model.state.value.filesError==null)delay(100)}
            assertNull(model.state.value.filesError)
            val tailFiles=requireNotNull(model.state.value.files)
            assertEquals(64,tailFiles.page?.offset);assertEquals(firstFiles.page?.snapshotId,tailFiles.page?.snapshotId)
            assertNull(tailFiles.page?.nextOffset);assertFalse(tailFiles.truncated)
            val file = requireNotNull(model.state.value.files).entries.single { it.name == "fixture note.txt" }
            assertTrue(file.attachable); assertEquals("documents/fixture note.txt", file.path)
            compose.onNodeWithText("fixture note.txt").performScrollTo().assertIsDisplayed()
            saveScreenshot("headless-files.png")
            compose.onNodeWithText("fixture note.txt").performClick()
            assertFalse(model.state.value.filesOpen); assertNull(model.state.value.files)
            compose.onNodeWithTag("prompt-composer").assertTextContains("STAGE2_HEADLESS_ANDROID_FIRST @\"documents/fixture note.txt\"", substring = true)
            assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
            assertEquals(beforeFiles.turns, model.state.value.projection?.turns)
            assertEquals(beforeFiles.eventSequence, model.state.value.projection?.eventSequence)
            assertEquals(cachedBeforeFiles?.turns, store.loadState()?.projection?.turns)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "headless_file_picker_reference_verified") })
            compose.onNodeWithContentDescription("Send message").performScrollTo().performClick()
            suspend fun runningPreview(): HostProjection {
                repeat(150) {
                    val value = model.state.value
                    if (value.phase == PairingPhase.CONNECTED && value.pendingWorkspace == null
                        && value.projection?.projects?.singleOrNull()?.promptStatuses?.any {
                            it.state == "running" && it.streamText == "STAGE2_HEADLESS_ANDROID_FIRST_DONE"
                        } == true) return requireNotNull(value.projection)
                    delay(100)
                }
                error("Running provider preview did not reach the foreground subscriber")
            }
            val streaming = runningPreview()
            val running = streaming.projects.single().promptStatuses.single()
            assertFalse(streaming.turns.any { it.role == "assistant" && it.content.contains("STAGE2_HEADLESS_ANDROID_FIRST_DONE") })
            compose.onNodeWithTag("live-prompt-${running.requestId}").performScrollTo()
                .assertTextEquals("STAGE2_HEADLESS_ANDROID_FIRST_DONE").assertIsDisplayed()
            val cachedStreaming = requireNotNull(store.loadState()?.projection)
            assertEquals(streaming.eventSequence, cachedStreaming.eventSequence)
            assertEquals(streaming.journalSequence, cachedStreaming.journalSequence)
            assertEquals(streaming.turns, cachedStreaming.turns)
            val cachedStatus = cachedStreaming.projects.single().promptStatuses.single()
            assertEquals(running.requestId, cachedStatus.requestId)
            assertEquals("unknown_outcome", cachedStatus.state)
            assertNull(cachedStatus.streamText)
            saveScreenshot("headless-stream.png")
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "headless_running_stream_verified") })
            compose.runOnUiThread { model.setForeground(false) }
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            val offline = model.state.value
            val cacheFile = java.io.File(instrumentation.targetContext.noBackupFilesDir, "paired-host.$namespace")
            val offlineBytes = cacheFile.readBytes()
            compose.runOnUiThread {
                model.refresh()
                model.refresh(streaming.selectedId)
                model.reviewDiff("review.txt")
                model.browseFiles(); model.browseFiles("docs")
                model.browseResources(); model.browseResources(32)
                model.inspectModel()
                model.selectModel("BACKGROUND_MODEL_MUST_NOT_SEND")
                model.selectReasoning("low")
                streaming.turns.firstOrNull()?.let { model.readTranscript(it.id); model.readTranscript(it.id, true) }
                model.cancelPrompt(running.requestId)
                model.steer(running.requestId, "BACKGROUND_STEER_MUST_NOT_SEND")
                model.prompt("BACKGROUND_PROMPT_MUST_NOT_SEND")
                model.workspace("workspace_select", requireNotNull(streaming.selectedId))
                model.retryWorkspace()
                model.refresh(streaming.selectedId, WorkspaceCommand.cancelPrompt(streaming, running.requestId))
                model.revokeDevice()
            }
            assertEquals(offline, model.state.value)
            delay(500)
            assertEquals(offline, model.state.value)
            assertArrayEquals(offlineBytes, cacheFile.readBytes())
            assertEquals(cachedStreaming, model.state.value.projection)
            assertNull(model.state.value.projection?.workspaceOwner)
            assertNull(model.state.value.projection?.modelStatus)
            assertTrue(requireNotNull(model.state.value.projection).projects.all {
                !it.promptControl && !it.workerControl && it.promptStatuses.all { status ->
                    status.state == "unknown_outcome" && status.streamText == null
                }
            })
            assertEquals(cachedStreaming, store.loadState()?.projection)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "headless_background_writes_fenced_verified") })
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "headless_background_reads_fenced_verified") })
            compose.runOnUiThread { model.setForeground(true) }
            val reattached = runningPreview()
            assertEquals(streaming.sessionId, reattached.sessionId)
            assertEquals(streaming.eventSequence, reattached.eventSequence)
            assertEquals(streaming.journalSequence, reattached.journalSequence)
            assertEquals(running.requestId, reattached.projects.single().promptStatuses.single().requestId)
            assertFalse(reattached.turns.any { it.role == "assistant" && it.content.contains("STAGE2_HEADLESS_ANDROID_FIRST_DONE") })
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "headless_running_stream_resumed_verified") })
            repeat(100) {
                if (model.state.value.projection?.turns?.none { it.content.contains("STAGE2_HEADLESS_ANDROID_FIRST_DONE") } != false
                    || model.state.value.phase != PairingPhase.CONNECTED || model.state.value.pendingWorkspace != null
                    || model.state.value.projection?.projects?.singleOrNull()?.promptStatuses?.singleOrNull()?.state != "succeeded") delay(100)
            }
            val first = requireNotNull(model.state.value.projection)
            assertTrue(first.turns.any { it.role == "assistant" && it.content.contains("STAGE2_HEADLESS_ANDROID_FIRST_DONE") })
            assertEquals("succeeded", first.projects.single().promptStatuses.single().state)
            assertNull(first.projects.single().promptStatuses.single().streamText)
            assertNull(store.loadState()?.pendingWorkspace)
            // The production listener has four slots, including the foreground subscription.
            val concurrentReads = 3
            val parallel = coroutineScope {
                (0 until concurrentReads).map { async { hostClient().inspect(host, identity = identity, previous = first) } }.awaitAll()
            }
            assertEquals(concurrentReads, parallel.size)
            assertTrue(parallel.all { it.sessionId == first.sessionId && it.turns == first.turns })
            assertEquals(PairingPhase.CONNECTED, model.state.value.phase)
            assertEquals(first.sessionId, model.state.value.projection?.sessionId)
            instrumentation.sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", "headless_concurrent_reads_verified")
                putInt("headless_concurrent_read_count", concurrentReads)
            })
            compose.onNodeWithText("STAGE2_HEADLESS_ANDROID_FIRST_DONE", substring = true).performScrollTo().assertIsDisplayed()
            saveScreenshot("headless-host.png")
            suspend fun inspectSelection(): RemoteModelStatus {
                if (model.state.value.modelOpen) {
                    compose.onNodeWithContentDescription("Close host model").performClick()
                }
                compose.onNodeWithContentDescription("Inspect host model").performScrollTo().performClick()
                repeat(100) { if (model.state.value.modelLoading) delay(100) }
                assertNull(model.state.value.modelError)
                return requireNotNull(model.state.value.modelStatus)
            }
            val beforeSelection = requireNotNull(model.state.value.projection)
            val initialSelection = inspectSelection()
            assertEquals("gpt-5.2", initialSelection.model); assertEquals("high", initialSelection.reasoningEffort)
            compose.onNodeWithContentDescription("Select host model from catalog").performScrollTo().performClick()
            compose.onNodeWithText("gpt-5.2-2025-12-11").performClick()
            assertNull(model.state.value.pendingWorkspace);assertNull(store.loadState()?.pendingWorkspace)
            compose.onNodeWithText("Apply model").performScrollTo().performClick()
            repeat(100) { if (model.state.value.phase != PairingPhase.CONNECTED || model.state.value.pendingWorkspace != null) delay(100) }
            assertNull(model.state.value.error); assertNull(store.loadState()?.pendingWorkspace)
            val selectedModel = inspectSelection()
            assertEquals("gpt-5.2-2025-12-11", selectedModel.model); assertEquals("high", selectedModel.reasoningEffort)
            assertTrue(selectedModel.reasoningChoices.contains("low"))
            assertTrue(java.math.BigInteger(selectedModel.selectionRevision) > java.math.BigInteger(initialSelection.selectionRevision))
            compose.waitUntil(5_000) {
                compose.onAllNodes(hasContentDescription("Select reasoning level") and isEnabled())
                    .fetchSemanticsNodes().size == 1
            }
            compose.onNodeWithContentDescription("Select reasoning level").performScrollTo().assertIsEnabled().performClick()
            compose.onNodeWithText("low").performClick()
            compose.onNodeWithText("Apply reasoning").performScrollTo().performClick()
            repeat(100) { if (model.state.value.phase != PairingPhase.CONNECTED || model.state.value.pendingWorkspace != null) delay(100) }
            assertNull(model.state.value.error); assertNull(store.loadState()?.pendingWorkspace)
            val selectedEffort = inspectSelection()
            assertEquals(selectedModel.model, selectedEffort.model); assertEquals("low", selectedEffort.reasoningEffort)
            assertTrue(java.math.BigInteger(selectedEffort.selectionRevision) > java.math.BigInteger(selectedModel.selectionRevision))
            assertEquals(beforeSelection.turns, model.state.value.projection?.turns)
            assertEquals(beforeSelection.eventSequence, model.state.value.projection?.eventSequence)
            compose.onNodeWithText("Host model").assertIsDisplayed()
            compose.onNodeWithText("Reasoning: low").assertIsDisplayed()
            saveScreenshot("headless-selection.png")
            compose.onNodeWithContentDescription("Close host model").performClick()
            assertNull(model.state.value.modelStatus); assertFalse(model.state.value.modelOpen)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "headless_model_reasoning_selection_verified") })
            val resourceArguments = "STAGE2_HEADLESS_ANDROID_SECOND literal $1 $(touch never-created)"
            val beforeResources = requireNotNull(model.state.value.projection)
            val cachedBeforeResources = store.loadState()?.projection
            compose.onNodeWithTag("prompt-composer").performScrollTo().performTextReplacement(resourceArguments)
            compose.onNodeWithContentDescription("Select host resource").performScrollTo().performClick()
            repeat(100) { if (model.state.value.resources == null && model.state.value.resourcesError == null) delay(100) }
            assertNull(model.state.value.resourcesError)
            val firstPage = requireNotNull(model.state.value.resources)
            assertEquals(0, firstPage.offset); assertEquals(32, firstPage.entries.size); assertEquals(32, firstPage.nextOffset)
            assertTrue(firstPage.entries.any { it.command == "/skill:stage2-research" })
            assertTrue(firstPage.entries.filter { it.name.startsWith("fixture-") }.all { it.description.isEmpty() })
            assertFalse(firstPage.entries.any { it.command == "/zz-stage2-review" })
            compose.onNodeWithContentDescription("Next resource page").assertIsDisplayed().assertIsEnabled().performClick()
            repeat(100) { if (model.state.value.resources?.offset != 32 && model.state.value.resourcesError == null) delay(100) }
            assertNull(model.state.value.resourcesError)
            val lastPage = requireNotNull(model.state.value.resources)
            assertEquals(32, lastPage.offset); assertEquals(firstPage.snapshotId, lastPage.snapshotId)
            assertNull(lastPage.nextOffset); assertFalse(lastPage.truncated)
            assertTrue(lastPage.entries.filter { it.name.startsWith("fixture-") }.all { it.description.isEmpty() })
            compose.onNodeWithContentDescription("Next resource page").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Previous resource page").assertIsDisplayed().assertIsEnabled().performClick()
            repeat(100) { if (model.state.value.resources?.offset != 0 && model.state.value.resourcesError == null) delay(100) }
            assertNull(model.state.value.resourcesError)
            assertEquals(firstPage, model.state.value.resources)
            compose.onNodeWithContentDescription("Next resource page").assertIsDisplayed().assertIsEnabled().performClick()
            repeat(100) { if (model.state.value.resources?.offset != 32 && model.state.value.resourcesError == null) delay(100) }
            assertNull(model.state.value.resourcesError)
            compose.onNodeWithText("/zz-stage2-review").performScrollTo().assertIsDisplayed()
            saveScreenshot("headless-resources.png")
            compose.onNodeWithText("/zz-stage2-review").performClick()
            assertFalse(model.state.value.resourcesOpen); assertNull(model.state.value.resources)
            val resourcePrompt = compose.onNodeWithTag("prompt-composer").fetchSemanticsNode()
                .config[androidx.compose.ui.semantics.SemanticsProperties.EditableText].text
            assertEquals("/zz-stage2-review $resourceArguments", resourcePrompt)
            assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
            assertEquals(beforeResources.turns, model.state.value.projection?.turns)
            assertEquals(beforeResources.eventSequence, model.state.value.projection?.eventSequence)
            assertEquals(cachedBeforeResources?.turns, store.loadState()?.projection?.turns)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "headless_resource_pages_draft_verified") })
            compose.runOnUiThread { model.setForeground(false) }
            val live = hostClient().inspect(host, identity = identity, previous = first)
            val command = WorkspaceCommand.prompt(live, resourcePrompt)
            store.beginWorkspace(host, command)
            var confirmed = false
            val second = hostClient().inspect(host, identity = identity, previous = live, workspaceCommand = command,
                onWorkspaceConfirmed = { confirmed = true })
            assertTrue(confirmed)
            var completed = second
            repeat(100) {
                if (completed.turns.none { it.content.contains("STAGE2_HEADLESS_ANDROID_SECOND_DONE") }) {
                    delay(100)
                    try { completed = hostClient().inspect(host, identity = identity, previous = completed) }
                    catch (error: WireException) { if (error.message != "Host request refused: busy") throw error }
                }
            }
            assertTrue(completed.turns.any { it.content.contains("STAGE2_HEADLESS_ANDROID_SECOND_DONE") })
            // Host committed, but local encrypted pending/cursor deliberately retain the pre-result state.
            assertEquals(command.envelope, store.loadState()?.pendingWorkspace?.envelope)
            assertFalse(requireNotNull(store.loadState()?.projection).turns.any { it.content.contains("STAGE2_HEADLESS_ANDROID_SECOND_DONE") })
            retain = true
            instrumentation.sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", "headless_prompt_committed_before_death")
                putString("network_process_pid", android.os.Process.myPid().toString())
                putString("request_id", command.envelope["id"].asString)
            })
            delay(60_000)
            error("Harness did not kill the live process")
        } finally {
            compose.runOnUiThread { model.setForeground(false); model.cancel() }
            if (!retain) { store.wipe(); identity.wipe() }
        }
    }

    @Test fun newAndroidProcessResolvesOnlyTheSavedHeadlessPrompt() = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        val namespace = requireNotNull(args.getString("process_namespace"))
        assertNotEquals(requireNotNull(args.getString("previous_pid")).toInt(), android.os.Process.myPid())
        val store = PairedHostStore(instrumentation.targetContext, namespace)
        val identity = DeviceIdentity.loadOrCreate("zenpi.headless-process.$namespace")
        val saved = requireNotNull(store.loadState())
        val pending = requireNotNull(saved.pendingWorkspace)
        val seed = requireNotNull(saved.projection)
        assertEquals(requireNotNull(args.getString("request_id")), pending.envelope["id"].asString)
        assertEquals(saved.host.deviceId, identity.deviceId)
        val model = PairingViewModel(instrumentation.targetContext.applicationContext as android.app.Application, store, identity)
        try {
            repeat(60) { if (model.state.value.host == null) delay(100) }
            compose.setContent { MaterialTheme {
                PairingScreen(model.state.collectAsState().value, {}, {}, {}, onRetryWorkspace = model::retryWorkspace)
            } }
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            assertEquals(pending.envelope, model.state.value.pendingWorkspace?.envelope)
            compose.runOnUiThread { model.setForeground(true) }
            delay(400)
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            assertEquals(seed, model.state.value.projection)
            compose.onNodeWithText("Retry same request").performClick()
            repeat(100) {
                if (model.state.value.pendingWorkspace != null || model.state.value.phase != PairingPhase.CONNECTED
                    || model.state.value.projection?.turns?.none { it.content.contains("STAGE2_HEADLESS_ANDROID_SECOND_DONE") } != false) delay(100)
            }
            assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
            assertEquals(PairingPhase.CONNECTED, model.state.value.phase)
            val resumed = requireNotNull(model.state.value.projection)
            assertEquals(seed.sessionId, resumed.sessionId)
            assertTrue(resumed.replayedEventCount > 0)
            assertTrue(java.math.BigInteger(requireNotNull(resumed.eventSequence)) > java.math.BigInteger(requireNotNull(seed.eventSequence)))
            assertEquals(2, resumed.projects.single().promptStatuses.count { it.state == "succeeded" })
            assertTrue(resumed.turns.any { it.content.contains("STAGE2_HEADLESS_ANDROID_FIRST_DONE") })
            assertTrue(resumed.turns.any { it.content.contains("STAGE2_HEADLESS_ANDROID_SECOND_DONE") })
            val duplicate = hostClient().inspect(saved.host, identity = identity, previous = resumed, workspaceCommand = pending)
            assertEquals(resumed.workspaceRevision, duplicate.workspaceRevision)
            assertEquals(resumed.turns, duplicate.turns)
            val durable = requireNotNull(store.loadState()?.projection)
            val acknowledged = hostClient().inspect(saved.host, identity = identity, previous = durable, acknowledgePrevious = true)
            assertEquals(resumed.sessionId, acknowledged.sessionId)
            assertEquals(resumed.turns, acknowledged.turns)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "headless_durable_cursor_ack_verified") })
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "headless_prompt_process_recovery_verified") })
        } finally {
            compose.runOnUiThread { model.setForeground(false); model.cancel() }
            store.wipe(); identity.wipe()
        }
    }
}
