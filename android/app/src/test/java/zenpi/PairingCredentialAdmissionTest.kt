package zenpi

import kotlinx.coroutines.*
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.PairingCredentialAdmission
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicLong
import zenpi.pairing.PairingInvite
import zenpi.pairing.PairingException

class PairingCredentialAdmissionTest {
    @Test fun monotonicExpiryDuringStorageWaitRefusesCredentialsDespiteWallRollback(): Unit = runBlocking {
        val storage = Mutex(locked = true)
        val elapsed = AtomicLong(0)
        val token = java.util.Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32))
        val invite = PairingInvite.parse("zenpi://pair?v=1&address=192.168.50.55%3A7777&fingerprint=${"a".repeat(64)}&token=$token&expires=1060", 1000, elapsed::get)
        var calls = 0
        val waiter = async(start = CoroutineStart.UNDISPATCHED) {
            runCatching { PairingCredentialAdmission.load(storage, { true }, { invite.requireFresh(990) }) { calls++ } }.exceptionOrNull()
        }
        elapsed.set(60_000_000_000); storage.unlock()
        assertTrue(withTimeout(5000) { waiter.await() } is PairingException)
        assertEquals(0, calls)
    }
    @Test fun invitationExpiringDuringStorageWaitNeverCreatesCredentials(): Unit = runBlocking {
        val storage = Mutex(locked = true)
        val clock = AtomicLong(1000)
        val token = java.util.Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32))
        val invite = PairingInvite.parse("zenpi://pair?v=1&address=192.168.50.55%3A7777&fingerprint=${"a".repeat(64)}&token=$token&expires=1100", now = clock.get()) { 0 }
        var calls = 0
        val waiter = async(start = CoroutineStart.UNDISPATCHED) {
            runCatching { PairingCredentialAdmission.load(storage, { true }, { invite.requireFresh(clock.get()) }) { calls++ } }.exceptionOrNull()
        }
        clock.set(1100)
        storage.unlock()
        assertTrue(withTimeout(5000) { waiter.await() } is PairingException)
        assertEquals(0, calls)
        clock.set(1099)
        assertTrue(runCatching { PairingCredentialAdmission.load(storage, { true }, { invite.requireFresh(clock.get()) }) { calls++ } }.isFailure)
        assertEquals(0, calls)
    }

    @Test fun cancelledBlockingCreationFinishesBeforeResetCanWipe(): Unit = runBlocking {
        val storage = Mutex()
        val entered = CountDownLatch(1); val release = CountDownLatch(1)
        val present = AtomicBoolean(false); val published = AtomicBoolean(false)
        val resetAttempted = CompletableDeferred<Unit>(); val resetFinished = CompletableDeferred<Unit>()
        val creator = launch {
            PairingCredentialAdmission.load(storage, { true }) {
                entered.countDown()
                check(release.await(5, TimeUnit.SECONDS))
                present.set(true)
            }
            published.set(true)
        }
        var reset: Job? = null
        try {
            assertTrue(withContext(Dispatchers.IO) { entered.await(5, TimeUnit.SECONDS) })
            creator.cancel()
            reset = launch {
                resetAttempted.complete(Unit)
                storage.withLock { present.set(false) }
                resetFinished.complete(Unit)
            }
            resetAttempted.await()
            assertNull(withTimeoutOrNull(100) { resetFinished.await() })
            release.countDown()
            withTimeout(5000) { creator.join(); resetFinished.await() }
            assertFalse(present.get()); assertFalse(published.get())
        } finally {
            release.countDown()
            withContext(NonCancellable) { creator.cancelAndJoin(); reset?.cancelAndJoin() }
        }
    }

    @Test fun replacedGenerationNeverStartsKeyCreation(): Unit = runBlocking {
        var calls = 0
        val failure = runCatching { PairingCredentialAdmission.load(Mutex(), { false }) { calls++ } }.exceptionOrNull()
        assertTrue(failure is CancellationException); assertEquals(0, calls)
    }

    @Test fun cancelledWaiterNeverCreatesAfterResetReleasesLock(): Unit = runBlocking {
        val storage = Mutex(locked = true)
        var calls = 0
        val waiter = launch(start = CoroutineStart.UNDISPATCHED) {
            PairingCredentialAdmission.load(storage, { true }) { calls++ }
        }
        waiter.cancelAndJoin()
        storage.unlock()
        assertEquals(0, calls)
        assertEquals("existing", PairingCredentialAdmission.load(storage, { true }) { "existing" })
    }

    @Test fun generationIsCheckedAfterStorageWaitNotBeforeIt(): Unit = runBlocking {
        val storage = Mutex(locked = true)
        val current = AtomicBoolean(true)
        var calls = 0
        val waiter = async(start = CoroutineStart.UNDISPATCHED) {
            runCatching { PairingCredentialAdmission.load(storage, current::get) { calls++ } }.exceptionOrNull()
        }
        current.set(false)
        storage.unlock()
        assertTrue(withTimeout(5000) { waiter.await() } is CancellationException)
        assertEquals(0, calls)
    }
}
