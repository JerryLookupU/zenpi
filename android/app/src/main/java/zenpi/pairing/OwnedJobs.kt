package zenpi.pairing

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

internal class OwnedJobs {
    private val jobs = mutableSetOf<Job>()

    fun track(job: Job) {
        synchronized(jobs) { jobs.add(job) }
        job.invokeOnCompletion { synchronized(jobs) { jobs.remove(job) } }
    }

    fun launch(scope: CoroutineScope, block: suspend CoroutineScope.() -> Unit): Job {
        // Register before execution, including immediate completion or cancellation.
        val job = scope.launch(start = CoroutineStart.LAZY, block = block)
        track(job)
        job.start()
        return job
    }

    suspend fun cancelAndJoin() {
        val pending = synchronized(jobs) { jobs.toList() }
        pending.forEach { it.cancel() }
        pending.forEach { it.join() }
    }
}
