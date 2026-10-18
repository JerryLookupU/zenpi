package zenpi

import androidx.test.platform.app.InstrumentationRegistry
import androidx.compose.ui.test.junit4.createComposeRule
import kotlinx.coroutines.delay
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.security.*

class DeviceRevokeTest {
    @get:Rule val compose = createComposeRule()
    @Test fun cancelledIdentityRemovalRetainsTheKeyAndExactRevocationIntent() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "revoke-cancel-${System.nanoTime()}"
        val alias = "zenpi.$namespace"
        val identity = DeviceIdentity.loadOrCreate(alias)
        val store = PairedHostStore(context, namespace)
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), identity.deviceId)
        val request = java.util.UUID.randomUUID().toString()
        val cancelled = kotlinx.coroutines.CancellationException("revocation owner replaced")
        try {
            store.save(host); store.beginRevoke(host, request)
            val metadata = java.io.File(context.noBackupFilesDir, "paired-host.$namespace")
            val saved = metadata.readBytes()
            // Identity checks bracket its read, then recheck after loading the deletion store.
            for (cancelAt in 1..3) {
                var checks = 0
                val error = runCatching {
                    store.finishRevoke(host, request) {
                        identity.wipe(host.deviceId) { if (++checks == cancelAt) throw cancelled }
                    }
                }.exceptionOrNull()
                assertSame(cancelled, error); assertEquals(cancelAt, checks)
                assertEquals(host.deviceId, DeviceIdentity.loadExisting(alias)?.deviceId)
                assertArrayEquals(saved, metadata.readBytes())
                assertEquals(request, store.loadState()?.pendingRevoke)
            }
            val entered = java.util.concurrent.CountDownLatch(1)
            val release = java.util.concurrent.CountDownLatch(1)
            val waiting = java.util.concurrent.atomic.AtomicReference<Thread>()
            val holder = Thread { synchronized(DeviceIdentity.Companion) {
                entered.countDown(); check(release.await(10, java.util.concurrent.TimeUnit.SECONDS))
            } }.apply { start() }
            var job: kotlinx.coroutines.Job? = null
            try {
                assertTrue(entered.await(5, java.util.concurrent.TimeUnit.SECONDS))
                job = launch(kotlinx.coroutines.Dispatchers.IO) {
                    val owner = kotlin.coroutines.coroutineContext
                    waiting.set(Thread.currentThread())
                    store.finishRevoke(host, request) { identity.wipe(host.deviceId) { owner.ensureActive() } }
                }
                kotlinx.coroutines.withTimeout(5000) { while (waiting.get()?.state != Thread.State.BLOCKED) delay(10) }
                job.cancel(); release.countDown()
                kotlinx.coroutines.withTimeout(5000) { job.join() }
                holder.join(5000); assertFalse(holder.isAlive); assertTrue(job.isCancelled)
                assertEquals(host.deviceId, DeviceIdentity.loadExisting(alias)?.deviceId)
                assertArrayEquals(saved, metadata.readBytes()); assertEquals(request, store.loadState()?.pendingRevoke)
            } finally {
                release.countDown(); job?.cancel()
                kotlinx.coroutines.withContext(kotlinx.coroutines.NonCancellable) { job?.join() }
                holder.join(5000)
            }
            store.finishRevoke(host, request) { identity.wipe(host.deviceId) {} }
            assertNull(DeviceIdentity.loadExisting(alias)); assertNull(store.loadState())
            InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", "revocation_identity_removal_cancellation_fenced_verified")
            })
        } finally { identity.wipe(); store.wipe() }
    }
    @Test fun oldRevocationCannotDeleteAReplacementKeyOrClearItsPendingReceipt() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "revoke-replaced-${System.nanoTime()}"
        val alias = "zenpi.$namespace"
        val identity = DeviceIdentity.loadOrCreate(alias)
        val store = PairedHostStore(context, namespace)
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), identity.deviceId)
        val request = java.util.UUID.randomUUID().toString()
        try {
            store.save(host); store.beginRevoke(host, request)
            val metadata = java.io.File(context.noBackupFilesDir, "paired-host.$namespace")
            val saved = metadata.readBytes()
            identity.wipe()
            val missing = runCatching { store.finishRevoke(host, request) { identity.wipe(host.deviceId) } }.exceptionOrNull()
            assertTrue(missing is zenpi.transport.HostAuthenticationException)
            assertArrayEquals(saved, metadata.readBytes()); assertEquals(request, store.loadState()?.pendingRevoke)
            val replacement = DeviceIdentity.loadOrCreate(alias)
            try {
                val replacementId = replacement.deviceId
                assertNotEquals(host.deviceId, replacementId)
                val changed = runCatching { store.finishRevoke(host, request) { identity.wipe(host.deviceId) } }.exceptionOrNull()
                assertTrue(changed is zenpi.transport.HostAuthenticationException)
                assertEquals(replacementId, DeviceIdentity.loadExisting(alias)?.deviceId)
                assertArrayEquals(saved, metadata.readBytes()); assertEquals(request, store.loadState()?.pendingRevoke)
                assertNull(store.loadResumeSnapshot())
                // A matching receipt may remove only its own current identity.
                replacement.wipe(replacementId)
                assertNull(DeviceIdentity.loadExisting(alias))
            } finally { replacement.wipe() }
        } finally { identity.wipe(); store.wipe() }
    }
    @Test fun unconfirmedHostRevocationKeepsMetadataAndSigningKey() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "revoke-failure-${System.nanoTime()}"
        val alias = "zenpi.$namespace"
        val identity = DeviceIdentity.loadOrCreate(alias)
        val store = PairedHostStore(context, namespace)
        val listener = java.net.ServerSocket(0)
        val host = PairedHost("127.0.0.1", listener.localPort, "a".repeat(64), identity.deviceId)
        listener.close()
        store.save(host)
        val model = PairingViewModel(context.applicationContext as android.app.Application, store, identity)
        try {
            repeat(30) { if (model.state.value.host == null) delay(100) }
            val before = model.state.value
            compose.runOnUiThread { model.revokeDevice() }
            assertEquals(before, model.state.value)
            assertNull(store.loadState()?.pendingRevoke)
            compose.runOnUiThread { model.setForeground(true) }
            repeat(60) { if (model.state.value.phase == PairingPhase.CONNECTING) delay(100) }
            compose.runOnUiThread { model.revokeDevice() }
            repeat(60) { if (model.state.value.phase != PairingPhase.ERROR) delay(100) }
            assertEquals(PairingPhase.ERROR, model.state.value.phase)
            assertTrue(requireNotNull(model.state.value.error).contains("unconfirmed"))
            assertEquals(host, store.load())
            assertEquals(host.deviceId, DeviceIdentity.loadExisting(alias)?.deviceId)
            assertTrue(model.state.value.registered)
            val intent = requireNotNull(store.loadState()?.pendingRevoke)
            assertEquals(intent, model.state.value.pendingRevoke)
            assertNull(store.loadResumeSnapshot())
            assertTrue(runCatching { store.save(host) }.isFailure)
            val restored = PairingViewModel(context.applicationContext as android.app.Application, PairedHostStore(context, namespace), identity)
            try {
                compose.runOnUiThread { restored.acceptUri("invalid-invitation") }
                repeat(30) { if (restored.state.value.host == null) delay(100) }
                assertEquals(intent, restored.state.value.pendingRevoke)
                compose.runOnUiThread {
                    restored.setForeground(true); restored.networkChanged(null); restored.networkChanged(99L)
                    restored.cancel(); restored.acceptUri("invalid-invitation"); restored.refresh(); restored.revokeDevice()
                }
                delay(300)
                assertEquals(intent, restored.state.value.pendingRevoke)
                assertEquals(PairingPhase.OFFLINE, restored.state.value.phase)
                assertEquals(intent, store.loadState()?.pendingRevoke)
            } finally { compose.runOnUiThread { restored.cancel() } }
        } finally { compose.runOnUiThread { model.cancel() }; store.wipe(); identity.wipe() }
    }
    @Test fun revocationIntentFencesBackgroundAndOtherWritesAndRequiresMatchingConfirmation() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "revoke-store-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), "b".repeat(64))
        val projection = zenpi.transport.HostProjection(listOf(zenpi.transport.RemoteProject("project", "Workspace", "session")),
            "project", "session", "idle", emptyList(), "1", false, "0")
        val intent = java.util.UUID.randomUUID().toString()
        try {
            store.save(host); store.saveProjection(host, projection)
            val prior = requireNotNull(store.loadResumeSnapshot())
            store.beginRevoke(host, intent)
            assertNull(store.loadResumeSnapshot()); assertFalse(store.commitResume(prior, projection))
            assertTrue(runCatching { store.saveProjection(host, projection) }.isFailure)
            assertTrue(runCatching { store.beginRevoke(host, intent) }.isFailure)
            var keyRemoved = false
            assertTrue(runCatching { store.finishRevoke(host, java.util.UUID.randomUUID().toString()) { keyRemoved = true } }.isFailure)
            assertFalse(keyRemoved)
            assertTrue(runCatching { store.finishRevoke(host, intent) { error("fixture key removal failure") } }.isFailure)
            assertEquals(intent, store.loadState()?.pendingRevoke)
            store.finishRevoke(host, intent) { keyRemoved = true }
            assertTrue(keyRemoved); assertNull(store.loadState())
        } finally { store.wipe() }
    }
}
