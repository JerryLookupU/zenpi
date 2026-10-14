package zenpi

import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.lifecycle.LanSocketRouting
import zenpi.security.*
import zenpi.transport.*
import zenpi.pairing.PairingViewModel
import zenpi.lifecycle.ResumeScheduler

/** The operator points the physical camera and enters the desktop SAS. No URI is injected. */
class PhysicalQrHostTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test fun cameraScanManualConfirmationAndAuthenticatedDesktopTranscript(): Unit = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        assertEquals("true", args.getString("physical_camera_run"))
        for (forbidden in listOf("invite", "token", "safety_code", "uri")) assertNull(args.getString(forbidden))
        val address = requireNotNull(args.getString("expected_address"))
        val fingerprint = requireNotNull(args.getString("expected_fingerprint"))
        val project = requireNotNull(args.getString("expected_project"))
        assertEquals("zenpi.PhysicalQrHostTest", args.getString("class"))
        val context = instrumentation.targetContext
        val store = PairedHostStore(context)
        // Never overwrite or wipe the operator's existing pairing/key.
        assertNull("A fresh dedicated test installation is required", store.loadState())
        assertNull("An existing device key must not be replaced", DeviceIdentity.loadExisting())
        var owned = false
        fun phase(name: String) = instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", name) })
        try {
            owned = true
            delay(1000)
            compose.onNodeWithText("Scan QR").performScrollTo().performClick()
            phase("physical_qr_camera_requested")
            compose.waitUntil(125_000) { compose.onAllNodesWithText("Desktop safety code").fetchSemanticsNodes().isNotEmpty() }
            compose.onNodeWithText(address).performScrollTo().assertIsDisplayed()
            compose.onNodeWithText(fingerprint.chunked(16).joinToString("\n")).assertExists()
            assertNull(store.loadState()); assertNull(DeviceIdentity.loadExisting())
            phase("physical_qr_confirmation_visible")
            // No performTextInput, confirm click or ViewModel injection here.
            // The operator must compare and enter the SAS, then tap Confirm host.
            var saved: SavedHostState? = null
            val registrationDeadline = System.nanoTime() + 90_000_000_000L
            while (saved == null && System.nanoTime() < registrationDeadline) { delay(100); saved = store.loadState() }
            val host = requireNotNull(saved).host
            assertEquals(address, if (host.host.contains(':')) "[${host.host}]:${host.port}" else "${host.host}:${host.port}")
            assertEquals(fingerprint, host.fingerprint)
            val identity = requireNotNull(DeviceIdentity.loadExisting())
            assertEquals(identity.deviceId, host.deviceId)
            val model = androidx.lifecycle.ViewModelProvider(compose.activity)[PairingViewModel::class.java]
            compose.waitUntil(15_000) { model.state.value.host == host }
            assertTrue(model.state.value.registered)
            assertNull("A successful registration must retire its one-use invitation", model.state.value.invite)
            phase("physical_qr_invitation_retired_verified")
            val client = HostClient(LanSocketRouting(context)::bind)
            assertTrue(client.inspect(host, identity = identity).projects.isEmpty())
            phase("ungranted_projection_verified")
            var projection: HostProjection? = null
            repeat(200) {
                if (projection?.selectedId != project) {
                    delay(100)
                    try { projection = client.inspect(host, identity = identity) }
                    catch (error: HostRefusedException) { if (error.code != "unauthorized") throw error }
                }
            }
            val current = requireNotNull(projection)
            assertEquals(project, current.selectedId)
            assertTrue(current.turns.any { it.role == "assistant" && it.content == "STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT" })
            compose.onNodeWithText("Refresh host").performScrollTo().performClick()
            compose.waitUntil(30_000) { compose.onAllNodesWithText("STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT", substring = true).fetchSemanticsNodes().isNotEmpty() }
            compose.onNodeWithText("STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT", substring = true).performScrollTo().assertIsDisplayed()
            phase("physical_qr_desktop_transcript_verified")
            val beforeRecreation = compose.activity
            val recreationCursor = java.math.BigInteger(requireNotNull(model.state.value.projection?.eventSequence))
            compose.activityRule.scenario.recreate()
            compose.activityRule.scenario.onActivity { recreated ->
                assertNotSame(beforeRecreation, recreated)
                assertSame(model, androidx.lifecycle.ViewModelProvider(recreated)[PairingViewModel::class.java])
                assertNull(recreated.intent.data)
            }
            // Lifecycle reconnect must use the saved pin/key, not another QR or refresh click.
            compose.waitUntil(30_000) { model.state.value.phase == zenpi.pairing.PairingPhase.CONNECTED
                && model.state.value.projection?.turns?.any {
                    it.role == "assistant" && it.content == "STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT"
                } == true }
            assertEquals(host, model.state.value.host)
            assertTrue(model.state.value.registered); assertNull(model.state.value.invite)
            assertFalse(model.state.value.scanInProgress)
            assertEquals(identity.deviceId, requireNotNull(DeviceIdentity.loadExisting()).deviceId)
            assertEquals(host, store.loadState()?.host)
            assertEquals(current.sessionId, model.state.value.projection?.sessionId)
            assertTrue(java.math.BigInteger(requireNotNull(model.state.value.projection?.eventSequence)) >= recreationCursor)
            compose.onNodeWithText("STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT", substring = true).performScrollTo().assertIsDisplayed()
            phase("physical_qr_activity_recreation_resume_verified")
            val priorCursor = java.math.BigInteger(requireNotNull(model.state.value.projection?.eventSequence))
            phase("desktop_update_requested")
            // No explicit refresh: the foreground subscription must deliver a
            // locally submitted desktop turn to the already connected phone.
            compose.waitUntil(30_000) { model.state.value.projection?.turns?.any {
                it.role == "assistant" && it.content == "STAGE2_PHYSICAL_DESKTOP_LOCAL_ANSWER"
            } == true }
            compose.onNodeWithText("STAGE2_PHYSICAL_DESKTOP_LOCAL_ANSWER", substring = true).performScrollTo().assertIsDisplayed()
            assertEquals(current.sessionId, model.state.value.projection?.sessionId)
            assertTrue(java.math.BigInteger(requireNotNull(model.state.value.projection?.eventSequence)) > priorCursor)
            val localSaved = requireNotNull(store.loadState()).projection
            assertEquals(current.sessionId, localSaved?.sessionId)
            assertTrue(requireNotNull(localSaved).turns.any { it.role == "assistant" && it.content == "STAGE2_PHYSICAL_DESKTOP_LOCAL_ANSWER" })
            compose.waitUntil(15_000) { model.state.value.projection?.phase == "idle" }
            phase("physical_qr_desktop_live_update_verified")
            compose.activityRule.scenario.moveToState(androidx.lifecycle.Lifecycle.State.CREATED)
            // This fixture isolates foreground replay from opportunity-based WorkManager reads.
            ResumeScheduler.cancel(context)
            assertEquals(zenpi.pairing.PairingPhase.OFFLINE, model.state.value.phase)
            assertNull(model.state.value.projection?.workspaceOwner)
            val beforeBackground = requireNotNull(store.loadState()).projection
            val backgroundCursor = requireNotNull(beforeBackground?.eventSequence)
            phase("background_update_requested")
            val backgroundDeadline = System.nanoTime() + 20_000_000_000L
            var backgroundCommitted = false
            while (!backgroundCommitted && System.nanoTime() < backgroundDeadline) {
                delay(200)
                try {
                    backgroundCommitted = client.inspect(host, identity = identity).turns.any {
                        it.role == "assistant" && it.content == "STAGE2_PHYSICAL_DESKTOP_BACKGROUND_ANSWER"
                    }
                } catch (error: HostRefusedException) { if (error.code != "busy") throw error }
            }
            assertTrue("Desktop must commit while the phone Activity is stopped", backgroundCommitted)
            assertEquals(beforeBackground, store.loadState()?.projection)
            assertEquals(backgroundCursor, model.state.value.projection?.eventSequence)
            assertFalse(requireNotNull(model.state.value.projection).turns.any {
                it.content == "STAGE2_PHYSICAL_DESKTOP_BACKGROUND_ANSWER"
            })
            phase("physical_qr_background_cache_fenced_verified")
            compose.activityRule.scenario.moveToState(androidx.lifecycle.Lifecycle.State.RESUMED)
            // No injected invitation/cursor and no Refresh click: onStart must reauthenticate and replay.
            compose.waitUntil(30_000) { model.state.value.phase == zenpi.pairing.PairingPhase.CONNECTED
                && model.state.value.projection?.turns?.any {
                    it.role == "assistant" && it.content == "STAGE2_PHYSICAL_DESKTOP_BACKGROUND_ANSWER"
                } == true }
            compose.onNodeWithText("STAGE2_PHYSICAL_DESKTOP_BACKGROUND_ANSWER", substring = true).performScrollTo().assertIsDisplayed()
            compose.waitUntil(15_000) { model.state.value.projection?.phase == "idle" }
            val recovered = requireNotNull(model.state.value.projection)
            assertEquals(current.sessionId, recovered.sessionId)
            assertTrue(java.math.BigInteger(requireNotNull(recovered.eventSequence)) > java.math.BigInteger(backgroundCursor))
            assertEquals(zenpi.state.ProjectionCacheCodec.encode(recovered),
                zenpi.state.ProjectionCacheCodec.encode(requireNotNull(store.loadState()?.projection)))
            assertEquals(1, recovered.turns.count { it.content == "STAGE2_PHYSICAL_DESKTOP_BACKGROUND_ANSWER" })
            instrumentation.sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_background_event_cursor", backgroundCursor)
                putString("zenpi_recovered_event_cursor", recovered.eventSequence)
                putString("zenpi_replay_session_id", recovered.sessionId)
            })
            phase("physical_qr_foreground_replay_verified")
            val controlDeadline = System.nanoTime() + 30_000_000_000L
            while (System.nanoTime() < controlDeadline && model.state.value.projection?.projects?.singleOrNull()?.promptControl != true) {
                compose.runOnUiThread { model.refresh() }
                delay(500)
            }
            assertEquals(project, model.state.value.projection?.selectedId)
            assertTrue(requireNotNull(model.state.value.projection?.projects?.singleOrNull()).promptControl)
            compose.onNodeWithText("Message").performScrollTo().performTextReplacement("STAGE2_PHYSICAL_ANDROID_PROMPT")
            compose.onNodeWithContentDescription("Send message").performScrollTo().assertIsEnabled().performClick()
            compose.waitUntil(30_000) {
                model.state.value.pendingWorkspace == null && model.state.value.projection?.turns?.any {
                    it.role == "assistant" && it.content == "STAGE2_PHYSICAL_DESKTOP_ANSWER"
                } == true
            }
            compose.onNodeWithText("STAGE2_PHYSICAL_DESKTOP_ANSWER", substring = true).performScrollTo().assertIsDisplayed()
            assertEquals(current.sessionId, model.state.value.projection?.sessionId)
            phase("physical_qr_android_prompt_answer_verified")
            compose.waitUntil(15_000) { model.state.value.phase == zenpi.pairing.PairingPhase.CONNECTED
                && model.state.value.pendingWorkspace == null }
            val observer = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
            try {
                lateinit var request: Deferred<String>
                compose.runOnUiThread {
                    request = observer.async(start = CoroutineStart.UNDISPATCHED) {
                        requireNotNull(model.state.first { it.pendingRevoke != null }.pendingRevoke)
                    }
                }
                compose.onNodeWithText("Revoke device on host").performScrollTo().assertIsEnabled().performClick()
                compose.onNodeWithText("Revoke", useUnmergedTree = true).assertIsEnabled().performClick()
                val requestId = withTimeout(15_000) { request.await() }
                compose.waitUntil(30_000) { model.state.value.host == null
                    && model.state.value.error == "Device revoked on host" }
                assertNull(store.loadState()); assertNull(DeviceIdentity.loadExisting())
                assertNull(model.state.value.pendingRevoke); assertNull(model.state.value.projection)
                assertFalse(model.state.value.registered)
                compose.onNodeWithText("Device revoked on host").assertExists()
                instrumentation.sendStatus(0, android.os.Bundle().apply {
                    putString("zenpi_physical_revoke_request_id", requestId)
                })
                phase("physical_qr_self_revoke_verified")
            } finally { observer.cancel() }
        } finally {
            if (owned) {
                val model = androidx.lifecycle.ViewModelProvider(compose.activity)[PairingViewModel::class.java]
                compose.runOnUiThread { model.forget() }
                val cleanupDeadline = System.nanoTime() + 15_000_000_000L
                while (model.state.value.resetting && System.nanoTime() < cleanupDeadline) delay(100)
                assertFalse("Production local reset must finish", model.state.value.resetting)
                assertFalse("Failed reset must not publish successful cleanup", model.state.value.resetRequired)
                assertNull(model.state.value.host); assertNull(store.loadState()); assertNull(DeviceIdentity.loadExisting())
                ResumeScheduler.cancel(context)
                phase("physical_qr_local_cleanup_verified")
            }
        }
    }
}
