package zenpi

import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch
import org.junit.Assert.*
import org.junit.Test
import zenpi.security.*
import zenpi.transport.*

class ResumeStoreTest {
    @Test fun cancellationWhileWaitingForStorageOwnershipCannotCommitBackgroundProjection() = kotlinx.coroutines.runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "resume-lock-cancel-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), "b".repeat(64))
        val before = HostProjection(listOf(RemoteProject("project", "Workspace", "session")), "project", "session", "idle",
            listOf(RemoteTurn("old", "assistant", "old")), "1", false, "1")
        val next = before.copy(turns = listOf(RemoteTurn("new", "assistant", "new")), journalSequence = "2", eventSequence = "2")
        val lock = PairedHostStore::class.java.getDeclaredField("storageLock").apply { isAccessible = true }.get(null)
        val held = java.util.concurrent.CountDownLatch(1)
        val release = java.util.concurrent.CountDownLatch(1)
        val queued = kotlinx.coroutines.CompletableDeferred<Unit>()
        val checks = java.util.concurrent.atomic.AtomicInteger()
        val waitingThread = java.util.concurrent.atomic.AtomicReference<Thread>()
        var owner: Thread? = null
        var commit: kotlinx.coroutines.Job? = null
        try {
            store.save(host); store.saveProjection(host, before)
            val snapshot = requireNotNull(store.loadResumeSnapshot())
            val file = java.io.File(context.noBackupFilesDir, "paired-host.$namespace")
            val saved = file.readBytes()
            owner = Thread {
                synchronized(lock) {
                    held.countDown()
                    check(release.await(5, java.util.concurrent.TimeUnit.SECONDS))
                }
            }.apply { start() }
            assertTrue(held.await(5, java.util.concurrent.TimeUnit.SECONDS))
            commit = launch(kotlinx.coroutines.Dispatchers.IO) {
                val commitContext = kotlin.coroutines.coroutineContext
                commitContext.ensureActive()
                waitingThread.set(Thread.currentThread())
                queued.complete(Unit)
                store.commitResume(snapshot, next) {
                    checks.incrementAndGet()
                    commitContext.ensureActive()
                }
            }
            kotlinx.coroutines.withTimeout(5000) { queued.await() }
            kotlinx.coroutines.withTimeout(5000) {
                while (waitingThread.get()?.state != Thread.State.BLOCKED) kotlinx.coroutines.delay(10)
            }
            commit.cancel()
            release.countDown()
            kotlinx.coroutines.withTimeout(5000) { commit.join() }
            owner.join(5000); assertFalse(owner.isAlive)
            assertTrue(commit.isCancelled); assertEquals(1, checks.get())
            assertArrayEquals(saved, file.readBytes()); assertEquals(before, store.loadState()?.projection)
            val fresh = requireNotNull(store.loadResumeSnapshot())
            var finalChecks = 0
            val cancelledAtWrite = runCatching {
                store.commitResume(fresh, next) {
                    if (++finalChecks == 2) throw kotlinx.coroutines.CancellationException("fixture commit cancelled")
                }
            }.exceptionOrNull()
            assertTrue(cancelledAtWrite is kotlinx.coroutines.CancellationException)
            assertEquals(2, finalChecks); assertArrayEquals(saved, file.readBytes())
            assertEquals(before, store.loadState()?.projection)
            for (cancelAtRead in 1..2) {
                var active = true; var reads = 0
                val cancellation = kotlinx.coroutines.CancellationException("fixture identity read cancelled")
                assertSame(cancellation, runCatching {
                    store.commitResume(fresh, next) {
                        AuthenticationProof.current(host.deviceId, {
                            if (++reads == cancelAtRead) active = false
                            host.deviceId
                        }, { if (!active) throw cancellation })
                    }
                }.exceptionOrNull())
                assertEquals(cancelAtRead, reads)
                assertArrayEquals(saved, file.readBytes()); assertEquals(before, store.loadState()?.projection)
            }
            var successfulChecks = 0
            assertTrue(store.commitResume(fresh, next) { successfulChecks++ })
            assertEquals(2, successfulChecks); assertEquals(next, store.loadState()?.projection)
            InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", "resume_storage_wait_cancellation_fenced_verified")
            })
        } finally {
            release.countDown()
            commit?.cancel()
            kotlinx.coroutines.withContext(kotlinx.coroutines.NonCancellable) { commit?.join() }
            owner?.join(5000)
            store.wipe()
        }
    }
    @Test fun sameIdentityRePairKeepsEncryptedDisplayButFencesOldEndpointCommits() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "repair-address-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val other = PairedHostStore(context, namespace)
        val host = PairedHost("192.168.1.20", 7777, "a".repeat(64), "b".repeat(64))
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session")), "project", "session", "idle",
            listOf(RemoteTurn("turn", "assistant", "saved display")), "42", false, "7")
        try {
            store.save(host); store.saveProjection(host, projection)
            val old = requireNotNull(store.loadResumeSnapshot())
            val moved = host.copy(host = "192.168.2.25", port = 8888)
            assertEquals(SavedHostState(moved, projection), other.save(moved))
            assertEquals(SavedHostState(moved, projection), store.loadState())
            assertFalse(store.commitResume(old, projection))
            assertTrue(runCatching { store.saveProjection(host, projection) }.isFailure)
            val command = WorkspaceCommand.create(projection.copy(projects = projection.projects.map { it.copy(workspaceControl = true) },
                workspaceOwner = "c".repeat(64), workspaceRevision = "3"), "project", "workspace_select")
            store.beginWorkspace(moved, command)
            val file = java.io.File(context.noBackupFilesDir, "paired-host.$namespace")
            val bytes = file.readBytes()
            assertTrue(runCatching { other.save(host) }.isFailure)
            assertArrayEquals(bytes, file.readBytes())
            store.finishWorkspace(moved, command)
            val sameEndpoint = requireNotNull(store.loadResumeSnapshot())
            other.save(moved)
            assertFalse(store.commitResume(sameEndpoint, projection))
            store.beginRevoke(moved, java.util.UUID.randomUUID().toString())
            val revokeBytes = file.readBytes()
            assertTrue(runCatching { other.save(host) }.isFailure)
            assertArrayEquals(revokeBytes, file.readBytes())
            store.wipe(); store.save(moved); store.saveProjection(moved, projection)
            other.save(moved.copy(fingerprint = "e".repeat(64)))
            assertNull(store.loadState()?.projection)
        } finally { store.wipe() }
    }
    @Test fun backgroundCommitCannotOverwriteForegroundPendingOrRepairedHost() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "resume-cas-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val other = PairedHostStore(context, namespace)
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), "b".repeat(64))
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session")), "project", "session", "idle",
            listOf(RemoteTurn("turn", "assistant", "old")), "42", false, "7")
        val newer = projection.copy(turns = listOf(RemoteTurn("turn2", "assistant", "new")), journalSequence = "43", eventSequence = "8")
        try {
            store.save(host); store.saveProjection(host, projection)
            val before = requireNotNull(store.loadResumeSnapshot())
            other.saveProjection(host, newer)
            assertFalse(store.commitResume(before, projection))
            assertEquals(newer, store.loadState()?.projection)
            val fresh = requireNotNull(store.loadResumeSnapshot())
            assertTrue(store.commitResume(fresh, newer))
            assertFalse(store.commitResume(fresh, projection))
            assertTrue(runCatching { other.saveProjection(host, projection) }.isFailure)
            assertEquals(newer, store.loadState()?.projection)
            val sessionRead = requireNotNull(store.loadResumeSnapshot())
            val replacement = newer.copy(projects = listOf(RemoteProject("project", "Workspace", "replacement")),
                sessionId = "replacement", turns = emptyList(), journalSequence = "0", eventSequence = "0")
            assertTrue(store.commitResume(sessionRead, replacement))
            assertEquals(replacement, other.loadState()?.projection)
            assertFalse(store.commitResume(sessionRead, newer))
            val racedSessionRead = requireNotNull(store.loadResumeSnapshot())
            other.saveProjection(host, newer)
            assertFalse(store.commitResume(racedSessionRead, replacement))
            assertEquals(newer, store.loadState()?.projection)
            val pendingSnapshot = requireNotNull(store.loadResumeSnapshot())
            val command = WorkspaceCommand.create(newer.copy(projects = newer.projects.map { it.copy(workspaceControl = true) },
                workspaceOwner = "c".repeat(64), workspaceRevision = "3"), "project", "workspace_select")
            other.beginWorkspace(host, command)
            assertNull(store.loadResumeSnapshot())
            assertFalse(store.commitResume(pendingSnapshot, projection))
            assertEquals(command.envelope, store.loadState()?.pendingWorkspace?.envelope)
            other.finishWorkspace(host, command)
            val priorHost = requireNotNull(store.loadResumeSnapshot())
            other.save(host.copy(port = 8888))
            assertFalse(store.commitResume(priorHost, projection))
            val priorWipe = requireNotNull(store.loadResumeSnapshot())
            other.wipe()
            assertFalse(store.commitResume(priorWipe, projection))
            assertNull(store.loadState())
            store.save(host); store.saveProjection(host, projection)
            val base = java.io.File(context.noBackupFilesDir, "paired-host.$namespace")
            assertTrue(base.renameTo(java.io.File(base.path + ".bak")))
            assertEquals(projection, store.loadState()?.projection)
            assertTrue(base.exists())
        } finally { store.wipe() }
    }

    @Test fun backgroundIdentityLookupDoesNotCreateAReplacementKey() {
        val alias = "zenpi.resume-missing.${System.nanoTime()}"
        assertNull(DeviceIdentity.loadExisting(alias))
        val keys = java.security.KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        assertFalse(keys.containsAlias(alias))
        val identity = DeviceIdentity.loadOrCreate(alias)
        val proof = PairingIdentityProof.capture { identity.publicKey }
        try {
            assertEquals(identity.deviceId, proof.deviceId)
            assertEquals(identity.deviceId, DeviceIdentity.loadExisting(alias)?.deviceId)
            proof.requireCurrent { identity.publicKey }
        }
        finally { identity.wipe() }
        assertNull(DeviceIdentity.loadExisting(alias))
        assertTrue(runCatching { proof.requireCurrent { identity.publicKey } }.exceptionOrNull() is HostAuthenticationException)
        val replacement = DeviceIdentity.loadOrCreate(alias)
        try {
            assertNotEquals(proof.deviceId, replacement.deviceId)
            assertTrue(runCatching { proof.requireCurrent { replacement.publicKey } }.exceptionOrNull() is HostAuthenticationException)
        } finally { replacement.wipe() }
    }

    @Test fun lostSigningIdentityCannotCommitACompletedBackgroundRead() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "resume-key-fence-${System.nanoTime()}"
        val alias = "zenpi.resume-fence.${System.nanoTime()}"
        val identity = DeviceIdentity.loadOrCreate(alias)
        val store = PairedHostStore(context, namespace)
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), identity.deviceId)
        val before = HostProjection(listOf(RemoteProject("project", "Workspace", "session")), "project", "session", "idle",
            listOf(RemoteTurn("turn", "assistant", "old")), "1", false, "1")
        val next = before.copy(turns = listOf(RemoteTurn("new", "assistant", "new")), journalSequence = "2", eventSequence = "2")
        try {
            store.save(host); store.saveProjection(host, before)
            val snapshot = requireNotNull(store.loadResumeSnapshot())
            val file = java.io.File(context.noBackupFilesDir, "paired-host.$namespace")
            val saved = file.readBytes()
            identity.wipe()
            fun refuseCommit() {
                var committed = false
                val error = runCatching {
                    AuthenticationProof.current(host.deviceId, { identity.deviceId }, {})
                    committed = store.commitResume(snapshot, next)
                }.exceptionOrNull()
                assertTrue(error is HostAuthenticationException); assertFalse(committed)
                assertArrayEquals(saved, file.readBytes()); assertEquals(before, store.loadState()?.projection)
            }
            refuseCommit()
            val replacement = DeviceIdentity.loadOrCreate(alias)
            try { assertNotEquals(host.deviceId, replacement.deviceId); refuseCommit() }
            finally { replacement.wipe() }
        } finally { identity.wipe(); store.wipe() }
    }

    @Test fun staleProjectionRefusesAllCommitPathsAndKeepsUnresolvedRequest() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "projection-fence-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), "b".repeat(64))
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session")), "project", "session", "idle",
            listOf(RemoteTurn("turn", "assistant", "saved")), "42", false, "7")
        val stale = projection.copy(journalSequence = "41", eventSequence = "8")
        try {
            store.save(host); store.saveProjection(host, projection)
            val file = java.io.File(context.noBackupFilesDir, "paired-host.$namespace")
            val originalBytes = file.readBytes()
            val snapshot = requireNotNull(store.loadResumeSnapshot())
            val refused = listOf(stale, projection.copy(journalSequence = "43", eventSequence = null))
            for (candidate in refused) {
                assertTrue(runCatching { store.saveProjection(host, candidate) }.isFailure)
                assertArrayEquals(originalBytes, file.readBytes())
                assertTrue(runCatching { store.commitResume(snapshot, candidate) }.isFailure)
                assertArrayEquals(originalBytes, file.readBytes())
            }
            val command = WorkspaceCommand.create(projection.copy(projects = projection.projects.map { it.copy(workspaceControl = true) },
                workspaceOwner = "c".repeat(64), workspaceRevision = "3"), "project", "workspace_select")
            store.beginWorkspace(host, command)
            val pendingBytes = file.readBytes()
            for (candidate in refused) {
                assertTrue(runCatching { store.finishWorkspace(host, command, candidate) }.isFailure)
                assertArrayEquals(pendingBytes, file.readBytes())
                assertEquals(command.envelope, store.loadState()?.pendingWorkspace?.envelope)
                assertEquals(projection, store.loadState()?.projection)
            }
            store.finishWorkspace(host, command)
            // Display replacement can be shorter without regressing its transport cursor.
            val shorter = projection.copy(turns = emptyList(), journalSequence = "43")
            store.saveProjection(host, shorter)
            assertEquals(shorter, store.loadState()?.projection)
            assertTrue(runCatching { store.saveProjection(host, projection) }.isFailure)
            assertEquals(shorter, store.loadState()?.projection)
            val unreadable = shorter.copy(turns = listOf(RemoteTurn("invalid-scalar", "assistant", "\uD800")))
            val protectedBytes = file.readBytes()
            assertTrue(runCatching { store.saveProjection(host, unreadable) }.isFailure)
            assertArrayEquals(protectedBytes, file.readBytes())
            assertEquals(shorter, store.loadState()?.projection)
            val projects = (0 until 64).map { index ->
                RemoteProject("project-$index", "Workspace", "session-$index", promptStatuses = (0 until 9).map { status ->
                    RemotePromptStatus("request-$index-$status", "project-$index", "session-$index", "failed", failureCode = "internal_error")
                })
            }
            val capacity = shorter.copy(projects = projects, selectedId = projects.last().id, sessionId = projects.last().sessionId,
                turns = (0 until 64).map { RemoteTurn("turn-$it", "assistant", "answer") })
            store.saveProjection(host, capacity)
            val restored = requireNotNull(PairedHostStore(context, namespace).loadState()?.projection)
            assertEquals(256, restored.projects.sumOf { it.promptStatuses.size })
            assertEquals(projects.last().promptStatuses, restored.projects.last().promptStatuses)
            assertEquals(capacity.turns, restored.turns); assertEquals(capacity.eventSequence, restored.eventSequence)
            val large = capacity.copy(projects = projects.take(16).map { project -> project.copy(promptStatuses = (0 until 16).map {
                RemotePromptStatus("request-$it-" + "x".repeat(100), project.id, project.sessionId, "failed", failureCode = "internal_error")
            }) }, selectedId = projects[15].id, sessionId = projects[15].sessionId,
                turns = (0 until 8).map { RemoteTurn("large-$it", "assistant", "x".repeat(12 * 1024 - 128)) })
            store.saveProjection(host, large)
            val byteBounded = requireNotNull(PairedHostStore(context, namespace).loadState()?.projection)
            assertTrue(byteBounded.projects.sumOf { it.promptStatuses.size } < 256)
            assertEquals(large.projects.last().promptStatuses, byteBounded.projects.last().promptStatuses)
            assertEquals(large.turns, byteBounded.turns); assertEquals(large.eventSequence, byteBounded.eventSequence)
        } finally { store.wipe() }
    }
    @Test fun tabOnlyUpdateFencesOlderReadsAcrossEveryEncryptedCommitPath() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "tab-fence-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), "b".repeat(64))
        val tab = "d".repeat(64)
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session")), "project", "session", "idle",
            emptyList(), "42", false, "7", subtabs = RemoteSubtabs("project", "9", tab,
                listOf(RemoteSubtab(tab, "new name", "main", 1)), false))
        val stale = projection.copy(subtabs = projection.subtabs?.copy(revision = "8", tabs = listOf(RemoteSubtab(tab, "old name", "main", 1))))
        try {
            store.save(host); store.saveProjection(host, projection)
            val file = java.io.File(context.noBackupFilesDir, "paired-host.$namespace")
            val before = file.readBytes()
            assertTrue(runCatching { store.saveProjection(host, stale) }.isFailure)
            assertArrayEquals(before, file.readBytes())
            val snapshot = requireNotNull(store.loadResumeSnapshot())
            assertTrue(runCatching { store.commitResume(snapshot, stale) }.isFailure)
            assertArrayEquals(before, file.readBytes())
            val command = WorkspaceCommand.create(projection.copy(projects = projection.projects.map { it.copy(workspaceControl = true) },
                workspaceOwner = "c".repeat(64), workspaceRevision = "3"), "project", "workspace_select")
            store.beginWorkspace(host, command)
            val pending = file.readBytes()
            assertTrue(runCatching { store.finishWorkspace(host, command, stale) }.isFailure)
            assertArrayEquals(pending, file.readBytes())
            assertEquals(command.envelope, store.loadState()?.pendingWorkspace?.envelope)
            assertEquals(projection, store.loadState()?.projection)
            store.finishWorkspace(host, command)
            val newer = projection.copy(subtabs = projection.subtabs?.copy(revision = "10"))
            store.saveProjection(host, newer)
            assertEquals(newer, store.loadState()?.projection)
        } finally { store.wipe() }
    }
}
