# Host Identity Failure Fencing

Provisional S2-032/S2-038/S2-052 progress on 2026-10-04. Previous goal turn
repaired legacy cache recovery and passed source gates, so it was progress.
Current LAN socket routing, network monitor, pairing/HostClient TLS wrappers,
foreground retry loop, ResumeWorker and authority-loss policy were inspected.
No nested agents, worker admission or authoritative checkbox promotion.

## Repair

Pin and certificate validity refusals already prevented network admission, but
they surfaced as ordinary transient failures. Foreground refresh retained the
previous live capabilities, including error-phase steer/cancel/approval controls,
and its read loop retried. A changed/unverifiable host identity must not leave
those capabilities live merely because TLS safely blocked the current attempt.

PinnedTrustManager now emits HostIdentityException for invalid configured pin,
server chain shape, certificate validity, DER encoding or pin mismatch.
HostIdentityFailure recognizes that typed local signal through TLS cause
wrappers, with identity-based cycle protection. Generic SSL/Certificate errors,
timeouts and matching diagnostic text do not impersonate it.

The existing AuthorityLossPolicy recognizes this signal: the production
ViewModel fences owned reads, removes stale controls and transient surfaces,
preserves bounded durable display/cursors and stops the automatic foreground
read loop. ResumeRetryPolicy also makes this scheduled background opportunity
terminal. No new pin is accepted, token retried, host record deleted or unknown
write resolved. Explicit future refresh/retry still uses the original pinned
TLS verification. Ordinary transport failures retain existing retry semantics.

Three added JVM regressions exercise actual trust-manager refusals using the
existing public Rust certificate, direct/TLS/nested wrappers, forged diagnostic
strings, cause cycles, prebuilt decision refusal and pending write/revoke
identity preservation. No private key is newly embedded. These do not establish
the behavior of Android's actual TLS provider or connected Activity lifecycle.

## Verification

- Final command: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-host-identity-fence-final-20261004; exit 0.
  Main/unit/instrumented sources compile and 400 JVM tests pass.
  Existing API deprecation and unnecessary-safe-call warnings remain.
  Earlier source run predates the final DER-encoding refusal change.
- Stage2 Python discovery: 114 passed; lifecycle verifier tests: 21 passed;
  Blueprint verifier tests: 5 passed. Strict Blueprint passes, all 36 rows [ ].
- rtk cargo fmt --all -- --check and git diff --check pass. Rust production
  source/binary are unchanged; debug host provenance verify-only passes.
- APK verify-only exits 1: missing current-source APK build provenance.
  No stale APK install, unchanged blocked Gradle/ADB/TCP retry, real camera
  scan or direct physical LAN test was attempted.

```text
HostIdentityFailure.kt 6eb3ccba22c2f40e560ce2a2fa2747e3a422df79237edbafee646190e7d8e690
PairingClient.kt de54c4284c1178d7c0b97ad4ab00f2379e7c9f625ce40e80db4adc3f53b287f4
AuthorityLossPolicy.kt 39bce72f691ed615b51d3f0397d29f9669e106ecf820aae3bcd4b3828b40dcb8
ResumeWorker.kt f182c6b1f2f010622d2cb6b98cb442ae8b485427fa605c4c3a6325c9ae065d67
PinnedTrustManagerTest.kt 3b583d18ebaa31f1d1600972f015ff8c7f6a51a1524f0c9879b1fb41a55eaa0e
AuthorityLossPolicyTest.kt f742bfbef9d21d722272ea7a8b2c010ade2f8566ed730408481860d99f63ff5e
main.jar efd6c421f9449b843817859eb177f01f01b8245ef58370e142e5351c9113e4f4
unit.jar 4108f51e0375120384ea2dcfac7c6d0ac09f6da4825431bd16e774cd8ebe3f91
instrumented.jar e81660547be5a7d39bd504a2e267e6f680f8729d1ab9a8100f2dbfcc98026fda
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current-source APK, actual camera QR/direct physical LAN
bidirectional sync, full parity/API lifecycle and signed release acceptance
remain incomplete or unverified. This narrow repair does not accept the final
workflow or a whole Blueprint item.
