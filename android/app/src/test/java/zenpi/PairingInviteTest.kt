package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.PairingInvite
import java.security.MessageDigest
import java.util.Base64

class PairingInviteTest {
    private val fingerprint = "a".repeat(64)
    private val token = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32) { it.toByte() })
    private val uri = "zenpi://pair?v=1&address=192.168.50.55%3A7777&fingerprint=$fingerprint&token=$token&expires=1060"
    private fun refused(value: String, now: Long = 1000) { assertTrue(runCatching { PairingInvite.parse(value, now) { 0 } }.isFailure) }
    @Test fun roundtripSafetyCodeAndRedaction() {
        val invite = PairingInvite.parse(uri, 1000) { 0 }
        assertEquals("192.168.50.55", invite.host); assertEquals(7777, invite.port)
        val digest = MessageDigest.getInstance("SHA-256").digest("zenpi-pair-v1:$fingerprint:$token".toByteArray())
        val code = java.nio.ByteBuffer.wrap(digest).int.toLong() and 0xffff_ffffL
        assertEquals((code % 1_000_000).toString().padStart(6, '0'), invite.safetyCode)
        assertFalse(invite.toString().contains(token))
        assertEquals("fd00::1234", PairingInvite.parse(uri.replace("192.168.50.55%3A7777", "%5Bfd00%3A%3A1234%5D%3A7777"), 1000) { 0 }.host)
    }
    @Test fun rejectsDuplicatesOriginsMalformedAndExpiry() {
        for (value in listOf(uri + "&v=1", uri + "&api_key=secret", uri + "#x", uri.replace("zenpi://pair", "https://pair"),
            uri.replace("zenpi://pair", "zenpi://user@pair"), uri.replace("zenpi://pair", "zenpi://pair/path"),
            uri.replace("192.168.50.55", "example.com"), uri.replace("192.168.50.55", "0.0.0.0"),
            uri.replace("192.168.50.55%3A7777", "::::%3A7777"),
            uri.replace("192.168.50.55%3A7777", "%5B1%3A%3A2%3A%3A3%5D%3A7777"),
            uri.replace("192.168.50.55", "224.0.0.1"), uri.replace("192.168.50.55", "255.255.255.255"),
            uri.replace("192.168.50.55%3A7777", "%5B%3A%3Affff%3A255.255.255.255%5D%3A7777"),
            uri.replace("192.168.50.55%3A7777", "%5B%3A%3Affff%3A169.254.1.2%5D%3A7777"),
            uri.replace("7777", "0"), uri.replace("7777", "65536"), uri.replace("v=1", "v=2"),
            uri.replace(token, token.dropLast(1)), "x".repeat(2049))) refused(value)
        refused(uri, 1060); refused(uri, 759); refused(uri, Long.MIN_VALUE)
    }

    @Test fun rejectsEncodedControlCharactersAndNonCanonicalEscapes() {
        for (value in listOf(
            uri.replace("192.168.50.55%3A7777", "192.168.50.55%3A7777%00"),
            uri.replace("fingerprint=$fingerprint", "fingerprint=$fingerprint%0A"),
            uri.replace("token=$token", "token=${token}%2f"),
        )) refused(value)
    }
    @Test fun rawScannerContentsCannotNormalizeControlsWhitespaceOrOverLimitPayloads() {
        for (value in listOf(" $uri", "$uri ", "\n$uri", "$uri\r\n", "\t$uri\t",
            "\u0000$uri", "$uri\u007f", " ".repeat(2048) + uri, uri + " ".repeat(2048))) refused(value)
        assertEquals("192.168.50.55:7777", PairingInvite.parse(uri, 1000) { 0 }.address)
    }
    @Test fun queryPlusIsNotDecodedAsSpace() {
        refused(uri.replace("token=$token", "token=${token.take(10)}+${token.drop(11)}"))
        refused(uri.replace("fingerprint=$fingerprint", "fingerprint=${fingerprint.take(10)}+${fingerprint.drop(11)}"))
    }
    @Test fun tokenPaddingIsRejectedAsNonCanonical() {
        refused(uri.replace("token=$token", "token=${token}="))
    }
    @Test fun monotonicDeadlineCountsSleepAndWallRollbackCannotExtendOrReviveInvite() {
        var elapsed = 10L
        val invite = PairingInvite.parse(uri, 1000) { elapsed }
        assertEquals(60, invite.remainingSeconds(1000))
        elapsed += 59_000_000_001
        assertEquals(1, invite.remainingSeconds(990))
        invite.requireFresh(1000)
        elapsed = 60_000_000_010
        assertTrue(runCatching { invite.requireFresh(1000) }.isFailure)
        assertEquals(0, invite.remainingSeconds(990))
        elapsed = 10
        assertTrue(runCatching { invite.requireFresh(1000) }.isFailure)
        val fresh = PairingInvite.parse(uri, 1000) { elapsed }
        fresh.requireFresh(1000)
    }
    @Test fun countdownUsesOneValidatedClockObservationAndObservedExpiryNeverRevives() {
        var elapsed = 0L; var reads = 0
        val invite = PairingInvite.parse(uri, 1000) { reads++; elapsed }
        reads = 0
        elapsed = 59_000_000_001
        assertEquals(1, invite.remainingSeconds(990))
        assertEquals(1, reads)
        elapsed = 60_000_000_000
        assertEquals(0, invite.remainingSeconds(990))
        elapsed = 0
        assertEquals(0, invite.remainingSeconds(1000))
        assertTrue(runCatching { invite.requireFresh(1000) }.isFailure)
    }
    @Test fun countdownClockAnomalyRetiresInvitationEvenAfterClockRecovery() {
        var elapsed = 100L
        val invite = PairingInvite.parse(uri, 1000) { elapsed }
        elapsed = 99L
        assertEquals(0, invite.remainingSeconds(1000))
        elapsed = 100L
        assertEquals(0, invite.remainingSeconds(1000))
        assertTrue(runCatching { invite.requireFresh(1000) }.isFailure)
    }
    @Test fun observedWallExpiryOrClockAnomalyPermanentlyRetiresThatParsedInvitation() {
        for (bad in listOf(1060L, 759L, -1L, Long.MAX_VALUE)) {
            val invite = PairingInvite.parse(uri, 1000) { 0 }
            assertTrue(runCatching { invite.requireFresh(bad) }.isFailure)
            assertTrue(runCatching { invite.requireFresh(1000) }.isFailure)
            assertEquals(0, invite.remainingSeconds(1000))
        }
        var elapsed = 100L
        val invite = PairingInvite.parse(uri, 1000) { elapsed }
        elapsed = 99
        assertTrue(runCatching { invite.requireFresh(1000) }.isFailure)
        elapsed = 100
        assertTrue(runCatching { invite.requireFresh(1000) }.isFailure)
    }
    @Test fun monotonicSignedWrapAndMaximumWallExpiryRemainOverflowSafe() {
        var elapsed = Long.MAX_VALUE - 10
        val invite = PairingInvite.parse(uri, 1000) { elapsed }
        elapsed = Long.MIN_VALUE + 9
        invite.requireFresh(1000)
        assertEquals(60, invite.remainingSeconds(1000))
        val maximum = PairingInvite.parse(uri.replace("expires=1060", "expires=${Long.MAX_VALUE}"), Long.MAX_VALUE - 300) { 0 }
        assertEquals(300, maximum.remainingSeconds(Long.MAX_VALUE - 300))
    }
}
