package zenpi

import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import zenpi.security.*
import zenpi.transport.*
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference

class ForegroundStoreTest {
    private fun exercise(blocked: Boolean) = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val lock = PairedHostStore::class.java.getDeclaredField("storageLock").apply { isAccessible = true }.get(null)
        for (operation in listOf("pair", "projection", "begin_workspace", "finish_workspace", "begin_revoke", "finish_revoke")) {
            val namespace = "foreground-store-${System.nanoTime()}"
            val store = PairedHostStore(context, namespace)
            val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), "b".repeat(64))
            val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session")), "project", "session", "idle",
                listOf(RemoteTurn("old", "assistant", "old")), "1", false, "1")
            val next = projection.copy(turns = listOf(RemoteTurn("new", "assistant", "new")), journalSequence = "2", eventSequence = "2")
            val command = WorkspaceCommand.create(projection.copy(projects = projection.projects.map { it.copy(workspaceControl = true) },
                workspaceOwner = "c".repeat(64), workspaceRevision = "1"), "project", "workspace_select")
            val revoke = java.util.UUID.randomUUID().toString()
            val checks = AtomicInteger()
            var keyRemoved = false
            fun commit(check: () -> Unit) {
                when (operation) {
                    "pair" -> store.save(host.copy(port = 8888), check)
                    "projection" -> store.saveProjection(host, next, check)
                    "begin_workspace" -> store.beginWorkspace(host, command, check)
                    "finish_workspace" -> store.finishWorkspace(host, command, next, check)
                    "begin_revoke" -> store.beginRevoke(host, revoke, check)
                    "finish_revoke" -> store.finishRevoke(host, revoke, check) { keyRemoved = true }
                }
            }
            val held = CountDownLatch(1); val release = CountDownLatch(1)
            var owner: Thread? = null; var job: Job? = null
            try {
                store.save(host); store.saveProjection(host, projection)
                if (operation == "finish_workspace") store.beginWorkspace(host, command)
                if (operation == "finish_revoke") store.beginRevoke(host, revoke)
                val before = store.loadState()
                val file = java.io.File(context.noBackupFilesDir, "paired-host.$namespace")
                val bytes = file.readBytes()
                if (blocked) {
                    owner = Thread { synchronized(lock) { held.countDown(); check(release.await(10, TimeUnit.SECONDS)) } }.apply { start() }
                    assertTrue(held.await(5, TimeUnit.SECONDS))
                    val waiting = AtomicReference<Thread>()
                    job = launch(Dispatchers.IO) {
                        val commitContext = kotlin.coroutines.coroutineContext
                        waiting.set(Thread.currentThread())
                        commit { checks.incrementAndGet(); commitContext.ensureActive() }
                    }
                    withTimeout(5000) { while (waiting.get()?.state != Thread.State.BLOCKED) delay(10) }
                    job.cancel(); release.countDown()
                    withTimeout(5000) { job.join() }
                    owner.join(5000); assertFalse(owner.isAlive)
                    assertTrue(job.isCancelled); assertEquals(operation, 1, checks.get())
                } else {
                    val error = runCatching {
                        commit { if (checks.incrementAndGet() == 2) throw CancellationException("fixture operation replaced") }
                    }.exceptionOrNull()
                    assertTrue("$operation: $error", error is CancellationException)
                    assertEquals(operation, 2, checks.get())
                }
                assertFalse(operation, keyRemoved)
                val after = requireNotNull(store.loadState())
                assertArrayEquals(operation, bytes, file.readBytes())
                assertEquals(operation, before?.copy(pendingWorkspace = null), after.copy(pendingWorkspace = null))
                assertEquals(operation, before?.pendingWorkspace?.envelope, after.pendingWorkspace?.envelope)
                var admitted = 0
                commit { admitted++ }
                assertEquals(operation, 2, admitted)
                when (operation) {
                    "pair" -> assertEquals(8888, store.load()?.port)
                    "projection" -> assertEquals(next, store.loadState()?.projection)
                    "begin_workspace" -> assertEquals(command.envelope, store.loadState()?.pendingWorkspace?.envelope)
                    "finish_workspace" -> { assertNull(store.loadState()?.pendingWorkspace); assertEquals(next, store.loadState()?.projection) }
                    "begin_revoke" -> assertEquals(revoke, store.loadState()?.pendingRevoke)
                    "finish_revoke" -> { assertTrue(keyRemoved); assertNull(store.loadState()) }
                }
            } finally {
                release.countDown(); job?.cancel()
                withContext(NonCancellable) { job?.join() }
                owner?.join(5000); store.wipe()
            }
        }
        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
            putString("zenpi_phase", if (blocked) "foreground_storage_wait_cancellation_fenced_verified"
                else "foreground_storage_final_admission_fenced_verified")
        })
    }

    @Test fun cancelledForegroundWritesCannotPublishAfterWaitingForStorage() = exercise(true)
    @Test fun replacedForegroundWritesCannotPublishOrRemoveKeysAtFinalAdmission() = exercise(false)
}
