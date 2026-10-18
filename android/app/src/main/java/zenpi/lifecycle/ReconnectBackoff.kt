package zenpi.lifecycle

import kotlin.random.Random

/** Foreground read retries only; unresolved writes never enter this loop. */
internal class ReconnectBackoff(private val sample: (Long) -> Long = { Random.nextLong(it) }) {
    companion object { const val MIN_FAILURE_DELAY_MS = 250L }
    private var failures = 0
    fun failed() { failures = (failures + 1).coerceAtMost(7) }
    fun succeeded() { failures = 0 }
    fun delayMillis(hasMoreEvents: Boolean): Long {
        if (failures == 0) return if (hasMoreEvents) 200 else 2_000
        val ceiling = (1_000L shl (failures - 1)).coerceAtMost(60_000)
        val range = ceiling - MIN_FAILURE_DELAY_MS + 1
        return MIN_FAILURE_DELAY_MS + sample(range).also { require(it in 0 until range) }
    }
}
