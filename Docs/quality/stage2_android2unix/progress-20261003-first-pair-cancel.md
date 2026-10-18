# First Pairing Cancellation Reconciles Saved Credentials

Provisional S2-031/S2-037/S2-038 progress on 2026-10-03. Previous turn repaired
the reset read-job join barrier and produced verification evidence, so it was
progress. Current worktree and execution skill were inspected again. No nested
agents, checkbox promotion, commit/push or unrelated rollback.

## Repair

cancel previously reconciled encrypted storage only when previousPairing
existed. On first pairing, an already-started synchronous credential save could
finish after cancellation, leaving an empty UI while the actual store held a
registered host. cancel now reconciles any unsaved invitation state as well as
replacement pairing. The storage mutex orders that read after an owned commit.
Saved records restore an offline, display-only host; absent records restore an
empty state. Unreadable storage remains an explicit reset-required failure.

Metadata admission is suspended while reconciliation runs. New camera scans
and refreshes are fenced so they cannot advance generation and strand this
read; incoming invitation links queue until reconciliation completes. The
generation check still prevents late reconciliation overriding a local reset.

PairingCancelTest holds the production storage mutex through a non-cancellable
fixture credential commit and invokes the actual cancel path. It checks no new
scan before release, eventual offline registered host and preserved signing
key, then uses the actual local reset for cleanup. The held commit models the
storage/publication race; it is not evidence of host/network acceptance.

Desktop suites now explicitly include PairingCancelTest: 22 base plus 51
control tests, 73/74 total without/with Doze. The harness and lifecycle receipt
validator require first_pair_cancel_saved_host_restored_verified. Missing,
duplicated or unscoped phase text cannot pass the lifecycle validator.

## Verification

- Stage2 Python discovery: 107 passed. Lifecycle verifier: 20 passed.
- Final Kotlin source check /private/tmp/stage2-first-pair-cancel-final-20261003
  exits zero: main/unit/device-test source compile and 378 JVM tests pass.
  PairingCancelTest compiled only, not device-executed. Physical verifier six,
  instrumentation and Blueprint verifier five each pass.
- Rust debug/release current-source provenance, formatting, whitespace and
  strict Blueprint pass. All 36 authoritative checklist rows remain unfinished.
- APK verify-only refuses missing provenance before device activity. No stale
  APK install, unchanged blocked Gradle/socket retry or physical scan occurred.

```text
PairingViewModel.kt 5e8b33f87a00ab1c9bc2b874d51f191d6788549d04023082fd4ed19370393107
PairingCancelTest.kt d19a70872c2e75fe2bd75c322450d110ca301fe2c17c2ca7571adcb3f4f31ac8
stage2_desktop_android_smoke.py 2da34c3b6e4ca07ec163bfe8b0500b6893e5db7b7c8d2851938933db715de2a1
validate_stage2_android_lifecycle.py ebcc22d9a5ea63fcefd9e5b0e92f49bf83ee602948fcc7af365d8c665c0de27d
test_validate_stage2_android_lifecycle.py 803f0f4317676a3fef51bdb791ef2b903a68c02712160e5cf337114751fd04b4
main.jar de2d56d06771ea52e037b0831394b80515e375961937b1967dd9f34a4df52aee
unit.jar a2a24570e2dffdd018d635b161598dad8b0ab38eb3d269fe7355ed89784f395f
instrumented.jar 8821f351597d59b7428c520c39ea81e12264abcd0a259bfacda2ad18093c3496
```

Goal stays active. Current APK, actual physical QR/direct LAN desktop sync,
complete parity, supported API lifecycle matrix and signed release remain
unaccepted. Source regression preparation cannot satisfy those runtime gates.
