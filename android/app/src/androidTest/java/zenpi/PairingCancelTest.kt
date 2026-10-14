package zenpi

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.security.*
import java.util.Base64

class PairingCancelTest {
    @get:Rule val compose = createComposeRule()

    @Test fun firstPairingCancelReconcilesAnAlreadyOwnedCredentialSave() = heldSave("cancel")
    @Test fun backgroundPairingReconcilesAnAlreadyOwnedCredentialSave() {
        heldSave("background", saveRecord = false, cancelRecovery = true)
        heldSave("background", cancelRecovery = true)
        heldSave("background", saveRecord = false); heldSave("background")
    }
    @Test fun networkLossPairingReconcilesAnAlreadyOwnedCredentialSave() {
        heldSave("network", saveRecord = false, cancelRecovery = true)
        heldSave("network", cancelRecovery = true)
        heldSave("network", saveRecord = false); heldSave("network")
    }

    private fun heldSave(interruption: String, saveRecord: Boolean = true, cancelRecovery: Boolean = false): Unit = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext
        val namespace = "pair-cancel-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val identity = DeviceIdentity.loadOrCreate("zenpi.$namespace")
        val model = PairingViewModel(context.applicationContext as android.app.Application, store, identity)
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
        val locked = CompletableDeferred<Unit>(); val release = CompletableDeferred<Unit>()
        var save: Job? = null
        val host = PairedHost("192.168.50.55", 7777, "a".repeat(64), identity.deviceId)
        try {
            var ready = false
            repeat(50) {
                if (!ready) { compose.runOnUiThread { ready = model.beginScan() }; if (!ready) delay(100) }
            }
            assertTrue(ready)
            val token = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32))
            compose.runOnUiThread {
                model.acceptUri("zenpi://pair?v=1&address=192.168.50.55%3A7777&fingerprint=${host.fingerprint}&token=$token&expires=${System.currentTimeMillis()/1000 + 120}")
                model.scanResult(null)
                assertEquals(PairingPhase.CONFIRMING, model.state.value.phase)
                model.setForeground(true)
                @Suppress("UNCHECKED_CAST")
                val state = PairingViewModel::class.java.getDeclaredField("mutable").apply { isAccessible = true }.get(model) as MutableStateFlow<PairingState>
                state.value = state.value.copy(phase = PairingPhase.CONNECTING)
            }
            val storage = PairingViewModel::class.java.getDeclaredField("storage").apply { isAccessible = true }.get(model) as Mutex
            // Model the non-suspending credential commit after host acceptance,
            // before its main-owner publication. No network outcome is fabricated.
            save = scope.launch {
                storage.withLock {
                    withContext(NonCancellable) { locked.complete(Unit); release.await(); if (saveRecord) store.save(host) }
                }
            }
            withTimeout(5000) { locked.await() }
            compose.runOnUiThread {
                PairingViewModel::class.java.getDeclaredField("connection").apply { isAccessible = true }.set(model, save)
                val connecting = model.state.value
                val replacementToken = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32) { 1 })
                model.acceptUri("zenpi://pair?v=1&address=192.168.50.99%3A8888&fingerprint=${"e".repeat(64)}&token=$replacementToken&expires=${System.currentTimeMillis()/1000 + 120}")
                assertEquals("Deep link must not replace an unresolved registration/save", connecting, model.state.value)
                assertFalse(model.beginScan())
                assertTrue("Rejected deep link must not cancel the credential owner", requireNotNull(save).isActive)
                when (interruption) {
                    "cancel" -> model.cancel()
                    "background" -> model.setForeground(false)
                    "network" -> model.networkChanged(null)
                }
                assertNull(model.state.value.host)
                if (interruption == "cancel") assertNull(model.state.value.invite)
                else {
                    assertEquals(PairingPhase.ERROR, model.state.value.phase)
                    assertTrue(requireNotNull(model.state.value.error).contains("unknown"))
                }
                assertFalse("A new scan must wait for the credential commit", model.beginScan())
                if (cancelRecovery) {
                    model.cancel()
                    // Restore foreground/LAN while storage is held: the late
                    // read must honor explicit stop rather than auto-refresh.
                    model.setForeground(true); model.networkChanged(123L)
                }
            }
            release.complete(Unit)
            withTimeout(5000) { requireNotNull(save).join() }
            val metadataLoaded = PairingViewModel::class.java.getDeclaredField("metadataLoaded").apply { isAccessible = true }
            ready = false
            repeat(50) {
                if (!ready) { compose.runOnUiThread { ready = metadataLoaded.getBoolean(model) }; if (!ready) delay(100) }
            }
            assertTrue("Reconciliation must finish the held metadata read", ready)
            fun scannerReopened() = compose.runOnUiThread {
                assertTrue("Reconciliation must reopen scanner admission", model.beginScan())
                model.scanResult(null)
            }
            if (!saveRecord) {
                if (cancelRecovery) {
                    assertEquals(PairingState(), model.state.value)
                    assertNull(store.loadState())
                    scannerReopened()
                    return@runBlocking
                }
                assertEquals(PairingPhase.ERROR, model.state.value.phase)
                assertNull(model.state.value.host); assertNull(store.loadState())
                assertNotNull(model.state.value.invite)
                assertTrue(requireNotNull(model.state.value.error).contains("unknown"))
                scannerReopened()
                return@runBlocking
            }
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            assertTrue(model.state.value.registered); assertNull(model.state.value.invite)
            assertEquals(host, store.loadState()?.host)
            assertNotNull(DeviceIdentity.loadExisting("zenpi.$namespace"))
            if (cancelRecovery) {
                delay(250)
                assertEquals("Cancelled recovery must not auto-connect", PairingPhase.OFFLINE, model.state.value.phase)
                scannerReopened()
                return@runBlocking
            }
            instrumentation.sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", when (interruption) {
                    "background" -> "background_pair_saved_host_restored_verified"
                    "network" -> "network_pair_saved_host_restored_verified"
                    else -> "first_pair_cancel_saved_host_restored_verified"
                })
            })
            scannerReopened()
        } finally {
            release.complete(Unit)
            withContext(NonCancellable) { save?.cancelAndJoin(); scope.cancel() }
            compose.runOnUiThread { model.forget() }
            withContext(NonCancellable) { withTimeout(5000) { model.state.first { !it.resetting } } }
            store.wipe(); identity.wipe()
        }
    }
}
