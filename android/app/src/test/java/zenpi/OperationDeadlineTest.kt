package zenpi

import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.OperationDeadline
import zenpi.transport.SocketLease
import zenpi.transport.HostProjection
import zenpi.transport.RemoteProject
import zenpi.transport.WorkspaceCommand
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.pairing.WorkspaceDispatchPolicy
import java.net.Socket
import java.net.SocketTimeoutException

class OperationDeadlineTest {
    @Test fun ownDeadlineIsFailureNotUserCancellationAndClosesOwnedSocket() = runBlocking {
        val lease = SocketLease()
        val socket = Socket()
        lease.install(socket)
        var cleaned = false
        val error = runCatching {
            OperationDeadline.run(50) {
                try { awaitCancellation() }
                finally { lease.close(); cleaned = true }
            }
        }.exceptionOrNull()
        assertTrue(error is SocketTimeoutException)
        assertFalse(error is CancellationException)
        assertEquals("Host operation timed out", error?.message)
        assertTrue(cleaned); assertTrue(socket.isClosed)
    }
    @Test fun parentDeadlineStaysCancellationRatherThanBecomingAnOperationFailure() = runBlocking {
        var cleaned = false
        val error = runCatching {
            withTimeout(50) {
                OperationDeadline.run(5_000) {
                    try { awaitCancellation() } finally { cleaned = true }
                }
            }
        }.exceptionOrNull()
        assertTrue(error is TimeoutCancellationException)
        assertFalse(error is SocketTimeoutException)
        assertTrue(cleaned)
    }
    @Test fun nestedDeadlineIsNotMisattributedToTheClientOperation() = runBlocking {
        val error = runCatching { OperationDeadline.run(5_000) { withTimeout(50) { awaitCancellation() } } }.exceptionOrNull()
        assertTrue(error is TimeoutCancellationException)
        assertFalse(error is SocketTimeoutException)
    }
    @Test fun explicitCancellationPropagatesWithoutRetryOrFailurePublication() = runBlocking {
        val entered = CompletableDeferred<Unit>()
        var runs = 0; var failures = 0; var cleaned = false
        val job = launch {
            try {
                OperationDeadline.run(5_000) {
                    runs++; entered.complete(Unit)
                    try { awaitCancellation() } finally { cleaned = true }
                }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (_: Exception) { failures++ }
        }
        entered.await(); job.cancelAndJoin()
        assertEquals(1, runs); assertEquals(0, failures); assertTrue(cleaned)
    }
    @Test fun lateBlockingResultCannotBecomeConfirmedAfterDeadline() = runBlocking {
        val released = java.util.concurrent.CountDownLatch(1)
        val entered = java.util.concurrent.CountDownLatch(1)
        val lease = SocketLease()
        val socket = object : Socket() {
            override fun close() { super.close(); released.countDown() }
        }
        lease.install(socket)
        var published = false
        val closer = launch(Dispatchers.Default) {
            check(entered.await(2, java.util.concurrent.TimeUnit.SECONDS))
            delay(100); lease.close()
        }
        try {
            val error = runCatching {
                OperationDeadline.run(50) {
                    withContext(Dispatchers.IO) {
                        entered.countDown()
                        check(released.await(2, java.util.concurrent.TimeUnit.SECONDS))
                        "late receipt"
                    }
                }
                published = true
            }.exceptionOrNull()
            assertTrue(error is SocketTimeoutException); assertFalse(published)
            assertTrue(socket.isClosed)
        } finally { lease.close(); closer.cancelAndJoin() }
    }
    @Test fun valuesNullableResultsAndOriginalErrorsRemainUnchanged() = runBlocking {
        assertEquals("confirmed", OperationDeadline.run { "confirmed" })
        assertNull(OperationDeadline.run<String?> { null })
        val error = IllegalStateException("fixture")
        val propagated = requireNotNull(runCatching { OperationDeadline.run { throw error } }.exceptionOrNull())
        // Coroutine debug recovery may copy exceptions while retaining the original cause.
        assertEquals(error.javaClass, propagated.javaClass); assertEquals(error.message, propagated.message)
        assertSame(error, generateSequence(propagated) { it.cause }.last())
        val cancelled = CancellationException("fixture cancellation")
        val cancellation = requireNotNull(runCatching { OperationDeadline.run { throw cancelled } }.exceptionOrNull())
        assertTrue(cancellation is CancellationException); assertEquals(cancelled.message, cancellation.message)
        assertSame(cancelled, generateSequence(cancellation) { it.cause }.last())
        assertTrue(runCatching { OperationDeadline.run(0) { "bad" } }.exceptionOrNull() is IllegalArgumentException)
    }
    @Test fun timedOutWriteIsReportedUnknownWithoutGeneratingAReplacementRequest() = runBlocking {
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", promptControl = true)),
            "project", "session", "idle", emptyList(), "1", false, "0", workspaceOwner = "a".repeat(64), workspaceRevision = "1")
        val command = WorkspaceCommand.prompt(projection, "hello")
        val savedId = command.envelope["id"].asString
        val submitted = mutableListOf<String>()
        var state = PairingState(phase = PairingPhase.CONNECTING, projection = projection, pendingWorkspace = command)
        var unknown: Boolean
        try {
            OperationDeadline.run(50) { submitted.add(savedId); awaitCancellation() }
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) { unknown = true; state = state.copy(phase = PairingPhase.ERROR) }
        assertTrue(unknown); assertEquals(command.envelope, state.pendingWorkspace?.envelope)
        assertEquals(listOf(savedId), submitted)
        val retry = WorkspaceCommand.restore(command.envelope)
        assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, retry))
        assertFalse(WorkspaceDispatchPolicy.allows(state, true, true, WorkspaceCommand.prompt(projection, "hello")))
        OperationDeadline.run { submitted.add(retry.envelope["id"].asString) }
        assertEquals(listOf(savedId, savedId), submitted)
    }
}
