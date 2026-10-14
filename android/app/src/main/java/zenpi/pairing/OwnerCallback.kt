package zenpi.pairing

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** State transitions share the UI owner; check freshness after dispatch, not on the IO caller. */
internal suspend fun ownerCallback(dispatcher: CoroutineDispatcher = Dispatchers.Main.immediate,
    isCurrent: () -> Boolean, block: suspend () -> Unit) = withContext(dispatcher) {
    if (!isCurrent()) throw CancellationException("Host callback replaced")
    block()
}
