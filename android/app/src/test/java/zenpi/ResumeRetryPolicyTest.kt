package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.lifecycle.ResumeRetryPolicy
import zenpi.transport.HostRefusedException
import zenpi.transport.WireException

class ResumeRetryPolicyTest {
    @Test fun retryBudgetIncludesOnlyTwoRetriesAndNeverConsumesCancellation() {
        for (attempt in 0..5) {
            assertEquals(attempt < 2, ResumeRetryPolicy.retry(java.net.SocketTimeoutException("deadline"), attempt))
            assertFalse(ResumeRetryPolicy.retry(zenpi.transport.HostAuthenticationException(), attempt))
            val cancelled = kotlinx.coroutines.CancellationException("WorkManager stopped")
            assertSame(cancelled, runCatching { ResumeRetryPolicy.retry(cancelled, attempt) }.exceptionOrNull())
        }
    }

    @Test fun ownResumeDeadlineRetriesButParentDeadlineEscapesWithoutRetryOrCommit() = kotlinx.coroutines.runBlocking {
        var commits = 0
        val own = runCatching {
            zenpi.transport.OperationDeadline.run(20) { kotlinx.coroutines.awaitCancellation() }
            commits++
        }.exceptionOrNull() as Exception
        assertTrue(own is java.net.SocketTimeoutException)
        assertTrue(ResumeRetryPolicy.retry(own, 0)); assertFalse(ResumeRetryPolicy.retry(own, 2))
        var retries = 0
        val parent = runCatching {
            kotlinx.coroutines.withTimeout(20) {
                try {
                    zenpi.transport.OperationDeadline.run(20_000) { kotlinx.coroutines.awaitCancellation() }
                    commits++
                } catch (error: Exception) {
                    if (ResumeRetryPolicy.retry(error, 0)) retries++
                    Unit
                }
            }
        }.exceptionOrNull()
        assertTrue(parent is kotlinx.coroutines.TimeoutCancellationException)
        assertEquals(0, commits); assertEquals(0, retries)
    }

    @Test fun authorityRefusalsAreTerminalButTransientFailuresRetry() {
        for (code in listOf("unauthorized", "revoked_device", "owner_mismatch"))
            assertTrue(code, ResumeRetryPolicy.permanent(HostRefusedException(code)))
        for (code in listOf("busy", "invalid_request", "revision_conflict", "resource_limit"))
            assertFalse(code, ResumeRetryPolicy.permanent(HostRefusedException(code)))
        assertFalse(ResumeRetryPolicy.permanent(WireException("Host request refused: unauthorized")))
        assertFalse(ResumeRetryPolicy.permanent(java.net.SocketTimeoutException("unauthorized")))
        assertTrue(ResumeRetryPolicy.permanent(zenpi.transport.SelectedProjectUnavailableException("removed")))
        assertFalse(ResumeRetryPolicy.permanent(WireException("Selected project unavailable")))
    }
}
