package zenpi

import kotlinx.coroutines.*
import kotlinx.coroutines.channels.Channel
import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.ownerCallback
import kotlin.coroutines.CoroutineContext

class OwnerCallbackTest {
    private class OwnerQueue : CoroutineDispatcher() {
        private val pending = java.util.concurrent.ConcurrentLinkedQueue<Runnable>()
        val ready = Channel<Unit>(Channel.UNLIMITED)
        override fun dispatch(context: CoroutineContext, block: Runnable) {
            pending.add(block); check(ready.trySend(Unit).isSuccess)
        }
        // Generous: a slow CI runner may take seconds to start an IO thread.
        suspend fun next() { withTimeout(10_000) { ready.receive() }; requireNotNull(pending.poll()).run() }
    }
    @Test fun ioCallerPublishesOnlyOnOwnerThreadAfterQueuedLocalEdit() = runBlocking {
        val owner = OwnerQueue()
        val thread = Thread.currentThread()
        var state = listOf("initial")
        val job = async(Dispatchers.IO) {
            ownerCallback(owner, { true }) {
                assertSame(thread, Thread.currentThread())
                state = state + "host update"
            }
        }
        state = state + "local draft"
        owner.next(); job.await()
        assertEquals(listOf("initial", "local draft", "host update"), state)
    }
    @Test fun replacedGenerationWhileQueuedNeverClosesOrPublishesIntoNewView() = runBlocking {
        for (replacement in listOf("background", "new scan", "project selection", "new command", "authority lost")) {
            val owner = OwnerQueue()
            var generation = 1
            var state: String
            var closes = 0
            val job = async(Dispatchers.IO) {
                ownerCallback(owner, { generation == 1 }) { closes++; state = "late old view" }
            }
            generation = 2; state = replacement
            owner.next()
            assertTrue(runCatching { job.await() }.exceptionOrNull() is CancellationException)
            assertEquals(replacement, state); assertEquals(0, closes)
        }
    }
    @Test fun callerCancellationBeforeOwnerRunsNeverPublishesOrExecutesStorage() = runBlocking {
        val owner = OwnerQueue()
        var commits = 0
        val job = async(Dispatchers.IO) { ownerCallback(owner, { true }) { commits++ } }
        withTimeout(10_000) { owner.ready.receive() }
        job.cancel()
        // Queue signal was consumed above; dispatch's runnable remains queued.
        owner.ready.trySend(Unit)
        owner.next(); job.join()
        assertEquals(0, commits)
    }
    @Test fun storageFailurePropagatesWithoutPublishingConfirmation() = runBlocking {
        supervisorScope {
            val owner = OwnerQueue()
            val failure = java.io.IOException("fixture cache failure")
            val persist: suspend () -> Unit = { withContext(Dispatchers.IO) { throw failure } }
            var confirmed = false
            val job = async(Dispatchers.IO) {
                ownerCallback(owner, { true }) {
                    persist()
                    confirmed = true
                }
            }
            owner.next(); owner.next()
            val propagated = requireNotNull(runCatching { job.await() }.exceptionOrNull())
            assertEquals(failure.javaClass, propagated.javaClass); assertEquals(failure.message, propagated.message)
            assertSame(failure, generateSequence(propagated) { it.cause }.last())
            assertFalse(confirmed)
        }
    }
    @Test fun replacedOwnerAfterSuspendedPersistenceCannotPublishOldReceipt() = runBlocking {
        val owner = OwnerQueue()
        val storageEntered = CompletableDeferred<Unit>()
        val storageDone = CompletableDeferred<Unit>()
        var generation = 1
        var confirmed = false
        val job = async(Dispatchers.IO) {
            ownerCallback(owner, { generation == 1 }) {
                withContext(Dispatchers.IO) { storageEntered.complete(Unit); storageDone.await() }
                if (generation != 1) throw CancellationException("Host callback replaced")
                confirmed = true
            }
        }
        owner.next(); storageEntered.await()
        generation = 2; storageDone.complete(Unit)
        owner.next()
        assertTrue(runCatching { job.await() }.exceptionOrNull() is CancellationException)
        assertFalse(confirmed)
    }
}
