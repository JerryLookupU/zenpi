package zenpi

import androidx.compose.material3.MaterialTheme
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import java.util.Base64

class PairingScreenTest {
    @get:Rule val compose = createComposeRule()
    private fun invite(): PairingInvite {
        val token = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32))
        return PairingInvite.parse("zenpi://pair?v=1&address=192.168.50.55%3A7777&fingerprint=${"a".repeat(64)}&token=$token&expires=${System.currentTimeMillis()/1000 + 120}")
    }
    @Test fun pairedDeviceIdentityIsReadableAndCopiesExactlyOfflineOnNarrowScreen() {
        val id="0123456789abcdef".repeat(4)
        val host=zenpi.security.PairedHost("fd00::1234",7777,"a".repeat(64),id)
        var state by androidx.compose.runtime.mutableStateOf(PairingState(phase=PairingPhase.OFFLINE,registered=true,host=host))
        val instrumentation=androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
        val manager=instrumentation.targetContext.getSystemService(android.content.ClipboardManager::class.java)
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state,{},{},{})
        } } }
        compose.onNodeWithText("[fd00::1234]:7777").assertIsDisplayed()
        compose.onNodeWithTag("paired-device-id").performScrollTo().assertIsDisplayed()
            .assertTextEquals(id.chunked(16).joinToString("\n"))
        compose.onNodeWithContentDescription("Copy device ID").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(id,manager.primaryClip?.getItemAt(0)?.text?.toString()) }
        compose.runOnIdle { state=state.copy(resetting=true) }
        compose.onNodeWithContentDescription("Copy device ID").assertIsNotEnabled()
        compose.runOnIdle { state=PairingState() }
        compose.onNodeWithTag("paired-device-id").assertDoesNotExist()
        compose.onNodeWithContentDescription("Copy device ID").assertDoesNotExist()
    }
    @Test fun safetyCodeRequiresHumanEntryAndCancelIsAccessible() {
        val invite = invite(); var confirmed: String? = null; var cancelled = false
        var state by androidx.compose.runtime.mutableStateOf(PairingState(PairingPhase.CONFIRMING, invite))
        compose.setContent { MaterialTheme { PairingScreen(state, {}, { confirmed = it }, { cancelled = true }) } }
        compose.onAllNodesWithText("Confirm host").filter(hasClickAction()).onFirst().assertIsNotEnabled()
        compose.onNodeWithText("Desktop safety code").performTextInput(invite.safetyCode)
        compose.onNodeWithText("Desktop safety code").performImeAction()
        compose.waitForIdle()
        compose.onAllNodesWithText("Confirm host").filter(hasClickAction()).onFirst().performScrollTo().assertIsEnabled().performClick()
        compose.waitUntil(timeoutMillis = 3000) { confirmed != null }
        assertEquals(invite.safetyCode, confirmed)
        val screenshot = androidx.test.platform.app.InstrumentationRegistry.getInstrumentation().uiAutomation.takeScreenshot()
        val file = java.io.File(androidx.test.platform.app.InstrumentationRegistry.getInstrumentation().targetContext.filesDir, "pairing-confirmation.png")
        file.outputStream().use { screenshot.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it) }
        screenshot.recycle()
        val token = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32) { 1 })
        val replacement = PairingInvite.parse("zenpi://pair?v=1&address=192.168.50.55%3A7777&fingerprint=${invite.fingerprint}&token=$token&expires=${invite.expiresAt}")
        compose.runOnIdle { state = PairingState(PairingPhase.CONFIRMING, replacement) }
        compose.onNodeWithText("Desktop safety code").assert(SemanticsMatcher.expectValue(
            androidx.compose.ui.semantics.SemanticsProperties.EditableText, androidx.compose.ui.text.AnnotatedString("")))
        compose.onAllNodesWithText("Confirm host").filter(hasClickAction()).onFirst().assertIsNotEnabled()
        compose.onNodeWithText("Desktop safety code").performTextInput(replacement.safetyCode)
        compose.onAllNodesWithText("Confirm host").filter(hasClickAction()).onFirst().assertIsEnabled()
        for (bad in listOf(state.copy(resetting = true), state.copy(resetRequired = true), state.copy(pendingRevoke = "pending"))) {
            compose.runOnIdle { state = bad }
            compose.onNodeWithText("Desktop safety code").assertIsNotEnabled()
            compose.onAllNodesWithText("Confirm host").filter(hasClickAction()).onFirst().assertIsNotEnabled()
        }
        val expired = PairingInvite.parse("zenpi://pair?v=1&address=192.168.50.55%3A7777&fingerprint=${invite.fingerprint}&token=$token&expires=1060", 1000)
        compose.runOnIdle { state = PairingState(PairingPhase.CONFIRMING, expired) }
        compose.onNodeWithText("Desktop safety code").assertIsNotEnabled()
        compose.onAllNodesWithText("Confirm host").filter(hasClickAction()).onFirst().assertIsNotEnabled()
        var elapsed = 0L
        val wall = System.currentTimeMillis() / 1000
        val monotonicInvite = PairingInvite.parse("zenpi://pair?v=1&address=192.168.50.55%3A7777&fingerprint=${invite.fingerprint}&token=$token&expires=${wall + 120}", wall) { elapsed }
        compose.runOnIdle { state = PairingState(PairingPhase.CONFIRMING, monotonicInvite) }
        compose.onNodeWithText("Desktop safety code").performTextInput(monotonicInvite.safetyCode)
        compose.onAllNodesWithText("Confirm host").filter(hasClickAction()).onFirst().assertIsEnabled()
        compose.runOnIdle { elapsed = 120_000_000_000; state = state.copy(scanError = "Clock fixture changed") }
        compose.onNodeWithText("Desktop safety code").assertIsNotEnabled()
        compose.onAllNodesWithText("Confirm host").filter(hasClickAction()).onFirst().assertIsNotEnabled()
        compose.runOnIdle { elapsed = 0; state = state.copy(scanError = null) }
        compose.onAllNodesWithText("Confirm host").filter(hasClickAction()).onFirst().assertIsNotEnabled()
        androidx.test.platform.app.InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
            putString("zenpi_invite_monotonic_ui", "expired_confirmation_never_revives_verified")
        })
        compose.runOnIdle { state = PairingState(PairingPhase.CONFIRMING, replacement) }
        compose.onNodeWithText("Cancel").performScrollTo().performClick()
        compose.waitUntil(timeoutMillis = 3000) { cancelled }; assertTrue(cancelled)
    }
    @Test fun registeredDeviceDoesNotClaimHostControl() {
        val host = zenpi.security.PairedHost("192.168.50.55", 7777, "a".repeat(64), "b".repeat(64))
        var state by androidx.compose.runtime.mutableStateOf(PairingState(phase = PairingPhase.ACCEPTED, registered = true, host = host))
        compose.setContent { MaterialTheme { PairingScreen(state, {}, {}, {}) } }
        compose.onNodeWithText("Invitation accepted").assertExists()
        compose.onNodeWithText("192.168.50.55:7777").assertExists()
        compose.onNodeWithText("Device registered").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Host control unavailable").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Desktop safety code").assertDoesNotExist()
        compose.onNodeWithText("Host SHA-256").assertDoesNotExist()
        compose.runOnIdle { state = state.copy(phase = PairingPhase.CONNECTED) }
        compose.onNodeWithText("Device registered").assertDoesNotExist()
        compose.onNodeWithText("Host control unavailable").assertDoesNotExist()
        compose.onNodeWithText("192.168.50.55:7777").assertExists()
        compose.runOnIdle { state = state.copy(phase = PairingPhase.ACCEPTED) }
        val instrumentation = androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
        val screenshot = instrumentation.uiAutomation.takeScreenshot()
        java.io.File(instrumentation.targetContext.filesDir, "pairing-accepted.png").outputStream().use {
            screenshot.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
        }
        screenshot.recycle()
        instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "registered_host_without_invite_verified") })
    }
    @Test fun selfRevocationRequiresConfirmationSeparateFromLocalForget() {
        var revoked = false; var forgotten = false
        val host = zenpi.security.PairedHost("127.0.0.1", 7777, "a".repeat(64), "b".repeat(64))
        compose.setContent { MaterialTheme { PairingScreen(PairingState(phase = PairingPhase.CONNECTED, registered = true, host = host),
            {}, {}, {}, onForget = { forgotten = true }, onRevoke = { revoked = true }) } }
        compose.onNodeWithText("Revoke device on host").performScrollTo().performClick()
        assertFalse(revoked); assertFalse(forgotten)
        compose.onNodeWithText("Revoke this device?").assertExists()
        compose.onAllNodesWithText("Cancel").filter(hasClickAction()).onLast().performClick()
        assertFalse(revoked)
        compose.onNodeWithText("Revoke device on host").performClick()
        compose.onNodeWithText("Revoke").performClick()
        assertTrue(revoked); assertFalse(forgotten)
        compose.onNodeWithText("Forget device locally").performScrollTo().performClick()
        assertFalse(forgotten)
        compose.onNodeWithText("Remove local device data?").assertIsDisplayed()
        compose.onNodeWithText("Remove local data").performClick()
        assertTrue(forgotten)
    }
    @Test fun deviceConfirmationCannotSurviveHostReplacementOrAuthorizeDuringScan() {
        val host = zenpi.security.PairedHost("192.168.50.55", 7777, "a".repeat(64), "b".repeat(64))
        var state by androidx.compose.runtime.mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, registered = true, host = host))
        var revoked = 0; var forgotten = 0
        compose.setContent { MaterialTheme { PairingScreen(state, {}, {}, {},
            onRevoke = { revoked++ }, onForget = { forgotten++ }) } }
        compose.onNodeWithText("Revoke device on host").performScrollTo().performClick()
        compose.onNodeWithText("Revoke").assertIsEnabled()
        compose.runOnIdle { state = state.copy(scanInProgress = true) }
        compose.onNodeWithText("Revoke").assertIsNotEnabled()
        compose.runOnIdle { state = state.copy(scanInProgress = false, host = host.copy(port = 8888)) }
        compose.onNodeWithText("Revoke this device?").assertDoesNotExist()
        compose.runOnIdle { assertEquals(0, revoked); assertEquals(0, forgotten) }
        compose.onNodeWithText("Forget device locally").performScrollTo().performClick()
        compose.onNodeWithText("Remove local data").assertIsEnabled()
        compose.runOnIdle { state = state.copy(scanInProgress = true) }
        compose.onNodeWithText("Remove local data").assertIsNotEnabled()
        compose.runOnIdle { state = state.copy(scanInProgress = false, host = host.copy(fingerprint = "c".repeat(64))) }
        compose.onNodeWithText("Remove local device data?").assertDoesNotExist()
        compose.runOnIdle { assertEquals(0, revoked); assertEquals(0, forgotten) }
        compose.onNodeWithText("Revoke device on host").performScrollTo().performClick()
        compose.runOnIdle { state = state.copy(resetRequired = true) }
        compose.onNodeWithText("Revoke").assertIsNotEnabled()
        compose.onAllNodesWithText("Cancel").filter(hasClickAction()).onLast().performClick()
        compose.onNodeWithText("Reset local device").performScrollTo().performClick()
        compose.runOnIdle { state = state.copy(resetRequired = false) }
        compose.onNodeWithText("Remove local device data?").assertDoesNotExist()
        compose.runOnIdle { assertEquals(0, revoked); assertEquals(0, forgotten) }
        androidx.test.platform.app.InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
            putString("zenpi_phase", "device_confirmation_scope_verified")
        })
    }
    @Test fun unresolvedRevocationDisablesScanRefreshAndAnotherRevoke() {
        val host = zenpi.security.PairedHost("127.0.0.1", 7777, "a".repeat(64), "b".repeat(64))
        compose.setContent { MaterialTheme { PairingScreen(PairingState(phase = PairingPhase.OFFLINE, registered = true,
            host = host, pendingRevoke = java.util.UUID.randomUUID().toString()), {}, {}, {}) } }
        compose.onNodeWithText("Scan QR").assertDoesNotExist()
        compose.onNodeWithText("Refresh host").assertIsNotEnabled()
        compose.onNodeWithText("Revoke device on host").performScrollTo().assertIsNotEnabled()
        compose.onNodeWithText("Forget device locally").assertIsEnabled()
    }
    @Test fun scannerErrorIsVisibleWithoutRemovingCachedHostAndPendingWriteHidesScan() {
        val host = zenpi.security.PairedHost("127.0.0.1", 7777, "a".repeat(64), "b".repeat(64))
        var state by androidx.compose.runtime.mutableStateOf(PairingState(host = host, scanError = "Camera permission denied"))
        val selections = mutableListOf<String?>()
        compose.setContent { MaterialTheme { PairingScreen(state, {}, {}, {}, onRefresh = { selections += it }) } }
        compose.onNodeWithText("Camera permission denied").assertIsDisplayed()
        compose.onNodeWithText("127.0.0.1:7777").assertIsDisplayed()
        compose.onNodeWithText("Scan QR").assertIsEnabled()
        val projection = zenpi.transport.HostProjection(listOf(zenpi.transport.RemoteProject("project", "Project", "session", workspaceControl = true)),
            "project", "session", "idle", emptyList(), "1", false, workspaceOwner = "c".repeat(64), workspaceRevision = "1")
        compose.runOnIdle { state = state.copy(projection = projection, selectedProjectUnavailable = true) }
        compose.onNodeWithText("Refresh host").performScrollTo().performClick()
        assertEquals(listOf<String?>(null), selections)
        compose.runOnIdle { state = state.copy(selectedProjectUnavailable = false) }
        compose.onNodeWithText("Refresh host").performClick()
        assertEquals(listOf(null, "project"), selections)
        compose.runOnIdle { state = state.copy(pendingWorkspace = zenpi.transport.WorkspaceCommand.create(projection, "project", "workspace_select")) }
        compose.onNodeWithText("Scan QR").assertDoesNotExist()
        compose.onNodeWithText("Retry same request").performScrollTo().assertIsDisplayed()
        compose.runOnIdle { state = state.copy(phase = PairingPhase.CONNECTED, pendingWorkspace = null,
            projection = projection.copy(snapshotReplaced = true)) }
        compose.onNodeWithTag("synchronization-status").performScrollTo().assertTextEquals("Snapshot replaced")
        compose.runOnIdle { state = state.copy(projection = projection.copy(hasMoreEvents = true)) }
        compose.onNodeWithTag("synchronization-status").assertTextEquals("Catching up")
        compose.runOnIdle { state = state.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithTag("synchronization-status").assertDoesNotExist()
        androidx.test.platform.app.InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
            putString("zenpi_sync_status", "snapshot_tail_offline_status_verified")
        })
    }
    @Test fun localForgetRequiresConfirmationAndCorruptMetadataHasResetRoute() {
        var removed = false
        compose.setContent { MaterialTheme { PairingScreen(PairingState(phase = PairingPhase.ERROR,
            resetRequired = true, error = "Saved device metadata unavailable"), {}, {}, {}, onForget = { removed = true }) } }
        compose.onNodeWithText("Scan QR").assertDoesNotExist()
        compose.onNodeWithText("Reset local device").performClick()
        assertFalse(removed)
        compose.onNodeWithText("Remove local device data?").assertIsDisplayed()
        compose.onNodeWithText("Cancel").performClick(); assertFalse(removed)
        compose.onNodeWithText("Reset local device").performClick()
        compose.onNodeWithText("Remove local data").performClick(); assertTrue(removed)
    }
}
