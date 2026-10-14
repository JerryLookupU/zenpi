package zenpi

import org.junit.Assert.*
import org.junit.Test
import java.io.ByteArrayInputStream
import java.security.MessageDigest
import java.security.cert.CertificateException
import java.security.cert.CertificateFactory
import java.security.cert.X509Certificate
import java.util.Base64
import java.util.Date
import zenpi.transport.PinnedTrustManager
import zenpi.transport.HostIdentityFailure
import zenpi.lifecycle.ResumeRetryPolicy
import zenpi.pairing.AuthorityLossPolicy

class PinnedTrustManagerTest {
    // Public certificate from an existing Rust HostIdentity fixture; no signing key.
    private val certificate = "MIIBYDCCAQagAwIBAgIUbYd6MhB9Jf274Ndj71G9g0dNmQMwCgYIKoZIzj0EAwIwITEfMB0GA1UEAwwWcmNnZW4gc2VsZiBzaWduZWQgY2VydDAgFw03NTAxMDEwMDAwMDBaGA80MDk2MDEwMTAwMDAwMFowITEfMB0GA1UEAwwWcmNnZW4gc2VsZiBzaWduZWQgY2VydDBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABBuVn7OQ2g8fgc52BWnG/rJRuKroByxG/MdkqcBXwUnjrWo20NYNSFO1QWyxD4uGE6r26QjQD/F0iVY0OjkWq5WjGjAYMBYGA1UdEQQPMA2CC3plbnBpLmxvY2FsMAoGCCqGSM49BAMCA0gAMEUCIQCulFk8+wvpscZMxJUOqFqzb+CuJ2QauXwsHvleB4Ur7AIgNjW6JY6ciYAurXAzTUkbXGKsPEAv6BCLy3wDVkFbdRs="
    private fun cert(): X509Certificate = CertificateFactory.getInstance("X.509")
        .generateCertificate(ByteArrayInputStream(Base64.getDecoder().decode(certificate))) as X509Certificate
    private fun pin(bytes: ByteArray) = MessageDigest.getInstance("SHA-256").digest(bytes)
        .joinToString("") { "%02x".format(it.toInt() and 255) }
    private fun trust(value: X509Certificate, date: Date = Date(1_790_899_200_000L), fingerprint: String = pin(value.encoded)) =
        PinnedTrustManager(fingerprint) { date }
    @Test fun actualRustSelfSignedCertificateWithExactDerPinIsAccepted() {
        val value = cert()
        value.verify(value.publicKey)
        assertTrue(value.subjectAlternativeNames.any { it[1] == "zenpi.local" })
        trust(value).checkServerTrusted(arrayOf(value), "EC")
        assertTrue(trust(value).acceptedIssuers.isEmpty())
    }
    @Test fun everyChangedFingerprintByteRefusesRealCertificate() {
        val value = cert(); val fingerprint = pin(value.encoded)
        for (index in fingerprint.indices) {
            val wrong = fingerprint.toCharArray().apply { this[index] = if (this[index] == '0') '1' else '0' }.concatToString()
            assertTrue(runCatching { trust(value, fingerprint = wrong).checkServerTrusted(arrayOf(value), "EC") }.exceptionOrNull() is CertificateException)
        }
    }
    @Test fun keyOnlyPinCannotReplaceCertificateDerPin() {
        val value = cert()
        assertNotEquals(pin(value.encoded), pin(value.publicKey.encoded))
        assertTrue(runCatching { trust(value, fingerprint = pin(value.publicKey.encoded)).checkServerTrusted(arrayOf(value), "EC") }.isFailure)
    }
    @Test fun missingExtraOrEmptyCertificateChainIsNotTrusted() {
        val value = cert(); val manager = trust(value)
        for (chain in listOf(null, emptyArray<X509Certificate>(), arrayOf(value, value))) {
            assertTrue(runCatching { manager.checkServerTrusted(chain, "EC") }.exceptionOrNull() is CertificateException)
        }
    }
    @Test fun validityWindowIsCheckedEvenWithMatchingPin() {
        val value = cert()
        for (date in listOf(Date(value.notBefore.time - 1000), Date(value.notAfter.time + 1000))) {
            assertTrue(runCatching { trust(value, date).checkServerTrusted(arrayOf(value), "EC") }.exceptionOrNull() is CertificateException)
        }
        trust(value, value.notBefore).checkServerTrusted(arrayOf(value), "EC")
        trust(value, value.notAfter).checkServerTrusted(arrayOf(value), "EC")
    }
    @Test fun invalidPinsFailBeforeTrustAndClientTrustIsNeverGranted() {
        for (bad in listOf("", "a".repeat(63), "a".repeat(65), "A".repeat(64), "z".repeat(64), "../identity")) {
            assertTrue(runCatching { PinnedTrustManager(bad) }.exceptionOrNull() is CertificateException)
        }
        val value = cert()
        assertTrue(runCatching { trust(value).checkClientTrusted(arrayOf(value), "EC") }.exceptionOrNull() is CertificateException)
    }
    @Test fun actualTrustFailuresFenceAuthorityAndBackgroundRetriesThroughTlsWrappers() {
        val value = cert()
        val failures = listOf<() -> Unit>(
            { trust(value, fingerprint = "0".repeat(64)).checkServerTrusted(arrayOf(value), "EC") },
            { trust(value).checkServerTrusted(null, "EC") },
            { trust(value).checkServerTrusted(arrayOf(value, value), "EC") },
            { trust(value, Date(value.notBefore.time - 1000)).checkServerTrusted(arrayOf(value), "EC") },
            { trust(value, Date(value.notAfter.time + 1000)).checkServerTrusted(arrayOf(value), "EC") },
            { PinnedTrustManager("invalid") })
        for (operation in failures) {
            val failure = runCatching(operation).exceptionOrNull() as Exception
            for (error in listOf(failure, javax.net.ssl.SSLHandshakeException("private TLS diagnostic").apply { initCause(failure) },
                java.io.IOException("wrapper", javax.net.ssl.SSLException("TLS wrapper", failure)))) {
                assertTrue(HostIdentityFailure.refused(error))
                assertTrue(AuthorityLossPolicy.refused(error))
                assertTrue(ResumeRetryPolicy.permanent(error))
            }
        }
    }
    @Test fun transientTlsOrForgedDiagnosticTextNeverBecomesAPinRefusalAndCauseCyclesTerminate() {
        val cycle = java.io.IOException("cycle")
        val second = java.io.IOException("second", cycle)
        cycle.initCause(second)
        for (error in listOf(cycle, javax.net.ssl.SSLHandshakeException("Host identity verification failed"),
            javax.net.ssl.SSLException("Host fingerprint mismatch"), java.net.SocketTimeoutException("Host certificate refused"),
            CertificateException("Host identity verification failed"))) {
            assertFalse(HostIdentityFailure.refused(error))
            assertFalse(AuthorityLossPolicy.refused(error))
            assertFalse(ResumeRetryPolicy.permanent(error))
        }
    }
}
