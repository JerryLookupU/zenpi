package zenpi

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.security.*
import zenpi.transport.*
import java.util.Base64

class ScannerResultTest {
    @get:Rule val compose = createComposeRule()

    private fun invitation(): String {
        val token = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32))
        return "zenpi://pair?v=1&address=192.168.50.55%3A7777&fingerprint=${"a".repeat(64)}&token=$token&expires=${System.currentTimeMillis()/1000 + 120}"
    }

    private fun phase(name: String) {
        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", name) })
    }

    private fun coldForeground(pending: Boolean = false, uri: String? = null, cancelQueued: Boolean = false,
        inspectQueued: ((PairingViewModel) -> Unit)? = null,
        block: (PairingViewModel, PairedHostStore, SavedHostState) -> Unit): Unit = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "scanner-cold-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), "b".repeat(64))
        val projection = HostProjection(listOf(RemoteProject("project", "Project", "session", workspaceControl = true)),
            "project", "session", "idle", emptyList(), "1", false, eventSequence = "0", workspaceOwner = "c".repeat(64), workspaceRevision = "1")
        store.save(host); store.saveProjection(host, projection)
        if (pending) store.beginWorkspace(host, WorkspaceCommand.create(projection, "project", "workspace_select"))
        val saved = requireNotNull(store.loadState())
        val identity = DeviceIdentity.loadOrCreate("zenpi.$namespace")
        lateinit var model: PairingViewModel
        var created = false
        try {
            // Main-thread continuation of the IO read cannot run until this
            // block returns: reproduce onStart/deep link before restore.
            compose.runOnUiThread {
                model = PairingViewModel(context.applicationContext as android.app.Application, store, identity)
                created = true
                model.setForeground(true)
                uri?.let(model::acceptUri)
                inspectQueued?.invoke(model)
                if (cancelQueued) model.cancel()
            }
            val metadata = PairingViewModel::class.java.getDeclaredField("metadataLoaded").apply { isAccessible = true }
            repeat(60) { if (!metadata.getBoolean(model)) delay(100) }
            assertTrue("Cold metadata recovery did not finish", metadata.getBoolean(model))
            compose.runOnUiThread { block(model, store, saved) }
        } finally {
            if (created) compose.runOnUiThread { model.setForeground(false); model.cancel() }
            store.wipe(); identity.wipe()
        }
    }

    @Test fun savedHostColdForegroundReconnectDoesNotLeaveScannerPermanentlyLoading() = coldForeground { model, _, saved ->
        assertEquals(saved.host, model.state.value.host)
        model.setForeground(false)
        assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
        assertTrue("Metadata admission must complete before reconnect changes generation", model.beginScan())
        model.scanResult(invitation())
        assertEquals(PairingPhase.CONFIRMING, model.state.value.phase)
        phase("cold_restore_scanner_ready_verified")
    }

    @Test fun coldDeepLinkTakesPrecedenceOverAutomaticSavedHostReconnect() = coldForeground(uri = invitation()) { model, store, saved ->
        assertEquals(PairingPhase.CONFIRMING, model.state.value.phase)
        assertNotNull(model.state.value.invite); assertNull(model.state.value.host)
        assertEquals(saved, store.loadState())
        model.setForeground(true)
        val before = model.state.value
        val correct = requireNotNull(before.invite).safetyCode
        val wrong = if (correct == "000000") "111111" else "000000"
        for (code in listOf("", wrong, correct + "\n", " $correct")) {
            model.confirm(code)
            assertEquals(before, model.state.value); assertEquals(saved, store.loadState())
        }
        model.setForeground(false)
        phase("cold_deep_link_confirmation_verified")
    }

    @Test fun coldDeepLinkCannotReplaceUnresolvedDurableWorkspaceRequest() = coldForeground(pending = true, uri = invitation()) { model, store, saved ->
        assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
        assertEquals(saved.host, model.state.value.host); assertNull(model.state.value.invite)
        assertEquals(saved.pendingWorkspace?.envelope, model.state.value.pendingWorkspace?.envelope)
        assertFalse(model.beginScan()); assertNotNull(model.state.value.error)
        assertEquals(saved.host, store.loadState()?.host)
        assertEquals(saved.pendingWorkspace?.envelope, store.loadState()?.pendingWorkspace?.envelope)
        phase("cold_deep_link_pending_fence_verified")
    }

    @Test fun cancellingQueuedColdDeepLinkStillAllowsLaterScanning() {
        for (uri in listOf(null, invitation())) coldForeground(uri = uri, cancelQueued = true) { model, store, saved ->
            assertEquals(saved.host, model.state.value.host)
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            assertNull(model.state.value.invite)
            val generation = PairingViewModel::class.java.getDeclaredField("generation").apply { isAccessible = true }
            val connection = PairingViewModel::class.java.getDeclaredField("connection").apply { isAccessible = true }
            assertEquals("Cold restore must honor an explicit stop", 0, generation.getInt(model))
            assertNull(connection.get(model))
            assertEquals(saved, store.loadState())
            assertTrue(model.beginScan())
            model.scanResult(invitation())
            assertEquals(PairingPhase.CONFIRMING, model.state.value.phase)
            assertNotNull(model.state.value.invite)
            assertEquals(saved, store.loadState())
        }
        phase("cold_deep_link_cancel_verified")
    }

    @Test fun coldInvitationQueueRejectsRawOversizedInputAndRetainsOriginalDeadline() {
        var admitted: PairingInvite? = null
        coldForeground(uri = invitation(), inspectQueued = { model ->
            val queue = PairingViewModel::class.java.getDeclaredField("queuedInvite").apply { isAccessible = true }
            admitted = queue.get(model) as PairingInvite
            for (invalid in listOf(" ".repeat(4096), "not-a-qr-PRIVATE_SCAN_SECRET", "\n${invitation()}")) {
                model.acceptUri(invalid)
                assertSame(admitted, queue.get(model))
                assertFalse(requireNotNull(model.state.value.scanError).contains("PRIVATE_SCAN_SECRET"))
            }
        }) { model, store, saved ->
            assertEquals(PairingPhase.CONFIRMING, model.state.value.phase)
            assertSame("Restore must not parse again and renew the monotonic deadline", admitted, model.state.value.invite)
            assertNull(model.state.value.host)
            assertEquals(saved, store.loadState())
            phase("cold_invitation_validated_queue_verified")
        }
    }

    @Test fun invitationExpiredWhileQueuedCannotReenterConfirmationAfterRestore() = coldForeground(uri = invitation(), inspectQueued = { model ->
        val invite = PairingViewModel::class.java.getDeclaredField("queuedInvite").apply { isAccessible = true }
            .get(model) as PairingInvite
        assertTrue(runCatching { invite.requireFresh(invite.expiresAt) }.isFailure)
    }) { model, store, saved ->
        assertEquals(saved.host, model.state.value.host)
        assertNull(model.state.value.invite)
        val generation = PairingViewModel::class.java.getDeclaredField("generation").apply { isAccessible = true }
        assertTrue("Expired queued invitation must not suppress saved-host reconnect", generation.getInt(model) > 0)
        model.setForeground(false)
        assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
        assertEquals("Invitation expired or invalid", model.state.value.scanError)
        assertEquals(saved, store.loadState())
        phase("cold_invitation_expired_queue_verified")
    }

    private fun exercise(pending: Boolean = false, block: (PairingViewModel, PairedHostStore) -> Unit): Unit = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val store = PairedHostStore(context, "scanner-${System.nanoTime()}")
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), "b".repeat(64))
        val projection = HostProjection(listOf(RemoteProject("project", "Project", "session", workspaceControl = true)),
            "project", "session", "idle", emptyList(), "1", false, eventSequence = "0", workspaceOwner = "c".repeat(64), workspaceRevision = "1")
        store.save(host); store.saveProjection(host, projection)
        if (pending) store.beginWorkspace(host, WorkspaceCommand.create(projection, "project", "workspace_select"))
        val model = PairingViewModel(context.applicationContext as android.app.Application, store)
        try {
            repeat(60) { if (model.state.value.host == null) delay(100) }
            assertEquals(host, model.state.value.host)
            compose.runOnUiThread { block(model, store) }
        } finally {
            compose.runOnUiThread { model.cancel() }
            store.wipe()
        }
    }

    @Test fun permissionTimeoutAndUnavailablePreserveHostCacheAndAuthority() = exercise { model, store ->
        val before = model.state.value
        val saved = store.loadState()
        assertTrue(model.beginScan())
        model.scanResult(null, missingPermission = true)
        assertTrue(requireNotNull(model.state.value.scanError).contains("permission denied"))
        assertEquals(before.copy(scanError = model.state.value.scanError), model.state.value)
        assertEquals(saved, store.loadState())
        assertTrue(model.beginScan()); assertNull(model.state.value.scanError)
        model.scanResult(null, timedOut = true)
        assertTrue(requireNotNull(model.state.value.scanError).contains("timed out"))
        assertTrue(model.beginScan())
        model.cameraUnavailable(); assertEquals("Camera unavailable", model.state.value.scanError)
        assertEquals(saved, store.loadState())
    }

    @Test fun cancellationDoesNotForgetOrInventPairing() = exercise { model, store ->
        val before = model.state.value; val saved = store.loadState()
        assertTrue(model.beginScan())
        model.scanResult(null)
        assertEquals(before, model.state.value)
        val after = requireNotNull(store.loadState())
        assertEquals(saved?.host, after.host); assertEquals(saved?.projection, after.projection)
        assertEquals(saved?.pendingWorkspace?.envelope, after.pendingWorkspace?.envelope)
    }

    @Test fun validScanRequiresConfirmationAndNeverPersistsInvitation() = exercise { model, store ->
        val saved = store.loadState()
        assertTrue(model.beginScan())
        model.scanResult(invitation())
        assertEquals(PairingPhase.CONFIRMING, model.state.value.phase)
        assertNotNull(model.state.value.invite); assertNull(model.state.value.host)
        assertEquals(saved, store.loadState())
    }

    @Test fun malformedScanNeverBecomesHostOrEchoesSecret() = exercise { model, store ->
        val before = model.state.value
        val saved = store.loadState()
        val valid = invitation()
        for (contents in listOf("https://example.invalid/PRIVATE_SCAN_SECRET", " $valid", "$valid ",
            "\n$valid", "$valid\r\n", "\t$valid\t", " ".repeat(2048) + valid, valid + " ".repeat(2048))) {
            assertTrue(model.beginScan())
            model.scanResult(contents)
            assertEquals(before.copy(scanError = model.state.value.scanError), model.state.value)
            assertEquals(saved?.host, model.state.value.host)
            assertNull(model.state.value.invite)
            val error = requireNotNull(model.state.value.scanError)
            assertFalse(error.contains("PRIVATE_SCAN_SECRET")); assertFalse(error.contains("zenpi://pair"))
            assertEquals(saved, store.loadState())
        }
        phase("scanner_raw_payload_fence_verified")
    }

    @Test fun cancellingConfirmationRestoresSavedDisplayWithoutRestoringControl() = exercise { model, store ->
        val saved = requireNotNull(store.loadState())
        assertTrue(model.beginScan())
        model.scanResult(invitation())
        assertEquals(PairingPhase.CONFIRMING, model.state.value.phase)
        model.cancel()
        assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
        assertEquals(saved.host, model.state.value.host)
        assertEquals(saved.projection, model.state.value.projection)
        assertNull(model.state.value.invite)
        assertEquals(saved, store.loadState())
        phase("repair_confirmation_cancel_restores_host_verified")
    }

    @Test fun invalidReplacementQrDoesNotLoseAnExistingConfirmationOrSavedHost() = exercise { model, store ->
        val saved = requireNotNull(store.loadState())
        assertTrue(model.beginScan())
        model.scanResult(invitation())
        val before = model.state.value
        assertTrue(model.beginScan())
        model.scanResult("not-a-qr-PRIVATE_SCAN_SECRET")
        assertEquals(before.copy(scanError = model.state.value.scanError), model.state.value)
        assertFalse(requireNotNull(model.state.value.scanError).contains("PRIVATE_SCAN_SECRET"))
        model.cancel()
        assertEquals(saved.host, model.state.value.host)
        assertEquals(saved.projection, model.state.value.projection)
        assertEquals(saved, store.loadState())
        phase("invalid_replacement_qr_preserves_host_verified")
    }

    @Test fun unresolvedWriteFencesScannerResultsIncludingErrors() = exercise(pending = true) { model, store ->
        val before = model.state.value; val saved = store.loadState()
        assertFalse(before.canScan); assertFalse(model.beginScan())
        model.scanResult(null, missingPermission = true); model.cameraUnavailable()
        model.scanResult("invalid")
        assertEquals(before, model.state.value)
        val after = requireNotNull(store.loadState())
        assertEquals(saved?.host, after.host); assertEquals(saved?.projection, after.projection)
        assertEquals(saved?.pendingWorkspace?.envelope, after.pendingWorkspace?.envelope)
    }
    @Test fun duplicateLaunchAndLateResultsCannotReplaceCancellationOrNewDeepLink() = exercise { model, store ->
        val before = model.state.value; val saved = store.loadState()
        model.scanResult(invitation())
        assertEquals(before, model.state.value)
        assertTrue(model.beginScan()); assertFalse(model.beginScan()); assertFalse(model.state.value.canScan)
        model.setForeground(true); model.networkChanged(123L)
        assertTrue(model.state.value.scanInProgress)
        assertNotEquals(PairingPhase.CONNECTING, model.state.value.phase)
        model.cancel()
        val cancelled = model.state.value
        assertFalse(model.beginScan())
        model.scanResult(invitation()); model.scanResult(null, missingPermission = true); model.cameraUnavailable()
        assertEquals(cancelled.copy(scanInProgress = false), model.state.value)
        assertTrue(model.beginScan())
        model.acceptUri(invitation())
        val confirmed = model.state.value
        assertEquals(PairingPhase.CONFIRMING, confirmed.phase); assertTrue(confirmed.scanInProgress)
        assertFalse(model.beginScan())
        model.scanResult("invalid"); model.scanResult(null, timedOut = true)
        assertEquals(confirmed.copy(scanInProgress = false), model.state.value); assertEquals(saved, store.loadState())
        phase("scanner_late_result_fence_verified")
    }

    @Test fun cameraAdmissionImmediatelyStopsLiveReadsBeforeActivityBackgrounds() = exercise { model, store ->
        val saved = requireNotNull(store.loadState())
        val cached = requireNotNull(saved.projection)
        val live = cached.copy(workspaceOwner = "c".repeat(64), workspaceRevision = "1",
            projects = cached.projects.map { it.copy(workspaceControl = true, promptControl = true, approvalControl = true) })
        val connection = Job()
        val transcript = Job()
        @Suppress("UNCHECKED_CAST")
        val mutable = PairingViewModel::class.java.getDeclaredField("mutable").apply { isAccessible = true }
            .get(model) as MutableStateFlow<PairingState>
        val connectionField = PairingViewModel::class.java.getDeclaredField("connection").apply { isAccessible = true }
        val transcriptField = PairingViewModel::class.java.getDeclaredField("transcriptJob").apply { isAccessible = true }
        val generationField = PairingViewModel::class.java.getDeclaredField("generation").apply { isAccessible = true }
        try {
            mutable.value = model.state.value.copy(phase = PairingPhase.CONNECTED, projection = live, transcriptLoading = true)
            connectionField.set(model, connection); transcriptField.set(model, transcript)
            val generation = generationField.getInt(model)
            assertTrue(model.beginScan())
            assertTrue(connection.isCancelled); assertTrue(transcript.isCancelled)
            assertNull(connectionField.get(model)); assertNull(transcriptField.get(model))
            assertTrue(generationField.getInt(model) > generation)
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            assertFalse(model.state.value.transcriptLoading)
            assertNull(model.state.value.projection?.workspaceOwner)
            assertTrue(requireNotNull(model.state.value.projection).projects.all {
                !it.workspaceControl && !it.promptControl && !it.approvalControl
            })
            assertEquals(cached.turns, model.state.value.projection?.turns)
            assertEquals(cached.eventSequence, model.state.value.projection?.eventSequence)
            assertEquals(saved, store.loadState())
            val scanning = model.state.value
            assertFalse(model.beginScan())
            assertEquals(scanning, model.state.value)
            model.scanResult(null)
            assertEquals(scanning.copy(scanInProgress = false), model.state.value)
            assertEquals(saved, store.loadState())
            assertTrue(model.beginScan())
            PairingViewModel::class.java.getDeclaredField("foreground").apply { isAccessible = true }.setBoolean(model, true)
            model.cameraUnavailable()
            assertFalse(model.state.value.scanInProgress)
            assertEquals("Camera unavailable", model.state.value.scanError)
            assertEquals(PairingPhase.CONNECTING, model.state.value.phase)
            assertNull(model.state.value.projection?.workspaceOwner)
            assertEquals(saved, store.loadState())
            model.setForeground(false)
            phase("scanner_live_read_pause_verified")
        } finally {
            connection.cancel(); transcript.cancel()
        }
    }
}
