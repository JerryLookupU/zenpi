package zenpi.transport

import kotlinx.coroutines.withTimeoutOrNull
import java.net.SocketTimeoutException

/** Only this operation's deadline is a transport failure; parent cancellation stays cancellation. */
internal object OperationDeadline {
    private class Completed<T>(val value: T)
    suspend fun <T> run(milliseconds: Long = 15_000, block: suspend () -> T): T {
        require(milliseconds > 0)
        val result = withTimeoutOrNull(milliseconds) { Completed(block()) }
            ?: throw SocketTimeoutException("Host operation timed out")
        return result.value
    }
}
