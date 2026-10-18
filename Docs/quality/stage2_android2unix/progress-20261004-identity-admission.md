# Android Existing Identity Admission

Provisional S2-032/S2-037/S2-038/S2-052 progress on 2026-10-04. Previous goal
turn fixed host invitation admission and recorded passing and failing gates,
so it was progress. Current execution skill, dirty worktree, Blueprint,
foreground/transient reads, key storage, revoke flows, scanner harness and
authority/retry policy inspected. No workers/nested agents admitted; all 36
authoritative rows remain [ ].

## Implementation

Foreground restore previously reported a missing device key as an ordinary
IllegalStateException and continued its transport retry loop. Transient readers
used an ordinary WireException, so they did not trigger the authority fence.
A deleted injected DeviceIdentity or unreadable Keystore could fail while
reading its public key with the same generic behavior.

All ViewModel authenticated reads now use one existing-identity admission
helper on IO. It loads only an existing key, verifies the resulting device ID
against the captured paired host and maps missing/unreadable/deleted/mismatched
identity to the existing typed HostAuthenticationException. Cancellation is
re-thrown, not mapped. Foreground failure stops automatic retry and strips live
authority/transient surfaces. Transient failures fence competing reads too.
The helper never creates keys and executes before HostClient opens its socket.
HostClient retains its independent device-ID check. Explicit QR confirmation
still owns key creation; no wire schema change or default grant change.

Revoke and receipt-query use the same helper. Their authority-failure display
now removes old controls rather than copying them back from the earlier live
state. The persisted unknown revoke/request identities and bounded offline
display are retained; missing keys never resolve or replace those records.

Prepared DeviceIdentityAdmissionTest uses isolated aliases and cache namespaces
to exercise a deleted signing key and a mismatched paired ID through the actual
foreground ViewModel. It expects the typed authority diagnostic, unchanged
encrypted cache, stable Error state and no replacement key. This device test
compiled but did not execute. The scanner state suite now requires its named
test plus a unique marker (24 tests); verifier negatives reject absent,
duplicated or foreign markers. JVM policy regression confirms typed failure
stops retries while preserving unresolved identity/display and removing grants.

## Verification

Commands ran from /Users/mac/Github/zenpi against current dirty integration.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-identity-admission-final-20261004: exit 0; production,
  unit and instrumented source compile; 440 JVM tests pass. Existing warnings
  remain. No instrumented/device execution.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': exit 0; 127 pass.
- python3 tools/test_stage2_scanner_android_smoke.py: exit 0; 13 pass.
- Lifecycle verifier tests 21 pass; Blueprint verifier tests 5 pass.
- Strict Blueprint: exit 0; 36 [ ], zero [_]/[x].
- rtk cargo fmt --all -- --check and git diff --check: exit 0.
- python3 tools/stage2_host_build.py --verify-only: exit 0; Rust source
  unchanged this turn and current debug attribution valid.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. No blocked Gradle/socket retry, old APK install, ADB/camera/LAN run.

```text
PairingViewModel.kt ae3bf9c44036ecf3a17243e335c3f417c6ecf00d34f11ac9155912118d588192
AuthorityLossPolicyTest.kt 9637ca2f1ac97e88c8c4844d7937abba9f2e42227975354cc4f3a2324587ea22
DeviceIdentityAdmissionTest.kt 89511f3e96a06088924e34177867aedec4e24b4dc690fe248b4d7a62fe0c0da6
stage2_scanner_android_smoke.py 42823a4f795f1d0553d8f85107ed85c17d55a1a16978ade69bfc22ae050a51dc
test_stage2_scanner_android_smoke.py b939b007d5d03cf6a74211ba8ef9c03c882801a4b30cdd6a3f12e57749ec142c
main.jar 61741228b0d4caf235cea94749ae5c6468c46e1978e76bf19c127f14b31bedbb
unit.jar af9b261146e17d71291215466d01d76988a9a5a294f0bd3e11ff886c6c2df422
instrumented.jar 422f01da32de40e7fc570ca8a0d66c269e41666cf3edacde02150ca1965f3c1e
debug-host-receipt 91315921cde2e9261bda15641051eae65e6a2ce6d9b644be1186822d3d8e359e
debug-host-inputs ebc19189a14151e3b519da34397aea4e2562f729ef42e2faff123ccd0aee9733
debug-host-binary d9ef9b027304d65c79591e8187fd5439127f17124771d25bb8a4fcf21b1b83e4
```

Goal remains active. Current-source APK, physical QR/direct LAN bidirectional
sync/recovery, connected API 31/34/35 lifecycle matrix, complete parity and
signed release remain incomplete or unverified. Policy/source evidence does
not accept those device/production transport requirements or an entire item.
