package zenpi

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.OwnedJobs

class OwnedJobsTest {
    @Test fun cancelledJobsRemainOwnedUntilTheirCleanupCompletes(): Unit = runBlocking {
        val owner = OwnedJobs()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val stopped = List(3) { CompletableDeferred<Unit>() }
        val release = List(3) { CompletableDeferred<Unit>() }
        val jobs = stopped.indices.map { index -> owner.launch(scope) {
            try { awaitCancellation() }
            finally { withContext(NonCancellable) { stopped[index].complete(Unit); release[index].await() } }
        } }
        var removed = false
        var barrier: Job? = null
        try {
            jobs[0].cancel()
            assertTrue(jobs[0].isCancelled); assertFalse(jobs[0].isCompleted)
            barrier = launch(start = CoroutineStart.UNDISPATCHED) { owner.cancelAndJoin(); removed = true }
            withTimeout(5000) { stopped.forEach { it.await() } }
            for (index in jobs.indices) {
                assertFalse(removed)
                release[index].complete(Unit)
                jobs[index].join()
                yield()
                if (index < jobs.lastIndex) assertFalse(removed)
            }
            withTimeout(5000) { barrier.join() }
            assertTrue(removed); assertTrue(jobs.all { it.isCompleted })
            owner.cancelAndJoin()
        } finally {
            release.forEach { it.complete(Unit) }
            withContext(NonCancellable) { jobs.forEach { it.cancelAndJoin() }; barrier?.cancelAndJoin() }
            scope.cancel()
        }
    }

    @Test fun immediatelyCompletedAndCancelledScopeJobsDoNotBlockReset(): Unit = runBlocking {
        val owner = OwnedJobs()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        try {
            repeat(100) { assertTrue(owner.launch(scope) { }.isCompleted) }
            scope.cancel()
            var ran = false
            assertTrue(owner.launch(scope) { ran = true }.isCompleted)
            assertFalse(ran)
            withTimeout(5000) { owner.cancelAndJoin() }
        } finally { scope.cancel() }
    }

    @Test fun cancelledResetWaitCannotClaimCleanupFinished(): Unit = runBlocking {
        val owner = OwnedJobs()
        val release = CompletableDeferred<Unit>()
        val stopped = CompletableDeferred<Unit>()
        val scope = CoroutineScope(SupervisorJob() + Dispatchers.Unconfined)
        val job = owner.launch(scope) {
            try { awaitCancellation() }
            finally { withContext(NonCancellable) { stopped.complete(Unit); release.await() } }
        }
        var removed = false
        val wait = launch(start = CoroutineStart.UNDISPATCHED) { owner.cancelAndJoin(); removed = true }
        try {
            stopped.await(); wait.cancelAndJoin()
            assertFalse(removed); assertFalse(job.isCompleted)
            release.complete(Unit)
            withTimeout(5000) { owner.cancelAndJoin() }
            assertTrue(job.isCompleted)
        } finally {
            release.complete(Unit)
            withContext(NonCancellable) { job.cancelAndJoin(); wait.cancelAndJoin() }
            scope.cancel()
        }
    }
}
