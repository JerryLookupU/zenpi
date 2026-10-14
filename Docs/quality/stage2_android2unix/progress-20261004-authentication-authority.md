# Authentication Receipt Authority Fence

Provisional S2-032/S2-038/S2-052 progress on 2026-10-04. The previous goal turn
repaired negotiation refusal and verified source gates, so it was progress.
Current Blueprint, dirty worktree, authentication receipt, HostClient follow-up
admission, authority-loss and ResumeWorker policies were inspected. No nested
agents, worker admission or authoritative checkbox promotion.

## Repair

AuthenticationReceipt already refused invalid result shape, false/coerced
booleans, a foreign device, extra authority and invalid revision before workspace
reads. However these semantic failures used generic WireException, so a failed
foreground reauthentication could retain old Error-phase approval/cancel/steering
capabilities and continue automatic reads.

Complete receipt validation refusal now uses HostAuthenticationException.
Authority-loss handling strips old live/transient surfaces and terminates the
foreground automatic loop; background resume treats it as permanent for that
scheduled opportunity. A mismatch between the local signing identity and saved
host is refused with the same typed signal before opening the socket. Cause
classification is cycle-safe and type-based, not diagnostic-text matching.

Valid current and legacy no-revision read-only receipts remain accepted. Durable
host/text/cursors and unknown exact write/revoke identities remain, without key
replacement, invitation resend, automatic mutation retry or false resolution.
Incomplete transport and unrelated wire errors keep their previous behavior.

Three JVM regressions exercise actual fragmented/correlated receipt validation
before follow-ups, wrapper/type classification, cause cycles, spoofed text and
old decision rejection while preserving unresolved request identity. Existing
receipt adversarial tests remain. They are not connected TLS/Keystore evidence.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-authentication-authority-20261004: exit 0. Production,
  JVM and instrumented sources compile; 419 JVM tests pass. Existing warnings
  remain. Instrumented sources compile only, not device-executed.
- Stage2 Python discovery: 118 pass; lifecycle verifier tests: 21 pass;
  Blueprint verifier tests: 5 pass.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 authoritative rows remain [ ].
- Rust source/binary unchanged; debug host provenance verify-only passes.
- APK verify-only exits 1: missing current-source APK provenance. No unchanged
  blocked Gradle/ADB/TCP retry, old APK install or physical scan/LAN pass was
  attempted. Android device/Keystore, physical QR and sync acceptance remain open.

```text
AuthenticationReceipt.kt 68dddcdb174c0f0e0d3a95f495d8322f215b39964565ed4216179efd0b1aa4b0
HostClient.kt 6ff4a617ac465826344d40bfccbfa112b634b7efec64f4f3d0400f2be944caa9
AuthorityLossPolicy.kt 25d981307fdc1e3e88cba2d2cfe6a5f0332f2776ce4b1f24102df5c188f95144
ResumeWorker.kt eeb668353f98741672e02cf1df39abe57af22ed0087e0b438a9fa6b66ab227c9
AuthenticationReceiptTest.kt 732019a3e30a725852be2eb8c1e929cf8001ceaa30b20f59e8bfef87aa735bb0
AuthorityLossPolicyTest.kt 039ed3e12b52d5738a3f64ff380058ebeae0b62e316fccf16965047e62d534b8
debug-host-receipt 06c70100d9496d38e8128dfe1ed06e29ac935cba3b2c2b1f570f9159f50dcfce
main.jar 144db61df9a2677c04658e5d6c16f8637a8a151673131ad90e1a8a2e3df66a6d
unit.jar 5ca47e9e2204b2e184ee5b255c8e2cc650eede84d3f550fe7baa91dbefe90e4e
instrumented.jar a2d7ab9d56a37c3a8506707a1fe769e8173bfcab35e66d130e7baf13dcd6ec75
```

Goal remains active. Source-bound APK, physical QR/direct LAN bidirectional
communication/recovery, full parity/API lifecycle and signed release remain
incomplete or unverified. No complete Blueprint item acceptance is claimed.
