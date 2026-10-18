# Physical LAN Self-Revocation Acceptance Preparation

Recorded 2026-10-04. Partial S2-013/S2-037/S2-052/S2-053 preparation,
not Master acceptance. Previous goal turn added physical Activity recreation.
Current skill, Blueprint/controller, dirty checkout, production revoke UI,
ViewModel/registry and physical receipt paths inspected. No nested workers,
permission bypass, resets, device execution or acceptance checkbox changes.
An asynchronous question requests availability of a dedicated physical API31+
phone and an ordinary terminal environment permitting Gradle/host local TCP;
no answer or permission change is assumed.

## Added Gate

The existing physical QR workflow previously ended with local forget only.
It now submits self-revoke via the production button/confirmation after the
Android prompt roundtrip. A main-thread StateFlow observer installed before
the click captures the original pending revoke UUID without injecting a
request or invoking the model directly. Completion requires the actual
Device revoked on host state, no paired host/projection/pending request,
unregistered state and absent default signing key/encrypted host record.
Finally retains the owned-fixture local-reset cleanup on both success/failure.

The Python harness independently reads the persisted registry before removing
the desktop fixture. It requires exactly the observed device, revoked=true,
the next revision after its Control grant, unchanged capabilities/projects
and the same phone-observed revoke_request_id. v7 receipts require this
revoke_record, self_revoke_verified and exact ordered phase; invalid final
verification clears the gate. v6/earlier receipts cannot prove it. New negative
fixtures reject missing/duplicate/invalid UUID observations, foreign device,
wrong revision/type/ACL/request and missing/duplicate phase. No signing secrets
are logged; the correlation UUID is public request evidence.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-physical-self-revoke-20261004: exit 0. Main/unit/device-test
  source compiles, 593 ordinary JVM tests and all explicit current Rust/Kotlin
  QR/proof/workspace/worktree/session/ACK/approval/PTY/model/resource/file/diff/
  history/worker/prompt/cancel lanes pass. Existing warnings remain. New revoke
  UI/Keystore assertions are compiled only, not device execution.
- Physical verifier fixtures 12, full Stage 2 Python suite 171 and lifecycle
  validator fixtures 25 pass. Strict Blueprint, cargo fmt --check and
  git diff --check pass. All launched handles ended.
- Debug host attribution rebuilt after Cargo lanes; release verify-only passes.
  Binary/source runtime unchanged; no independent reproducibility claim.
- APK verify-only refuses missing provenance. No unchanged denied Gradle/ADB/
  listener attempt or physical QR/LAN run. Controller has zero active/live/
  authenticated claims, requests and handoffs; Blueprint 36 [ ], no [_]/[x].

Signed APK/install/upgrade, connected API31/34/35, physical camera/direct-LAN
two-way sync/recovery/revoke, process death and unrestricted full host gates
remain required. Goal active/incomplete; local forget alone is not host revoke.

```text
PhysicalQrHostTest.kt 95936825b51cf39011c187611f3d5e98af79b051e47783bb662a10910f649efa
physical-harness 476a2b9cf0e45637ece119c328d2d7b7900a91f1ab52b6c91629780119f0a076
physical-harness-tests 15fafb1fbab6796a0f00bf502c6cf1b592aa49fc44fb3b252ad4a99b9b098a6f
main.jar ebc11a7ffe7e70712f34ce0f6585c99950a9c7953de129b44b1c6e57b823ca43
unit.jar 53579dbfbcf54a05b16f56427cf769a0276f01dda11463a5ed8d1aa958b6d946
instrumented.jar b462583f63731142ee6631b770ddfb5bd9d7f2caa64b938c85e7f956653e29dd
debug-receipt 04eb5a3f8132659c5b5f73fdee4ea090079437f1dba77af1562f049dd04a71cf
release-receipt eaea698082564e190c5de4577c0c80a9e52fe9783203462762a7e825ab4131f9
```
