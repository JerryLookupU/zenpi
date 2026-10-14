package zenpi

import zenpi.ui.PairingScreen
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import zenpi.pairing.PairingInvite
import zenpi.transport.PairingClient
import zenpi.security.DeviceIdentity
import zenpi.security.PairedHostStore
import zenpi.security.PairedHost
import zenpi.transport.HostClient
import zenpi.lifecycle.LanSocketRouting
import kotlinx.coroutines.delay
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.collectAsState
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Rule
import zenpi.pairing.PairingState
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingViewModel

@RunWith(AndroidJUnit4::class)
class HostPairingTest {
    @get:Rule val compose = createComposeRule()
    private fun hostClient() = HostClient(LanSocketRouting(InstrumentationRegistry.getInstrumentation().targetContext)::bind)
    private fun pairingClient() = PairingClient(LanSocketRouting(InstrumentationRegistry.getInstrumentation().targetContext)::bind)
    @Test fun pinnedAndroidTlsConsumesProductionHostInviteOnce() = runBlocking {
        val uri = InstrumentationRegistry.getArguments().getString("invite")
            ?: error("Production host invite must be supplied by the validation harness")
        val invite = PairingInvite.parse(uri)
        val processNamespace = InstrumentationRegistry.getArguments().getString("process_namespace")
        val alias = processNamespace?.let { "zenpi.network-process.$it" } ?: "zenpi.test.${System.nanoTime()}"
        val identity = DeviceIdentity.loadOrCreate(alias)
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = processNamespace ?: "test-${System.nanoTime()}"
        val metadata = PairedHostStore(context, namespace)
        var retainForNextProcess = false
        val displayed = androidx.compose.runtime.mutableStateOf(PairingState())
        val displayedModel = androidx.compose.runtime.mutableStateOf<PairingViewModel?>(null)
        var selected: String? = null
        compose.setContent { MaterialTheme {
            val activeModel = displayedModel.value
            val current = activeModel?.state?.collectAsState()?.value ?: displayed.value
            PairingScreen(current, {}, {}, {}, onRefresh = { selected = it },
                onPrompt = { activeModel?.prompt(it) }, onRetryWorkspace = { activeModel?.retryWorkspace() }, onCancelPrompt = { activeModel?.cancelPrompt(it) },
                onSteer = { target, text -> activeModel?.steer(target, text) },
                onApprove = { target, decision -> activeModel?.approve(target, decision) },
                onOpenDesktopTerminal = { activeModel?.openDesktopTerminal() }, onTerminalInput = { activeModel?.terminalInput(it) })
        } }
        try {
        val keyStore = java.security.KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        assertNull(keyStore.getKey(alias, null).encoded)
        assertEquals(identity.deviceId, DeviceIdentity.loadOrCreate(alias).deviceId)
        assertTrue(identity.publicKey.length == 87)
        val wrongFingerprint = if (invite.fingerprint.startsWith('a')) "b".repeat(64) else "a".repeat(64)
        val wrong = PairingInvite.parse(uri.replace(invite.fingerprint, wrongFingerprint))
        assertTrue(runCatching { pairingClient().pair(wrong, "Android-emulator-test", identity) }.isFailure)
        val result = pairingClient().pair(invite, "Android-emulator-test", identity)
        assertTrue(result.paired); assertTrue(result.deviceRegistered)
        assertEquals(identity.deviceId, result.deviceId)
        val host = PairedHost(invite.host, invite.port, invite.fingerprint, result.deviceId)
        metadata.save(host); assertEquals(host, metadata.load())
        metadata.wipe(); assertNull(metadata.load())
        assertTrue(runCatching { pairingClient().pair(invite, "Android-emulator-test", identity) }.isFailure)
        val expectedProject = InstrumentationRegistry.getArguments().getString("expected_project")
        if (expectedProject != null) {
            val client = hostClient()
            val denied = client.inspect(host, identity = identity)
            val status = android.os.Bundle().apply { putString("zenpi_phase", "paired_projection_verified") }
            InstrumentationRegistry.getInstrumentation().sendStatus(0, status)
            var projection = denied
            repeat(30) {
                if (projection.projects.isEmpty()) {
                    delay(200)
                    try { projection = client.inspect(host, identity = identity) }
                    catch (error: zenpi.transport.WireException) {
                        // A host without a mounted project may still refuse while it is attached.
                        if (error.message != "Host request refused: unauthorized") throw error
                    }
                }
            }
            assertEquals(listOf(expectedProject), projection.projects.map { it.id })
            assertEquals(expectedProject, projection.selectedId)
            assertEquals(expectedProject, projection.subtabs?.projectId)
            assertEquals("main", projection.subtabs?.tabs?.first()?.kind)
            val expectedText = InstrumentationRegistry.getArguments().getString("expected_text") ?: error("Expected transcript required")
            assertTrue("Actual provider response must reach Android", projection.turns.any { it.role == "assistant" && it.content.contains(expectedText) })
            assertNotNull(projection.sessionId); assertNotNull(projection.journalSequence)
            val childProject = InstrumentationRegistry.getArguments().getString("workspace_child")
            if (childProject != null) {
                assertTrue(projection.projects.single().workspaceControl)
                val mainId = requireNotNull(projection.subtabs).activeId
                val rename = zenpi.transport.WorkspaceCommand.subtab(projection, "subtab_rename", mainId, "remote-main")
                val renamed = client.inspect(host, identity = identity, workspaceCommand = rename)
                assertEquals("remote-main", renamed.subtabs?.tabs?.single()?.name)
                assertEquals(mainId, renamed.subtabs?.activeId)
                val retried = client.inspect(host, identity = identity, workspaceCommand = rename)
                assertEquals(renamed.workspaceRevision, retried.workspaceRevision)
                assertEquals(renamed.subtabs, retried.subtabs)
                val reset = zenpi.transport.WorkspaceCommand.subtab(renamed, "subtab_rename", mainId, "main")
                projection = client.inspect(host, identity = identity, workspaceCommand = reset)
                InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "android_subtab_control_verified") })
                val command = zenpi.transport.WorkspaceCommand.create(projection, expectedProject, "workspace_open", "child")
                val opened = client.inspect(host, identity = identity, workspaceCommand = command)
                assertEquals(childProject, opened.selectedId)
                assertEquals(2, opened.projects.size)
                val duplicate = client.inspect(host, identity = identity, workspaceCommand = command)
                assertEquals(opened.workspaceRevision, duplicate.workspaceRevision)
                assertEquals(2, duplicate.projects.size)
                InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "workspace_open_verified") })
                delay(400)
                val stale = zenpi.transport.WorkspaceCommand.create(projection, expectedProject, "workspace_select")
                val conflict = runCatching { client.inspect(host, identity = identity, workspaceCommand = stale) }.exceptionOrNull()
                assertEquals("Host request refused: revision_conflict", conflict?.message)
                val close = zenpi.transport.WorkspaceCommand.create(opened, childProject, "workspace_close")
                projection = client.inspect(host, identity = identity, workspaceCommand = close)
                assertEquals(expectedProject, projection.selectedId)
                assertEquals(1, projection.projects.size)
                InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "workspace_close_verified") })
            }
            val reconnected = client.inspect(host, expectedProject, identity, previous = projection)
            assertEquals(projection, reconnected)
            val desktopUpdate = InstrumentationRegistry.getArguments().getString("desktop_update")
            if (desktopUpdate != null) {
                metadata.save(host)
                val application = InstrumentationRegistry.getInstrumentation().targetContext.applicationContext as android.app.Application
                val model = PairingViewModel(application, metadata, identity)
                try {
                    repeat(30) { if (model.state.value.host == null) delay(100) }
                    assertEquals(host, model.state.value.host)
                    compose.runOnUiThread { model.setForeground(true) }
                    repeat(40) { if (model.state.value.projection == null) delay(100) }
                    assertNotNull("Foreground must attach without manual refresh", model.state.value.projection)
                    val cachedSeed = requireNotNull(metadata.loadState()?.projection)
                    assertEquals(model.state.value.projection?.eventSequence, cachedSeed.eventSequence)
                    InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "desktop_update_requested") })
                    repeat(100) {
                        if (model.state.value.projection?.turns?.none { it.content.contains(desktopUpdate) } != false) {
                            delay(200)
                        }
                    }
                    var updated = requireNotNull(model.state.value.projection)
                    assertTrue("Foreground reducer must automatically observe desktop update", updated.turns.any { it.role == "assistant" && it.content.contains(desktopUpdate) })
                    assertEquals(reconnected.sessionId, updated.sessionId)
                    assertTrue(java.math.BigInteger(requireNotNull(updated.journalSequence)) > java.math.BigInteger(requireNotNull(reconnected.journalSequence)))
                    assertTrue(updated.turns.size > reconnected.turns.size)
                    assertTrue("Desktop update must arrive as replay events, not another snapshot", updated.replayedEventCount > 0)
                    assertTrue(java.math.BigInteger(requireNotNull(updated.eventSequence)) > java.math.BigInteger(requireNotNull(reconnected.eventSequence)))
                    if (childProject != null) {
                        assertTrue(updated.projects.single().promptControl)
                        compose.runOnUiThread { displayedModel.value = model }
                        assertTrue(updated.projects.single().terminalOpenControl)
                        compose.onNodeWithContentDescription("Desktop terminal").performScrollTo().performClick()
                        val initialTerminal = model.state.value.projection?.projects?.single()?.terminal
                        if (initialTerminal?.running == true) {
                            assertNotNull(initialTerminal.terminalId)
                            assertTrue(model.state.value.projection?.projects?.single()?.terminalControl == true)
                            // The desktop Execution pane may already own a PTY.
                            // Close it through the same authorized UI before testing Open.
                            compose.onNodeWithTag("terminal-input").performTextInput("exit")
                            compose.onNodeWithContentDescription("Send terminal input").performClick()
                            repeat(100) {
                                val current = model.state.value
                                if (current.phase != PairingPhase.CONNECTED || current.pendingWorkspace != null
                                    || current.projection?.projects?.single()?.terminal?.running != false) {
                                    delay(100)
                                    if (current.phase == PairingPhase.ERROR && current.pendingWorkspace != null)
                                        compose.runOnUiThread { model.retryWorkspace() }
                                }
                            }
                            assertEquals(PairingPhase.CONNECTED, model.state.value.phase)
                            assertNull(model.state.value.pendingWorkspace)
                            val exited = requireNotNull(model.state.value.projection?.projects?.single()?.terminal)
                            assertEquals(initialTerminal.terminalId, exited.terminalId)
                            assertFalse(exited.running)
                            compose.onNodeWithTag("terminal-input").assertDoesNotExist()
                        }
                        compose.onNodeWithContentDescription("Open desktop terminal").performClick()
                        repeat(100) {
                            val current = model.state.value
                            if (current.phase != PairingPhase.CONNECTED || current.pendingWorkspace != null || current.projection?.projects?.single()?.terminal == null) {
                                delay(100)
                                if (current.phase == PairingPhase.ERROR && current.pendingWorkspace != null)
                                    compose.runOnUiThread { model.retryWorkspace() }
                            }
                        }
                        assertNull(model.state.value.pendingWorkspace)
                        val terminal = requireNotNull(model.state.value.projection?.projects?.single()?.terminal)
                        assertNotNull(terminal.terminalId)
                        assertTrue(terminal.running)
                        initialTerminal?.let { assertNotEquals(it.terminalId, terminal.terminalId) }
                        compose.onNodeWithTag("terminal-input").performTextInput("printf '%s%s\\n' 'ANDROID_' 'TERMINAL_OWNED' | tee android-terminal-owned.txt")
                        compose.onNodeWithContentDescription("Send terminal input").performClick()
                        repeat(100) {
                            val current = model.state.value
                            if (current.phase != PairingPhase.CONNECTED || current.pendingWorkspace != null
                                || current.projection?.projects?.single()?.terminal?.text?.contains("ANDROID_TERMINAL_OWNED") != true) {
                                delay(100)
                                if (current.phase == PairingPhase.ERROR && current.pendingWorkspace != null)
                                    compose.runOnUiThread { model.retryWorkspace() }
                            }
                        }
                        assertNull(model.state.value.pendingWorkspace)
                        assertEquals(terminal.terminalId, model.state.value.projection?.projects?.single()?.terminal?.terminalId)
                        assertTrue(requireNotNull(model.state.value.projection?.projects?.single()?.terminal).text.contains("ANDROID_TERMINAL_OWNED"))
                        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "android_desktop_terminal_open_input_verified") })
                        compose.onNodeWithTag("terminal-input").performTextInput("exit")
                        compose.onNodeWithContentDescription("Send terminal input").performClick()
                        repeat(100) {
                            val current = model.state.value
                            if (current.pendingWorkspace != null || current.projection?.projects?.single()?.terminal?.running != false) {
                                delay(100)
                                if (current.phase == PairingPhase.ERROR && current.pendingWorkspace != null)
                                    compose.runOnUiThread { model.retryWorkspace() }
                            }
                        }
                        assertNull(model.state.value.pendingWorkspace)
                        assertFalse(requireNotNull(model.state.value.projection?.projects?.single()?.terminal).running)
                        compose.onNodeWithTag("terminal-input").assertDoesNotExist()
                        compose.onNodeWithContentDescription("Open desktop terminal").performClick()
                        repeat(100) {
                            val current = model.state.value
                            if (current.pendingWorkspace != null || current.projection?.projects?.single()?.terminal?.running != true) {
                                delay(100)
                                if (current.phase == PairingPhase.ERROR && current.pendingWorkspace != null)
                                    compose.runOnUiThread { model.retryWorkspace() }
                            }
                        }
                        val reopened = requireNotNull(model.state.value.projection?.projects?.single()?.terminal)
                        assertNull(model.state.value.pendingWorkspace); assertTrue(reopened.running)
                        assertNotEquals(terminal.terminalId, reopened.terminalId)
                        compose.onNodeWithTag("terminal-input").performTextInput("printf '%s%s\\n' 'ANDROID_' 'TERMINAL_REOPENED' | tee android-terminal-reopened.txt")
                        compose.onNodeWithContentDescription("Send terminal input").performClick()
                        repeat(100) {
                            val current = model.state.value
                            if (current.pendingWorkspace != null || current.projection?.projects?.single()?.terminal?.text?.contains("ANDROID_TERMINAL_REOPENED") != true) {
                                delay(100)
                                if (current.phase == PairingPhase.ERROR && current.pendingWorkspace != null)
                                    compose.runOnUiThread { model.retryWorkspace() }
                            }
                        }
                        assertNull(model.state.value.pendingWorkspace)
                        assertEquals(reopened.terminalId, model.state.value.projection?.projects?.single()?.terminal?.terminalId)
                        assertTrue(requireNotNull(model.state.value.projection?.projects?.single()?.terminal).text.contains("ANDROID_TERMINAL_REOPENED"))
                        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "android_desktop_terminal_exit_reopen_verified") })
                        compose.onNodeWithText("Message").performScrollTo().performTextInput("STAGE2_ANDROID_PROMPT_REQUEST")
                        compose.onNodeWithContentDescription("Send message").performScrollTo().performClick()
                        val remoteAnswer = "STAGE2_ANDROID_PROMPT_PROVIDER_ANSWER"
                        var sawRunning = false
                        repeat(50) {
                            if (model.state.value.phase == PairingPhase.CONNECTING || model.state.value.projection?.projects?.single()?.promptStatuses?.none { it.state == "running" } != false) delay(100)
                        }
                        val steeringTarget = requireNotNull(model.state.value.projection).projects.single().promptStatuses.single { it.state == "running" }
                        sawRunning = true
                        compose.onNodeWithText("Steer").performScrollTo().performTextReplacement("STAGE2_ANDROID_PROMPT_STEER")
                        compose.onNodeWithContentDescription("Send steering input").performScrollTo().performClick()
                        repeat(60) {
                            if (model.state.value.phase == PairingPhase.CONNECTING || model.state.value.pendingWorkspace != null) {
                                delay(100)
                                if (model.state.value.phase == PairingPhase.ERROR && model.state.value.pendingWorkspace != null)
                                    compose.runOnUiThread { model.retryWorkspace() }
                            }
                        }
                        assertNull("${model.state.value.error}; code=${model.state.value.workspaceFailureCode}", model.state.value.pendingWorkspace)
                        assertNull(model.state.value.pendingWorkspace)
                        assertNull(metadata.loadState()?.pendingWorkspace)
                        assertTrue(requireNotNull(model.state.value.projection).projects.single().promptStatuses.any { it.requestId == steeringTarget.requestId })
                        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "android_steer_durable_receipt_verified") })
                        repeat(100) {
                            if (model.state.value.projection?.projects?.any { project -> project.promptStatuses.any { it.state == "running" } } == true) sawRunning = true
                            if (model.state.value.projection?.turns?.none { it.content.contains(remoteAnswer) } != false) {
                                delay(200)
                                if (model.state.value.pendingWorkspace != null && model.state.value.phase == PairingPhase.ERROR)
                                    compose.runOnUiThread { model.retryWorkspace() }
                            }
                        }
                        updated = requireNotNull(model.state.value.projection)
                        assertNull(model.state.value.pendingWorkspace)
                        assertTrue(updated.turns.any { it.content.contains(remoteAnswer) })
                        assertEquals("succeeded", updated.projects.single().promptStatuses.single().state)
                        assertTrue("Busy owner must expose running status without taking the Agent lock", sawRunning)
                        compose.onNodeWithText("Message succeeded").performScrollTo().assertIsDisplayed()
                        compose.onNodeWithText("Message").performScrollTo().performTextReplacement("STAGE2_ANDROID_PROMPT_FAILURE")
                        compose.onNodeWithContentDescription("Send message").performScrollTo().performClick()
                        repeat(100) {
                            if (model.state.value.projection?.projects?.none { project -> project.promptStatuses.any { it.state == "failed" } } != false
                                || model.state.value.pendingWorkspace != null || model.state.value.phase != PairingPhase.CONNECTED) {
                                delay(200)
                                if (model.state.value.pendingWorkspace != null && model.state.value.phase == PairingPhase.ERROR)
                                    compose.runOnUiThread { model.retryWorkspace() }
                            }
                        }
                        updated = requireNotNull(model.state.value.projection)
                        assertNull(model.state.value.pendingWorkspace)
                        assertEquals(setOf("succeeded", "failed"), updated.projects.single().promptStatuses.map { it.state }.toSet())
                        compose.onNodeWithText("Message failed").performScrollTo().assertIsDisplayed()
                        compose.onNodeWithText("Message").performScrollTo().performTextReplacement("STAGE2_ANDROID_PROMPT_CANCEL")
                        compose.onNodeWithContentDescription("Send message").performScrollTo().performClick()
                        repeat(60) {
                            if (model.state.value.projection?.projects?.single()?.promptStatuses?.none { it.state == "running" } != false
                                || model.state.value.pendingWorkspace != null || model.state.value.phase == PairingPhase.CONNECTING) {
                                delay(100)
                                if (model.state.value.pendingWorkspace != null && model.state.value.phase == PairingPhase.ERROR)
                                    compose.runOnUiThread { model.retryWorkspace() }
                            }
                        }
                        val running = requireNotNull(model.state.value.projection).projects.single().promptStatuses.single { it.state == "running" }
                        assertNull(model.state.value.pendingWorkspace)
                        compose.onNodeWithContentDescription("Cancel message ${running.requestId}").performScrollTo().assertIsEnabled().performClick()
                        repeat(100) {
                            if (model.state.value.projection?.projects?.single()?.promptStatuses?.none { it.requestId == running.requestId && it.state == "cancelled" } != false
                                || model.state.value.pendingWorkspace != null || model.state.value.phase != PairingPhase.CONNECTED) {
                                delay(200)
                                if (model.state.value.pendingWorkspace != null && model.state.value.phase == PairingPhase.ERROR)
                                    compose.runOnUiThread { model.retryWorkspace() }
                            }
                        }
                        updated = requireNotNull(model.state.value.projection)
                        assertNull(model.state.value.pendingWorkspace)
                        assertEquals("cancelled", updated.projects.single().promptStatuses.single { it.requestId == running.requestId }.state)
                        compose.onNodeWithText("Message cancelled").performScrollTo().assertIsDisplayed()
                        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "android_prompt_cancel_verified") })
                        for (decision in listOf("allow", "deny")) {
                            InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "approval_${decision}_requested") })
                            repeat(100) {
                                val current = model.state.value
                                val selectedProject = current.projection?.projects?.singleOrNull()
                                if (selectedProject?.pendingApprovals?.isEmpty() != false
                                    || selectedProject.approvalControl.not() || current.pendingWorkspace != null
                                    || current.phase == PairingPhase.CONNECTING) {
                                    delay(100)
                                    if (current.phase == PairingPhase.ERROR && current.pendingWorkspace == null)
                                        compose.runOnUiThread { model.refresh() }
                                }
                            }
                            val current = requireNotNull(model.state.value.projection).projects.single()
                            assertTrue(current.approvalControl)
                            val target = current.pendingApprovals.single()
                            assertEquals("write_file", target.tool)
                            assertEquals("workspace_write", target.sideEffect)
                            val preview = requireNotNull(target.preview)
                            assertEquals("approval-$decision.txt", preview.path)
                            assertTrue(preview.patch.contains("+approved-$decision"))
                            assertFalse(preview.withheld)
                            compose.onNodeWithContentDescription("${if (decision == "allow") "Allow" else "Deny"} approval ${target.requestId}").assertIsEnabled()
                            compose.onNodeWithContentDescription("Review approvals").performClick()
                            compose.waitForIdle()
                            compose.onNodeWithText("Awaiting approval").assertIsDisplayed()
                            compose.onNodeWithText("approval-$decision.txt").assertIsDisplayed()
                            compose.onNodeWithText("+approved-$decision", substring = true).assertIsDisplayed()
                            delay(350)
                            val approvalBitmap = InstrumentationRegistry.getInstrumentation().uiAutomation.takeScreenshot()
                            java.io.File(context.filesDir, "live-approval-$decision.png").outputStream().use {
                                approvalBitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
                            }
                            approvalBitmap.recycle()
                            compose.onNodeWithText(if (decision == "allow") "Allow once" else "Deny").performClick()
                            compose.waitUntil(timeoutMillis = 3000) {
                                compose.onAllNodesWithTag("approval-sheet").fetchSemanticsNodes().isEmpty()
                            }
                            val answer = "STAGE2_APPROVAL_${decision.uppercase()}_DONE"
                            repeat(100) {
                                val state = model.state.value
                                if (state.projection?.turns?.none { it.content.contains(answer) } != false
                                    || state.pendingWorkspace != null || state.phase != PairingPhase.CONNECTED) {
                                    delay(100)
                                    if (state.pendingWorkspace != null && state.phase == PairingPhase.ERROR)
                                        compose.runOnUiThread { model.retryWorkspace() }
                                }
                            }
                            updated = requireNotNull(model.state.value.projection)
                            assertNull(model.state.value.pendingWorkspace)
                            assertTrue(updated.turns.any { it.content.contains(answer) })
                            assertTrue(updated.projects.single().pendingApprovals.isEmpty())
                        }
                        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "android_approval_preview_verified") })
                        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "approval_decisions_verified") })
                        repeat(40) {
                            val value = model.state.value
                            val tabs = value.projection?.subtabs
                            if (value.projection?.projects?.singleOrNull()?.promptControl != true || tabs?.control != true
                                || tabs.directoryControl.not() || value.phase != PairingPhase.CONNECTED) {
                                delay(100)
                                if (model.state.value.phase == PairingPhase.ERROR) compose.runOnUiThread { model.refresh() }
                            }
                        }
                        updated = requireNotNull(model.state.value.projection)
                        assertTrue(updated.projects.single().promptControl)
                        assertFalse(updated.projects.single().approvalControl)
                        assertTrue(requireNotNull(updated.subtabs).control)
                        assertEquals(reconnected.sessionId, updated.sessionId)
                        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "android_prompt_shared_runner_verified") })
                        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "android_prompt_terminal_status_verified") })
                    }
                    projection = updated
                    compose.runOnUiThread { model.setForeground(false) }
                    assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
                    val restored = PairingViewModel(application, PairedHostStore(context, namespace), identity)
                    try {
                        repeat(30) { if (restored.state.value.host == null) delay(100) }
                        assertEquals(host, restored.state.value.host)
                        val savedProjection = requireNotNull(restored.state.value.projection)
                        assertEquals(updated.turns.map { it.copy(toolCalls = emptyList(), toolCallsTruncated = false,
                            toolCallsOmitted = it.toolCalls.isNotEmpty() || it.toolCallsTruncated || it.toolCallsOmitted) }, savedProjection.turns)
                        assertEquals(zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(updated)), savedProjection)
                        assertEquals(updated.eventSequence, savedProjection.eventSequence)
                        assertEquals(updated.subtabs?.copy(control = false, directoryControl = false, worktreeControl = false,
                            concurrencyControl = false), savedProjection.subtabs)
                        assertFalse(requireNotNull(savedProjection.subtabs).control)
                        assertEquals(updated.projects.single().promptStatuses, savedProjection.projects.single().promptStatuses)
                        assertEquals(PairingPhase.OFFLINE, restored.state.value.phase)
                        compose.runOnUiThread { restored.setForeground(true) }
                        repeat(40) { if (restored.state.value.phase != PairingPhase.CONNECTED) delay(100) }
                        assertEquals(PairingPhase.CONNECTED, restored.state.value.phase)
                        assertEquals(savedProjection.eventSequence, restored.state.value.projection?.eventSequence)
                        assertEquals(updated.subtabs, restored.state.value.projection?.subtabs)
                    } finally { compose.runOnUiThread { restored.cancel() } }
                    val encrypted = java.io.File(context.noBackupFilesDir, "paired-host.$namespace")
                    val bytes = encrypted.readBytes()
                    assertFalse(bytes.toString(Charsets.ISO_8859_1).contains(desktopUpdate))
                    encrypted.writeBytes(bytes.copyOf().apply { this[lastIndex] = (this[lastIndex].toInt() xor 1).toByte() })
                    assertTrue("Tampered cache must not restore or advance cursor", runCatching { metadata.loadState() }.isFailure)
                    encrypted.writeBytes(bytes)
                    assertEquals(updated.eventSequence, metadata.loadState()?.projection?.eventSequence)
                    InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "encrypted_cache_restore_verified") })
                    InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "foreground_replay_verified") })
                } finally { compose.runOnUiThread { model.cancel() } }
                InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "shared_desktop_verified") })
            }
            val rendered = projection
            compose.runOnUiThread {
                displayedModel.value = null
                displayed.value = PairingState(phase = PairingPhase.CONNECTED, registered = true, host = host, projection = rendered)
            }
            compose.onNodeWithText(expectedText, substring = true).performScrollTo().assertIsDisplayed()
            if (desktopUpdate != null) compose.onNodeWithText(desktopUpdate, substring = true).performScrollTo().assertIsDisplayed()
            val instrumentation = InstrumentationRegistry.getInstrumentation()
            compose.waitForIdle()
            delay(350)
            val screenshot = instrumentation.uiAutomation.takeScreenshot()
            java.io.File(instrumentation.targetContext.filesDir, "host-projection.png").outputStream().use {
                screenshot.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
            }
            screenshot.recycle()
            compose.onNodeWithText("Refresh host").performScrollTo().performClick()
            assertEquals(expectedProject, selected)
            compose.onNodeWithText("Forget device locally").performScrollTo().assertIsDisplayed()
            if (processNamespace != null) {
                assertNotNull(metadata.loadState()?.projection?.eventSequence)
                instrumentation.sendStatus(0, android.os.Bundle().apply {
                    putString("zenpi_phase", "network_process_seed_verified")
                    putString("network_process_pid", android.os.Process.myPid().toString())
                })
                retainForNextProcess = true
                return@runBlocking
            }
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "snapshot_rendered_revoke_requested") })
            var refused = false
            repeat(30) {
                if (!refused) {
                    delay(200)
                    val failure = runCatching { client.inspect(host, expectedProject, identity) }.exceptionOrNull()
                    refused = failure is zenpi.transport.WireException && failure.message == "Host request refused: unauthorized"
                }
            }
            assertTrue("Host revocation must refuse saved device reconnect", refused)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "host_state_verified") })
        }
        } finally { if (!retainForNextProcess) { metadata.wipe(); assertNull(metadata.loadState()); identity.wipe() } }
    }
}
