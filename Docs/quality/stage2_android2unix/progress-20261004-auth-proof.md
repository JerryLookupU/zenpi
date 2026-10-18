# Connection Signing Identity Fence

Provisional S2-032/S2-037/S2-038/S2-052 progress on 2026-10-04. The preceding
goal turn changed existing-key admission and verified source/policy behavior,
so it was progress. Current execution skill, dirty worktree, Blueprint,
HostClient signing/receipt/ACK paths and DeviceIdentity inspected. No worker
or nested agent admitted; no authoritative checkbox promoted.

## Implementation

The ViewModel admission added previously checked the key before HostClient.
Deletion/replacement during TLS or a local challenge-signing failure could
still escape as a generic exception and enter foreground transport retry.
HostClient also reread the alias-derived device ID for receipts and ACKs,
instead of using the identity captured for that connection.

HostClient now captures the verified saved device ID on IO before socket
creation. AuthenticationProof checks the current identity immediately before
and after challenge/revoke-status signing. Observed missing/unreadable/deleted
or mismatched keys and local signing errors become HostAuthenticationException
before the request containing that proof is written. Original cancellation
propagates; no signature retry or key creation occurs. Fixed diagnostics omit
underlying private Keystore details. Auth receipt validation, revoke ownership,
revoke receipt and ACK checks use the captured device ID. The host still
verifies the actual cryptographic proof and current ACL; the client checks do
not claim an atomic lock across arbitrary external key changes and socket IO.

Six JVM tests cover call ordering, pre-sign key absence/replacement, post-sign
key deletion/replacement, signing failure, cancellation at each stage and an
actual P-256 canonical challenge signature executed exactly once. Tests invoke
the production helper with controlled callbacks; they do not execute Android
Keystore, production TLS sockets or physical LAN.

## Verification

Commands ran from /Users/mac/Github/zenpi against current dirty integration.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-auth-proof-final-20261004: exit 0; production, unit and
  instrumented source compile; 446 JVM tests pass. Existing warnings remain.
  Instrumented tests compiled only; no device run.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': exit 0; 127 pass.
- python3 tools/test_stage2_scanner_android_smoke.py: exit 0; 13 pass.
- Lifecycle verifier tests 21 pass; Blueprint verifier tests 5 pass.
- Strict Blueprint: exit 0; 36 [ ], zero [_]/[x].
- Generated Android schema current and source backup policy passes.
- rtk cargo fmt --all -- --check and git diff --check: exit 0.
- python3 tools/stage2_host_build.py --verify-only: exit 0; current debug host
  attribution valid. No Rust source changes this turn.
- python3 tools/stage2_android_build.py --verify-only: exit 1; missing APK
  provenance. No unchanged blocked Gradle/socket retry, old APK installation,
  camera scan, ADB or connected acceptance executed.

```text
AuthenticationProof.kt 24f0e6ca9e61ed426f80f527dd224bc8faa3db295800c46db2219bd537a42ab9
HostClient.kt 117f98dff6910e06efaf0f8da151fd564bc78e3d5b20ecff522a942420f1372f
AuthenticationProofTest.kt 18cdb200a353245baee960883b2e47a4b2a3139d67360ef44b52b179919c57e6
main.jar bf82b06f7708a5954cc3814605b7ee59c50fa7316e36875f7e6c672c2f05b621
unit.jar 7553963cdcdf7a3bf96e41116a1304a04ba3e615aea5fe2f136a50c0912976ca
instrumented.jar 422f01da32de40e7fc570ca8a0d66c269e41666cf3edacde02150ca1965f3c1e
debug-host-receipt 91315921cde2e9261bda15641051eae65e6a2ce6d9b644be1186822d3d8e359e
debug-host-inputs ebc19189a14151e3b519da34397aea4e2562f729ef42e2faff123ccd0aee9733
debug-host-binary d9ef9b027304d65c79591e8187fd5439127f17124771d25bb8a4fcf21b1b83e4
```

Goal remains active. Current-source APK, physical QR/direct LAN bidirectional
sync and recovery, connected API lifecycle matrix, complete simultaneous-client
parity and signed release remain incomplete or unverified. Source/policy gates
do not accept an entire item or prove those device/production requirements.
