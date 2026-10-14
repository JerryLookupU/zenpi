package zenpi

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.PairingViewModel
import zenpi.pairing.PairingPhase
import zenpi.security.DeviceIdentity
import zenpi.security.PairedHostStore

/** Uses emulator camera frames, not an injected scanner result or pairing URI. */
class EmulatorQrHostTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test fun terminalQrCameraFramesPairWithPinnedProductionHost(): Unit = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        assertEquals("true", args.getString("emulator_camera_run"))
        for (forbidden in listOf("invite", "token", "uri")) assertNull(args.getString(forbidden))
        val qemu = instrumentation.uiAutomation.executeShellCommand("getprop ro.kernel.qemu")
        android.os.ParcelFileDescriptor.AutoCloseInputStream(qemu).use {
            assertEquals("1", it.bufferedReader().readText().trim())
        }
        val address = requireNotNull(args.getString("expected_address"))
        val fingerprint = requireNotNull(args.getString("expected_fingerprint"))
        val project = requireNotNull(args.getString("expected_project"))
        val store = PairedHostStore(instrumentation.targetContext)
        assertNull("Existing pairings must not be overwritten", store.loadState())
        assertNull("Existing keys must not be replaced", DeviceIdentity.loadExisting())
        val model = androidx.lifecycle.ViewModelProvider(compose.activity)[PairingViewModel::class.java]
        fun phase(name: String) = instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", name) })
        val monitor = instrumentation.addMonitor(PairingCaptureActivity::class.java.name, null, false)
        try {
            compose.onNodeWithText("Scan QR").performScrollTo().performClick()
            val capture = requireNotNull(instrumentation.waitForMonitorWithTimeout(monitor, 15_000))
            instrumentation.runOnMainSync {
                val view = capture.findViewById<com.journeyapps.barcodescanner.DecoratedBarcodeView>(
                    com.google.zxing.client.android.R.id.zxing_barcode_scanner)
                assertTrue(view.decoderFactory is zenpi.pairing.PairingQrDecoderFactory)
            }
            assertTrue(instrumentation.uiAutomation.setRotation(android.app.UiAutomation.ROTATION_FREEZE_90))
            phase("emulator_qr_production_camera_requested")
            compose.waitUntil(60_000) { model.state.value.invite != null && !model.state.value.scanInProgress }
            compose.onNodeWithText("Confirm host").assertExists()
            compose.onNodeWithText(address).performScrollTo().assertIsDisplayed()
            compose.onNodeWithText(fingerprint.chunked(16).joinToString("\n")).assertExists()
            assertNull(store.loadState()); assertNull(DeviceIdentity.loadExisting())
            phase("emulator_qr_camera_confirmation_verified")
            compose.onNode(hasText("Confirm host") and hasClickAction()).performScrollTo().assertIsEnabled().performClick()
            compose.waitUntil(15_000) { model.state.value.registered && model.state.value.invite == null }
            val host = requireNotNull(store.loadState()).host
            assertEquals(fingerprint, host.fingerprint)
            assertEquals(host.deviceId, requireNotNull(DeviceIdentity.loadExisting()).deviceId)
            phase("paired_projection_verified")
            repeat(30) {
                if (model.state.value.projection?.selectedId != project) {
                    compose.runOnUiThread { model.refresh() }
                    delay(200)
                }
            }
            compose.waitUntil(20_000) {
                model.state.value.phase == PairingPhase.CONNECTED
                    && model.state.value.projection?.selectedId == project
                    && model.state.value.projection?.turns?.any {
                        it.role == "assistant" && it.content == "STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT"
                    } == true
            }
            compose.onNodeWithText("STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT", substring = true)
                .performScrollTo().assertIsDisplayed()
            phase("emulator_qr_authenticated_desktop_transcript_verified")
            if (args.getString("bidirectional") == "true") {
                val initial = requireNotNull(model.state.value.projection)
                val cursor = java.math.BigInteger(requireNotNull(initial.eventSequence))
                phase("desktop_update_requested")
                // The existing subscription must deliver the locally submitted desktop answer.
                compose.waitUntil(30_000) { model.state.value.projection?.turns?.any {
                    it.content == "STAGE2_PHYSICAL_DESKTOP_LOCAL_ANSWER" && it.role == "assistant"
                } == true }
                compose.onNodeWithText("STAGE2_PHYSICAL_DESKTOP_LOCAL_ANSWER", substring = true)
                    .performScrollTo().assertIsDisplayed()
                assertEquals(initial.sessionId, model.state.value.projection?.sessionId)
                assertTrue(java.math.BigInteger(requireNotNull(model.state.value.projection?.eventSequence)) > cursor)
                compose.waitUntil(15_000) { model.state.value.projection?.phase == "idle" }
                phase("emulator_qr_desktop_live_update_verified")
                compose.activityRule.scenario.moveToState(androidx.lifecycle.Lifecycle.State.CREATED)
                zenpi.lifecycle.ResumeScheduler.cancel(instrumentation.targetContext)
                assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
                assertNull(model.state.value.projection?.workspaceOwner)
                val cached = requireNotNull(store.loadState()).projection
                val backgroundCursor = requireNotNull(cached?.eventSequence)
                phase("background_update_requested")
                val client = zenpi.transport.HostClient(zenpi.lifecycle.LanSocketRouting(instrumentation.targetContext)::bind)
                val identity = requireNotNull(DeviceIdentity.loadExisting())
                var committed = false
                repeat(100) {
                    if (!committed) {
                        delay(100)
                        committed = client.inspect(host, identity = identity).turns.any {
                            it.role == "assistant" && it.content == "STAGE2_PHYSICAL_DESKTOP_BACKGROUND_ANSWER"
                        }
                    }
                }
                assertTrue("Desktop must complete while Android Activity is stopped", committed)
                assertEquals(cached, store.loadState()?.projection)
                assertEquals(backgroundCursor, model.state.value.projection?.eventSequence)
                phase("emulator_qr_background_cache_fenced_verified")
                compose.activityRule.scenario.moveToState(androidx.lifecycle.Lifecycle.State.RESUMED)
                compose.waitUntil(30_000) { model.state.value.phase == PairingPhase.CONNECTED
                    && model.state.value.projection?.turns?.any {
                        it.role == "assistant" && it.content == "STAGE2_PHYSICAL_DESKTOP_BACKGROUND_ANSWER"
                    } == true }
                compose.waitUntil(15_000) { model.state.value.projection?.phase == "idle" }
                val resumed = requireNotNull(model.state.value.projection)
                assertEquals(initial.sessionId, resumed.sessionId)
                assertEquals(1, resumed.turns.count { it.content == "STAGE2_PHYSICAL_DESKTOP_BACKGROUND_ANSWER" })
                assertTrue(java.math.BigInteger(requireNotNull(resumed.eventSequence)) > java.math.BigInteger(backgroundCursor))
                instrumentation.sendStatus(0, android.os.Bundle().apply {
                    putString("zenpi_background_event_cursor", backgroundCursor)
                    putString("zenpi_recovered_event_cursor", resumed.eventSequence)
                    putString("zenpi_replay_session_id", resumed.sessionId)
                })
                phase("emulator_qr_foreground_replay_verified")
                assertTrue(resumed.projects.single().promptControl)
                compose.onNodeWithTag("prompt-composer").performScrollTo()
                    .performTextReplacement("STAGE2_PHYSICAL_ANDROID_PROMPT")
                compose.onNodeWithContentDescription("Send message").performScrollTo().assertIsEnabled().performClick()
                compose.waitUntil(30_000) {
                    model.state.value.pendingWorkspace == null && model.state.value.projection?.turns?.any {
                        it.role == "assistant" && it.content == "STAGE2_PHYSICAL_DESKTOP_ANSWER"
                    } == true && model.state.value.projection?.projects?.singleOrNull()?.promptStatuses?.singleOrNull()?.state == "succeeded"
                }
                assertEquals(initial.sessionId, model.state.value.projection?.sessionId)
                compose.onNodeWithText("STAGE2_PHYSICAL_DESKTOP_ANSWER", substring = true)
                    .performScrollTo().assertIsDisplayed()
                phase("emulator_qr_android_prompt_answer_verified")
                if (args.getString("with_doze") == "true") {
                    fun shell(command: String): String = instrumentation.uiAutomation.executeShellCommand(command).use {
                        android.os.ParcelFileDescriptor.AutoCloseInputStream(it).bufferedReader().use { reader -> reader.readText().trim() }
                    }
                    val durable = requireNotNull(store.loadState())
                    try {
                        compose.activityRule.scenario.moveToState(androidx.lifecycle.Lifecycle.State.CREATED)
                        zenpi.lifecycle.ResumeScheduler.cancel(instrumentation.targetContext)
                        assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
                        assertNull(model.state.value.projection?.workspaceOwner)
                        assertTrue(requireNotNull(model.state.value.projection).projects.all { !it.promptControl && !it.workerControl })
                        shell("dumpsys battery unplug")
                        shell("input keyevent KEYCODE_SLEEP")
                        shell("dumpsys deviceidle force-idle deep")
                        assertEquals("IDLE", shell("dumpsys deviceidle get deep"))
                        phase("emulator_qr_deep_idle_verified")
                        delay(1500)
                        assertEquals(durable, store.loadState())
                        assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
                        assertNull(model.state.value.pendingWorkspace)
                        phase("emulator_qr_doze_cache_fenced_verified")
                    } finally {
                        shell("dumpsys deviceidle unforce"); shell("dumpsys battery reset")
                        shell("input keyevent KEYCODE_WAKEUP"); shell("wm dismiss-keyguard")
                    }
                    compose.activityRule.scenario.moveToState(androidx.lifecycle.Lifecycle.State.RESUMED)
                    compose.waitUntil(30_000) {
                        model.state.value.phase == PairingPhase.CONNECTED && model.state.value.projection?.turns == durable.projection?.turns
                    }
                    assertEquals(host, model.state.value.host)
                    assertNull(model.state.value.invite)
                    assertEquals(durable.projection?.sessionId, model.state.value.projection?.sessionId)
                    assertEquals(host.deviceId, requireNotNull(DeviceIdentity.loadExisting()).deviceId)
                    assertTrue(requireNotNull(model.state.value.projection).projects.single().promptControl)
                    compose.onNodeWithText("STAGE2_PHYSICAL_DESKTOP_ANSWER", substring = true)
                        .performScrollTo().assertIsDisplayed()
                    phase("emulator_qr_doze_foreground_resume_verified")
                }
            }
            compose.waitForIdle()
            instrumentation.uiAutomation.waitForIdle(500, 5_000)
            java.io.File(instrumentation.targetContext.filesDir, "emulator-qr-host.png").outputStream().use {
                assertTrue(instrumentation.uiAutomation.takeScreenshot().compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it))
            }
        } finally {
            instrumentation.removeMonitor(monitor)
            compose.runOnUiThread { model.forget() }
            compose.waitUntil(15_000) { !model.state.value.resetting }
            assertFalse(model.state.value.resetRequired)
            assertNull(store.loadState()); assertNull(DeviceIdentity.loadExisting())
            zenpi.lifecycle.ResumeScheduler.cancel(instrumentation.targetContext)
            phase("emulator_qr_local_cleanup_verified")
            instrumentation.uiAutomation.setRotation(android.app.UiAutomation.ROTATION_UNFREEZE)
        }
    }
}
