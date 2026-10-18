package zenpi

import kotlinx.coroutines.CancellationException
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import java.util.Base64
import zenpi.pairing.PairingInvite

class PairingIdentityProofTest {
    private val public = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(65).apply { this[0] = 4 })
    private fun invite(monotonic: () -> Long = { 0 }) = PairingInvite.parse(
        "zenpi://pair?v=1&address=192.168.1.20%3A7777&fingerprint=${"a".repeat(64)}&token=${"A".repeat(43)}&expires=1060",
        1000, monotonic)

    @Test fun preconnectAdmissionRetainsFreshInvitationAndCapturedIdentity() {
        val proof = PairingIdentityProof.capture { public }
        var checks = 0; var reads = 0
        proof.requireFresh(invite(), { reads++; public }, { checks++ }, { 1000 })
        assertEquals(1, reads); assertEquals(2, checks)
        val changed = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(65).apply { this[0] = 4; this[1] = 1 })
        assertTrue(runCatching { proof.requireFresh(invite(), { changed }, {}, { 1000 }) }
            .exceptionOrNull() is HostAuthenticationException)
    }

    @Test fun expiryBeforeOrDuringKeystoreReadCannotAdmitTheNextConnectionStage() {
        val proof = PairingIdentityProof.capture { public }
        var reads = 0
        assertTrue(runCatching { proof.requireFresh(invite(), { reads++; public }, {}, { 1060 }) }.isFailure)
        assertEquals(0, reads)
        for (monotonicExpiry in listOf(false, true)) {
            var elapsed = 0L; var wall = 1000L; var admitted = false
            val captured = invite { elapsed }
            assertTrue(runCatching {
                proof.requireFresh(captured, {
                    if (monotonicExpiry) { elapsed = 60_000_000_000; wall = 990 } else wall = 1060
                    public
                }, {}, { wall })
                admitted = true
            }.isFailure)
            assertFalse(admitted)
        }
    }

    @Test fun cancellationDuringKeystoreReadCannotAdmitTheNextConnectionStage() {
        val proof = PairingIdentityProof.capture { public }
        val cancelled = CancellationException("pairing stopped")
        var active = true; var admitted = false
        assertSame(cancelled, runCatching {
            proof.requireFresh(invite(), { active = false; public }, { if (!active) throw cancelled }, { 1000 })
            admitted = true
        }.exceptionOrNull())
        assertFalse(admitted)
    }
    @Test fun capturedPublicKeyAndDeviceIdRemainBoundAcrossKeyDeletionAndReplacement() {
        var reads = 0
        val proof = PairingIdentityProof.capture { reads++; public }
        assertEquals(1, reads); assertEquals(public, proof.publicKey)
        proof.requireCurrent { public }
        val replacement = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(65).apply { this[0] = 4; this[1] = 1 })
        for (read in listOf<() -> String>({ replacement }, { throw java.security.KeyStoreException("PRIVATE_ALIAS") })) {
            val error = runCatching { proof.requireCurrent(read) }.exceptionOrNull()
            assertTrue(error is HostAuthenticationException); assertNull(error?.cause)
            assertFalse(error.toString().contains("PRIVATE"))
        }
    }
    @Test fun malformedPublicIdentityAndCancellationCannotBecomeSuccessfulAdmission() {
        for (value in listOf("", "invalid", public + "=", Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(65))))
            assertTrue(runCatching { PairingIdentityProof.capture { value } }.exceptionOrNull() is HostAuthenticationException)
        val cancelled = CancellationException("pairing stopped")
        assertSame(cancelled, runCatching { PairingIdentityProof.capture { throw cancelled } }.exceptionOrNull())
        assertSame(cancelled, runCatching { PairingIdentityProof.capture { public }.requireCurrent { throw cancelled } }.exceptionOrNull())
    }
}
