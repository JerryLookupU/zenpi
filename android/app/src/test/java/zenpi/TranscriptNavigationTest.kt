package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.pairing.*

class TranscriptNavigationTest {
    private fun page(index: Int) = TranscriptPage("project", "session", "turn", "assistant", "a".repeat(64),
        index * 8192, (index + 1) * 8192, 8192 * 20, index < 19, "x".repeat(8192))
    @Test fun malformedRetainedMetadataCannotAuthorizeNavigationEvenWhenWholeChainAgrees() {
        val changes = listOf<(TranscriptPage) -> TranscriptPage>(
            { it.copy(projectId = "") }, { it.copy(sessionId = "bad\n") }, { it.copy(turnId = "x".repeat(129)) },
            { it.copy(role = "unknown") }, { it.copy(workerRequestId = "worker") },
            { it.copy(subtabId = "a".repeat(64)) }, { it.copy(workerRequestId = "worker", subtabId = "invalid") },
            { it.copy(toolCallId = "call") }, { it.copy(toolName = "read_file") },
            { it.copy(toolCallId = "call", toolName = "x".repeat(65)) },
            { it.copy(role = "user", toolCallId = "call", toolName = "read_file") },
            { it.copy(journalRevision = "01") }, { it.copy(journalRevision = "18446744073709551616") },
            { it.copy(workerRequestId = "worker", subtabId = "a".repeat(64), journalRevision = "1") })
        for (change in changes) {
            val first = change(page(0)); val second = change(page(1))
            assertFalse(TranscriptNavigation.validChain(listOf(first), second))
            assertTrue(runCatching { TranscriptNavigation.remember(emptyList(), null, first) }.isFailure)
            assertTrue(runCatching { TranscriptNavigation.previous(listOf(first), second) }.isFailure)
        }
        val worker = page(0).copy(totalBytes = 65536, workerRequestId = "worker", subtabId = "a".repeat(64), toolCallId = "call", toolName = "read_file")
        assertTrue(TranscriptNavigation.validChain(emptyList(), worker))
        assertTrue(TranscriptNavigation.validChain(emptyList(), page(0).copy(journalRevision = "18446744073709551615")))
    }
    @Test fun previousHistoryIsBoundedAndCannotAccumulateAWholeLargeMessage() {
        var previous = emptyList<TranscriptPage>()
        for (index in 1..19) previous = TranscriptNavigation.remember(previous, page(index - 1), page(index))
        assertEquals(7, previous.size); assertEquals((12..18).map { it * 8192 }, previous.map { it.offset })
        assertEquals(57_344, previous.sumOf { it.text.toByteArray().size })
        var current = page(19)
        repeat(7) {
            current = requireNotNull(TranscriptNavigation.previous(previous, current)); previous = previous.dropLast(1)
        }
        assertEquals(page(12), current); assertNull(TranscriptNavigation.previous(previous, current))
    }
    @Test fun backThenForwardRevalidatesFromPriorDigestWithoutDuplicateHistory() {
        var previous = TranscriptNavigation.remember(emptyList(), page(0), page(1))
        previous = TranscriptNavigation.remember(previous, page(1), page(2))
        val back = requireNotNull(TranscriptNavigation.previous(previous, page(2)))
        previous = previous.dropLast(1)
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session")), "project", "session", "idle",
            listOf(RemoteTurn("turn", "assistant", "prefix", true)), "1", true, "1")
        val request = TranscriptPageCodec.request(projection, "turn", back)
        val args = request["payload"].asJsonObject["arguments"].asJsonObject
        assertEquals(16_384, args["offset"].asInt); assertEquals(back.digest, args["digest"].asString)
        previous = TranscriptNavigation.remember(previous, back, page(2))
        assertEquals(listOf(page(0), page(1)), previous)
        assertEquals(emptyList<TranscriptPage>(), TranscriptNavigation.remember(previous, null, page(0)))
    }
    @Test fun foreignDigestScopeRoleSizeAndNoncontiguousPagesCannotNavigate() {
        for (bad in listOf(page(1).copy(projectId = "foreign"), page(1).copy(sessionId = "foreign"), page(1).copy(turnId = "foreign"),
            page(1).copy(role = "user"), page(1).copy(digest = "b".repeat(64)), page(1).copy(totalBytes = 1), page(2))) {
            assertTrue(runCatching { TranscriptNavigation.remember(emptyList(), page(0), bad) }.isFailure)
            assertTrue(runCatching { TranscriptNavigation.previous(listOf(page(0)), bad) }.isFailure)
        }
        assertTrue(runCatching { TranscriptNavigation.remember(emptyList(), page(0).copy(hasMore = false), page(1)) }.isFailure)
    }
    @Test fun authorityLossClearsEveryRememberedPageWithoutChangingDurableCursor() {
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session")), "project", "session", "idle",
            listOf(RemoteTurn("turn", "assistant", "prefix", true)), "42", true, "7")
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, transcriptTurn = "turn",
            transcriptPage = page(2), transcriptPreviousPages = listOf(page(0), page(1)))
        val next = AuthorityLossPolicy.displayOnly(state)
        assertTrue(next.transcriptPreviousPages.isEmpty()); assertNull(next.transcriptPage); assertNull(next.transcriptTurn)
        assertEquals("42", next.projection?.journalSequence); assertEquals("7", next.projection?.eventSequence)
    }
    @Test fun entireHistoryChainMustBeContiguousAndContinuationBearing() {
        for (broken in listOf(listOf(page(0).copy(digest = "foreign"), page(1)),
            listOf(page(0).copy(hasMore = false), page(1)), listOf(page(1), page(0), page(1)),
            listOf(page(0), page(0), page(1)), (0..8).map(::page))) {
            assertTrue(runCatching { TranscriptNavigation.previous(broken, page(2)) }.isFailure)
            assertTrue(runCatching { TranscriptNavigation.remember(broken, page(2), page(3)) }.isFailure)
        }
        assertTrue(runCatching { TranscriptNavigation.remember(emptyList(), null, page(1)) }.isFailure)
        assertTrue(runCatching { TranscriptNavigation.previous(listOf(page(0).copy(hasMore = false)), page(1)) }.isFailure)
    }
}
