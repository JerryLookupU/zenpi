package zenpi

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.first
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.security.*
import java.io.File

class LocalResetTest {
    @get:Rule val compose = createComposeRule()

    @Test fun resetAlsoJoinsCancelledJobsAfterTheirFieldsWereCleared(): Unit = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "retired-reset-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val identity = DeviceIdentity.loadOrCreate("zenpi.$namespace")
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), identity.deviceId)
        store.save(host)
        val model = PairingViewModel(context.applicationContext as android.app.Application, store, identity)
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
        val fields = listOf("connection", "sessionHistoryJob")
        val stopped = fields.map { CompletableDeferred<Unit>() }
        val released = fields.map { CompletableDeferred<Unit>() }
        val jobs = mutableListOf<Job>()
        try {
            repeat(60) { if (model.state.value.host == null) delay(100) }
            assertEquals(host, model.state.value.host)
            compose.runOnUiThread {
                val owner = PairingViewModel::class.java.getDeclaredField("ownedJobs")
                    .apply { isAccessible = true }.get(model) as OwnedJobs
                for ((index, field) in fields.withIndex()) {
                    val job = owner.launch(scope) {
                        try { awaitCancellation() }
                        finally { withContext(NonCancellable) { stopped[index].complete(Unit); released[index].await() } }
                    }
                    jobs += job
                    PairingViewModel::class.java.getDeclaredField(field).apply { isAccessible = true }.set(model, job)
                }
                model.cancel()
                fields.forEach { field -> assertNull(PairingViewModel::class.java.getDeclaredField(field)
                    .apply { isAccessible = true }.get(model)) }
                model.forget()
            }
            withTimeout(5000) { stopped.forEach { it.await() } }
            for ((index, field) in fields.withIndex()) {
                assertNull("Retired $field cleanup must prevent reset completion", withTimeoutOrNull(250) {
                    model.state.first { !it.resetting }
                })
                compose.runOnUiThread {
                    assertTrue(model.state.value.resetting)
                    assertEquals(host, store.loadState()?.host)
                    assertNotNull(DeviceIdentity.loadExisting("zenpi.$namespace"))
                }
                released[index].complete(Unit)
                withTimeout(5000) { jobs[index].join() }
            }
            withTimeout(5000) { model.state.first { !it.resetting } }
            assertFalse(model.state.value.resetRequired)
            assertNull(store.loadState()); assertNull(DeviceIdentity.loadExisting("zenpi.$namespace"))
            InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", "local_reset_retired_jobs_joined_verified")
            })
        } finally {
            released.forEach { it.complete(Unit) }
            withContext(NonCancellable) {
                jobs.forEach { it.cancelAndJoin() }; scope.cancel()
                withTimeout(5000) { model.state.first { !it.resetting } }
            }
            compose.runOnUiThread { model.cancel() }
            store.wipe(); identity.wipe()
        }
    }

    @Test fun resetWaitsForEveryOwnedJobBeforeRemovingCredentials(): Unit = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "read-reset-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val identity = DeviceIdentity.loadOrCreate("zenpi.$namespace")
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), identity.deviceId)
        store.save(host)
        val model = PairingViewModel(context.applicationContext as android.app.Application, store, identity)
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)
        val fields = listOf("connection", "diffJob", "terminalJob", "sessionsJob", "transcriptJob",
            "workerHistoryJob", "workerStatusesJob", "filesJob", "resourcesJob", "modelJob", "sessionHistoryJob")
        val stopped = fields.map { CompletableDeferred<Unit>() }
        val released = fields.map { CompletableDeferred<Unit>() }
        val jobs = mutableListOf<Job>()
        try {
            repeat(60) { if (model.state.value.host == null) delay(100) }
            assertEquals(host, model.state.value.host)
            compose.runOnUiThread {
                // Hold real coroutine cancellation cleanup without adding a production test API.
                for ((index, field) in fields.withIndex()) {
                    val job = scope.launch(start = CoroutineStart.UNDISPATCHED) {
                        try { awaitCancellation() }
                        finally { withContext(NonCancellable) { stopped[index].complete(Unit); released[index].await() } }
                    }
                    jobs += job
                    PairingViewModel::class.java.getDeclaredField(field).apply { isAccessible = true }.set(model, job)
                }
                model.forget()
                assertTrue(model.state.value.resetting)
            }
            withTimeout(5000) { stopped.forEach { it.await() } }
            for ((index, field) in fields.withIndex()) {
                assertNull("Held $field cleanup must prevent reset completion", withTimeoutOrNull(250) {
                    model.state.first { !it.resetting }
                })
                compose.runOnUiThread {
                    assertTrue("$field cleanup must join before wiping", model.state.value.resetting)
                    assertEquals(host, store.loadState()?.host)
                    assertNotNull(DeviceIdentity.loadExisting("zenpi.$namespace"))
                }
                released[index].complete(Unit)
                withTimeout(5000) { jobs[index].join() }
            }
            repeat(100) { if (model.state.value.resetting) delay(100) }
            assertFalse(model.state.value.resetting); assertFalse(model.state.value.resetRequired)
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            assertTrue(jobs.all { it.isCompleted })
            assertNull(store.loadState()); assertNull(DeviceIdentity.loadExisting("zenpi.$namespace"))
            InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", "local_reset_all_owned_jobs_joined_verified")
            })
        } finally {
            released.forEach { it.complete(Unit) }
            withContext(NonCancellable) {
                jobs.forEach { it.cancelAndJoin() }; scope.cancel()
                withTimeout(5000) { model.state.first { !it.resetting } }
            }
            compose.runOnUiThread { model.cancel() }
            store.wipe(); identity.wipe()
        }
    }

    @Test fun corruptedMetadataCanOnlyRecoverThroughExplicitConfirmedReset(): Unit = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "corrupt-reset-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val identity = DeviceIdentity.loadOrCreate("zenpi.$namespace")
        val base = File(context.noBackupFilesDir, "paired-host.$namespace")
        base.writeBytes(byteArrayOf(1, 2, 3))
        val model = PairingViewModel(context.applicationContext as android.app.Application, store, identity)
        try {
            repeat(60) { if (!model.state.value.resetRequired) delay(100) }
            assertTrue(model.state.value.resetRequired)
            compose.runOnUiThread {
                model.acceptUri("invalid queued invite")
                assertFalse(model.beginScan()); model.cancel()
                assertTrue(model.state.value.resetRequired)
                model.forget()
                assertTrue(model.state.value.resetting); assertFalse(model.beginScan())
                model.acceptUri("invalid during reset"); model.cancel(); model.forget()
                model.setForeground(true); model.networkChanged(null); model.setForeground(false)
                model.networkChanged(123)
                model.refresh(); model.reviewDiff(".")
                assertTrue(model.state.value.resetting)
                assertEquals(PairingPhase.CONNECTING, model.state.value.phase)
            }
            repeat(100) { if (model.state.value.resetting) delay(100) }
            assertFalse(model.state.value.resetRequired); assertFalse(model.state.value.resetting)
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            assertNull(store.loadState()); assertNull(DeviceIdentity.loadExisting("zenpi.$namespace"))
            assertFalse(base.exists()); assertFalse(File(base.path + ".reset").exists())
            compose.runOnUiThread { assertTrue(model.beginScan()) }
        } finally { compose.runOnUiThread { model.cancel() }; store.wipe(); identity.wipe() }
    }

    @Test fun interruptedResetFencesNewProcessReadsWritesAndResumeUntilRetry() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "interrupted-reset-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), "b".repeat(64))
        val base = File(context.noBackupFilesDir, "paired-host.$namespace")
        try {
            store.save(host)
            val snapshot = requireNotNull(store.loadResumeSnapshot())
            store.beginLocalReset()
            val restarted = PairedHostStore(context, namespace)
            assertTrue(runCatching { restarted.loadState() }.isFailure)
            assertTrue(runCatching { restarted.loadResumeSnapshot() }.isFailure)
            assertTrue(runCatching { restarted.save(host) }.isFailure)
            val projection = zenpi.transport.HostProjection(emptyList(), null, null, "idle", emptyList(), null, false)
            assertTrue(runCatching { restarted.commitResume(snapshot, projection) }.isFailure)
            assertTrue(base.exists())
            restarted.wipe()
            assertNull(store.loadState()); assertFalse(File(base.path + ".reset").exists())
            val keys = java.security.KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
            assertFalse(keys.containsAlias("zenpi.host-metadata.$namespace"))
        } finally { store.wipe() }
    }

    @Test fun failedFileDeletionPreservesResetFenceAndNeverClaimsEmptyStore(): Unit = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "failed-reset-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val base = File(context.noBackupFilesDir, "paired-host.$namespace")
        val identity = DeviceIdentity.loadOrCreate("zenpi.$namespace")
        assertTrue(base.mkdir()); val obstacle = File(base, "fixture").apply { writeText("fixture") }
        val model = PairingViewModel(context.applicationContext as android.app.Application, store, identity)
        try {
            assertTrue(runCatching { store.wipe() }.isFailure)
            assertTrue(File(base.path + ".reset").exists())
            assertTrue(runCatching { PairedHostStore(context, namespace).loadState() }.isFailure)
            repeat(60) { if (!model.state.value.resetRequired) delay(100) }
            compose.runOnUiThread { model.forget() }
            repeat(100) { if (model.state.value.resetting) delay(100) }
            assertTrue(model.state.value.resetRequired); assertFalse(model.state.value.resetting)
            assertEquals(PairingPhase.ERROR, model.state.value.phase)
            compose.runOnUiThread {
                model.acceptUri("invalid after failed reset"); model.cancel()
                model.refresh(); model.reviewDiff(".")
                assertFalse(model.beginScan()); assertTrue(model.state.value.resetRequired)
            }
            assertTrue(obstacle.delete()); assertTrue(base.delete())
            compose.runOnUiThread { model.forget() }
            repeat(100) { if (model.state.value.resetting) delay(100) }
            assertFalse(model.state.value.resetRequired); assertFalse(model.state.value.resetting)
            assertNull(store.loadState()); assertNull(DeviceIdentity.loadExisting("zenpi.$namespace"))
        } finally { compose.runOnUiThread { model.cancel() }; obstacle.delete(); base.delete(); store.wipe(); identity.wipe() }
    }
}
