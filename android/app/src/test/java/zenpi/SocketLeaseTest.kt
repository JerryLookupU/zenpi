package zenpi

import org.junit.Assert.*
import org.junit.Test
import kotlinx.coroutines.launch
import java.net.Socket
import java.net.SocketException
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import zenpi.transport.SocketLease

class SocketLeaseTest {
    private class Tracked(private val wrapped: Socket? = null, private val fail: Boolean = false) : Socket() {
        val closes = AtomicInteger()
        override fun close() { closes.incrementAndGet(); wrapped?.close(); if (fail) throw SocketException("fixture close") }
    }
    @Test fun cancellationBeforeInstallationClosesLateSocketAndCannotBeReopened() {
        val lease = SocketLease(); lease.close()
        repeat(2) {
            val raw = Tracked()
            assertTrue(runCatching { lease.install(raw) }.exceptionOrNull() is SocketException)
            assertEquals(1, raw.closes.get())
        }
    }
    @Test fun cancellationDuringTlsWrappingRefusesAndClosesLateWrapper() {
        val lease = SocketLease(); val raw = Tracked()
        lease.install(raw); lease.close()
        val tls = Tracked(raw)
        assertTrue(runCatching { lease.transfer(raw, tls) }.exceptionOrNull() is SocketException)
        assertEquals(1, tls.closes.get()); assertTrue(raw.closes.get() >= 1)
        lease.close(); assertEquals(1, tls.closes.get())
    }
    @Test fun successfulTransferDoesNotCloseRawUntilOwnedWrapperCloses() {
        val lease = SocketLease(); val raw = Tracked(); val tls = Tracked(raw)
        lease.install(raw); lease.transfer(raw, tls)
        assertEquals(0, raw.closes.get()); assertEquals(0, tls.closes.get())
        lease.close(); lease.close()
        assertEquals(1, tls.closes.get()); assertEquals(1, raw.closes.get())
    }
    @Test fun foreignTransferAndDoubleInstallCannotDiscardOwnedConnection() {
        val lease = SocketLease(); val raw = Tracked(); val foreign = Tracked(); val tls = Tracked()
        lease.install(raw)
        assertTrue(runCatching { lease.install(foreign) }.isFailure)
        assertEquals(1, foreign.closes.get()); assertEquals(0, raw.closes.get())
        assertTrue(runCatching { lease.transfer(foreign, tls) }.isFailure)
        assertEquals(1, tls.closes.get()); assertEquals(0, raw.closes.get())
        lease.close(); assertEquals(1, raw.closes.get())
    }
    @Test fun closeFailureIsTerminalAndLateRejectionRetainsDiagnostic() {
        val lease = SocketLease(); val raw = Tracked(fail = true)
        lease.install(raw); assertTrue(runCatching { lease.close() }.isFailure)
        lease.close(); assertEquals(1, raw.closes.get())
        val late = Tracked(fail = true)
        val error = runCatching { lease.install(late) }.exceptionOrNull()
        assertTrue(error is SocketException); assertEquals(1, error?.suppressed?.size)
    }
    @Test fun concurrentCloseAndTransferAlwaysCloseRawAndTlsWithoutLeakingOwnership() {
        repeat(100) {
            val lease = SocketLease(); val raw = Tracked(); val tls = Tracked(raw)
            lease.install(raw)
            val gate = CountDownLatch(1)
            val complete = CountDownLatch(2)
            val failures = java.util.concurrent.ConcurrentLinkedQueue<Throwable>()
            val closing = Thread {
                try { check(gate.await(2, TimeUnit.SECONDS)); lease.close() }
                catch (error: Throwable) { failures.add(error) } finally { complete.countDown() }
            }
            val transfer = Thread {
                try { check(gate.await(2, TimeUnit.SECONDS)); lease.transfer(raw, tls) }
                catch (_: SocketException) { /* Closing won the ownership race. */ }
                catch (error: Throwable) { failures.add(error) } finally { complete.countDown() }
            }
            closing.start(); transfer.start(); gate.countDown()
            val finished = complete.await(3, TimeUnit.SECONDS)
            closing.join(3000); transfer.join(3000)
            assertTrue(finished); assertFalse(closing.isAlive); assertFalse(transfer.isAlive)
            assertTrue(failures.toString(), failures.isEmpty())
            lease.close(); assertEquals(1, tls.closes.get()); assertTrue(raw.closes.get() >= 1)
        }
    }
    @Test fun terminalLeaseClosesRealUnconnectedSocketsWithoutPortBinding() {
        val lease = SocketLease(); val raw = Socket()
        lease.install(raw); lease.close(); assertTrue(raw.isClosed)
        val late = Socket()
        assertTrue(runCatching { lease.transfer(raw, late) }.isFailure)
        assertTrue(late.isClosed)
    }
    @Test fun cancelledCoroutineCannotPublishWrapperCreatedAfterClose() = kotlinx.coroutines.runBlocking {
        val lease = SocketLease(); val raw = Tracked(); val tls = Tracked(raw)
        lease.install(raw)
        val closer = launch(start = kotlinx.coroutines.CoroutineStart.UNDISPATCHED) {
            try { kotlinx.coroutines.awaitCancellation() } finally { lease.close() }
        }
        closer.cancel(); closer.join()
        assertTrue(runCatching { lease.transfer(raw, tls) }.isFailure)
        assertEquals(1, tls.closes.get()); assertTrue(raw.closes.get() >= 1)
    }
}
