package zenpi

import kotlinx.coroutines.CancellationException
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.pairing.AuthorityLossPolicy
import zenpi.lifecycle.ResumeRetryPolicy

class AuthenticationProofTest {
    private val device = "a".repeat(64)
    @Test fun storageGenerationReplacementDuringIdentityReadCannotAdmitCommitOrKeyRemoval() {
        for (stage in listOf("cache_commit", "request_resolution", "key_removal")) {
            var generation = 7; var admitted = false
            val cancelled = CancellationException("Storage operation replaced")
            assertSame(cancelled, runCatching {
                AuthenticationProof.current(device, { generation++; device }, {
                    if (generation != 7) throw cancelled
                })
                admitted = true
            }.exceptionOrNull())
            assertFalse(stage, admitted)
        }
    }

    @Test fun currentBoundaryChecksCancellationBeforeTouchingTheKey() {
        val cancelled = CancellationException("stopped")
        var read = false
        assertSame(cancelled, runCatching {
            AuthenticationProof.current(device, { read = true; device }, { throw cancelled })
        }.exceptionOrNull())
        assertFalse(read)
        AuthenticationProof.current(device, { read = true; device }, {})
        assertTrue(read)
    }

    @Test fun existingIdentityRequiresASavedMatchingKeyBeforeNetworkOrCommit() {
        val identity = Any()
        var loads = 0; var reads = 0
        assertSame(identity, AuthenticationProof.existingIdentity(device,
            { loads++; identity }, { reads++; device }))
        assertEquals(1, loads); assertEquals(1, reads)
        for (case in 0..3) {
            var connected = false; var committed = false
            val error = runCatching {
                AuthenticationProof.existingIdentity(device, {
                    if (case == 0) null else if (case == 1) throw java.security.KeyStoreException("PRIVATE_ALIAS") else identity
                }, {
                    if (case == 2) "b".repeat(64) else throw java.security.KeyStoreException("PRIVATE_KEY")
                })
                connected = true; committed = true
            }.exceptionOrNull() as Exception
            assertTrue(error is HostAuthenticationException)
            assertTrue(ResumeRetryPolicy.permanent(error)); assertFalse(ResumeRetryPolicy.retry(error, 0))
            assertFalse(connected); assertFalse(committed)
            assertNull(error.cause); assertFalse(error.toString().contains("PRIVATE"))
        }
    }

    @Test fun cancellationDuringExistingIdentityAdmissionIsNotAuthenticationFailure() {
        val cancelled = CancellationException("worker stopped")
        for (loadCancelled in listOf(true, false)) {
            val error = runCatching {
                AuthenticationProof.existingIdentity(device, { if (loadCancelled) throw cancelled else Any() }, { throw cancelled })
            }.exceptionOrNull()
            assertSame(cancelled, error)
        }
    }

    @Test fun successfulProofChecksIdentityAndCancellationOnBothSidesOfSigning() {
        val calls = mutableListOf<String>()
        val proof = AuthenticationProof.sign(device, { calls.add("identity"); device },
            { calls.add("active") }, { calls.add("sign"); "proof" })
        assertEquals("proof", proof)
        assertEquals(listOf("active", "identity", "active", "sign", "active", "identity", "active"), calls)
    }

    @Test fun cancellationWhileKeyReadIsBlockedCannotAdmitTheNextNetworkOrPublicationStage() {
        val entered = java.util.concurrent.CountDownLatch(1)
        val released = java.util.concurrent.CountDownLatch(1)
        val active = java.util.concurrent.atomic.AtomicBoolean(true)
        val admitted = java.util.concurrent.atomic.AtomicBoolean(false)
        val failure = java.util.concurrent.atomic.AtomicReference<Throwable?>()
        val cancelled = CancellationException("network ownership replaced")
        val executor = java.util.concurrent.Executors.newSingleThreadExecutor()
        try {
            val task = executor.submit {
                try {
                    AuthenticationProof.current(device, {
                        entered.countDown()
                        check(released.await(5, java.util.concurrent.TimeUnit.SECONDS))
                        device
                    }, { if (!active.get()) throw cancelled })
                    admitted.set(true)
                } catch (error: Throwable) { failure.set(error) }
            }
            assertTrue(entered.await(5, java.util.concurrent.TimeUnit.SECONDS))
            active.set(false); released.countDown()
            task.get(5, java.util.concurrent.TimeUnit.SECONDS)
            assertSame(cancelled, failure.get()); assertFalse(admitted.get())
            assertFalse(AuthorityLossPolicy.refused(cancelled))
        } finally { released.countDown(); executor.shutdownNow() }
    }

    @Test fun cancellationInsideEitherIdentityReadPreventsSigningOrProofTransmission() {
        val cancelled = CancellationException("connection cancelled")
        for (cancelAtRead in 1..2) {
            var active = true; var reads = 0; var signs = 0; var sent = false
            assertSame(cancelled, runCatching {
                AuthenticationProof.sign(device, {
                    if (++reads == cancelAtRead) active = false
                    device
                }, { if (!active) throw cancelled }, { signs++; "proof" })
                sent = true
            }.exceptionOrNull())
            assertFalse(sent); assertEquals(cancelAtRead - 1, signs)
            assertEquals(cancelAtRead, reads)
        }
    }

    @Test fun missingUnreadableOrReplacedKeyCannotStartSigning() {
        for (read in listOf<() -> String>({ throw java.security.KeyStoreException("PRIVATE_PATH") },
            { throw IllegalStateException("PRIVATE_ALIAS") }, { "b".repeat(64) }, { "invalid" })) {
            var signed = false
            val error = runCatching { AuthenticationProof.sign(device, read, {}, { signed = true; "proof" }) }.exceptionOrNull() as Exception
            assertTrue(error is HostAuthenticationException); assertFalse(signed)
            assertTrue(AuthorityLossPolicy.refused(error)); assertTrue(ResumeRetryPolicy.permanent(error))
            assertFalse(error.toString().contains("PRIVATE")); assertNull(error.cause)
        }
    }

    @Test fun deletedOrReplacedKeyDuringSigningCannotTransmitProof() {
        for (deleted in listOf(true, false)) {
            var changed = false; var sent = false; var signs = 0
            val error = runCatching {
                AuthenticationProof.sign(device, {
                    if (!changed) device else if (deleted) throw java.security.KeyStoreException("deleted") else "b".repeat(64)
                }, {}, { signs++; changed = true; "proof" })
                sent = true
            }.exceptionOrNull()
            assertTrue(error is HostAuthenticationException); assertFalse(sent); assertEquals(1, signs)
        }
    }

    @Test fun signingFailureIsTerminalWithoutRetryOrLeakingKeystoreDiagnostic() {
        var signs = 0
        val error = runCatching { AuthenticationProof.sign(device, { device }, {}, {
            signs++; throw java.security.SignatureException("PRIVATE_KEY_DIAGNOSTIC")
        }) }.exceptionOrNull() as Exception
        assertEquals(1, signs); assertTrue(error is HostAuthenticationException)
        assertTrue(AuthorityLossPolicy.refused(error)); assertTrue(ResumeRetryPolicy.permanent(error))
        assertEquals("Host device authentication unconfirmed", error.message)
    }

    @Test fun actualP256ProofKeepsExactChallengeBytesAndDoesNotResign() {
        val generator = java.security.KeyPairGenerator.getInstance("EC")
        generator.initialize(java.security.spec.ECGenParameterSpec("secp256r1"))
        val key = generator.generateKeyPair()
        val bytes = zenpi.security.DeviceProofCodec.authenticate("f".repeat(64), "A".repeat(43), device)
        var signs = 0
        val proof = AuthenticationProof.sign(device, { device }, {}, {
            signs++
            java.util.Base64.getUrlEncoder().withoutPadding().encodeToString(
                java.security.Signature.getInstance("SHA256withECDSA").run {
                    initSign(key.private); update(bytes); sign()
                })
        })
        assertEquals(1, signs)
        assertTrue(java.security.Signature.getInstance("SHA256withECDSA").run {
            initVerify(key.public); update(bytes); verify(java.util.Base64.getUrlDecoder().decode(proof))
        })
    }

    @Test fun cancellationAtReadSignOrPostSignCheckRemainsTheOriginalCancellation() {
        val cancelled = CancellationException("replaced")
        for (step in 0..3) {
            var checks = 0; var sent = false
            val error = runCatching {
                AuthenticationProof.sign(device, { if (step == 0) throw cancelled else device },
                    { checks++; if (step == 1 || step == 3 && checks == 2) throw cancelled },
                    { if (step == 2) throw cancelled else "proof" })
                sent = true
            }.exceptionOrNull()
            assertSame(cancelled, error); assertFalse(sent)
            assertFalse(AuthorityLossPolicy.refused(cancelled))
        }
    }
}
