# Initial Metadata Recovery Honors Explicit Cancel

Recorded 2026-10-04. Partial S2-031/S2-038/S2-052 evidence, not Master
acceptance. Previous goal turn repaired expired queued-invitation reconnect.
Current skill, Blueprint, controller, dirty checkout and cancellation/recovery
paths inspected. No nested workers, resets, permission bypass or acceptance marks.

## Repair

cancel() already cleared queuedInvite and set pairingRecoveryCancelled when
metadata was still loading. Interrupted-pair reconciliation honored that marker,
but initial cold recovery did not. A user cancelling before saved metadata
arrived could therefore still trigger automatic saved-host reconnection.

Initial recovery now checks the existing cancellation marker before refresh.
It still restores saved host/display offline and finishes metadata admission,
so the scanner is not permanently fenced. A later explicit scan/refresh or
lifecycle/network transition remains a separate recovery action. Valid queued
invitations and the previous expired-invitation fallback behavior are unchanged.

The existing cold cancellation instrumentation test now covers both no URI and
a queued valid URI. It requires offline/no invite, zero connection generation,
null connection job and unchanged encrypted record after restore, then performs
a new explicit scan and requires confirmation without writing the store. The
mandatory cold_deep_link_cancel_verified phase is emitted once after both cases.
This regression is compiled only; no connected execution claim follows.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-cold-cancel-resume-20261004: exit 0. Main/unit/device-test
  source compile; 593 ordinary JVM tests and all explicit current Rust/Kotlin
  QR/proof/workspace/worktree/session/ACK/approval/PTY/model/resources/files/
  diff/history/worker/prompt/cancel lanes pass. Existing warnings remain.
- Stage 2 Python suite 169 pass; scanner verifier 20 and lifecycle fixtures 25
  pass. Strict Blueprint, cargo fmt --check and git diff --check pass.
- Debug host attribution rebuilt after Cargo lanes; release verify-only passes.
  Rust production source unchanged. No independent reproducibility claim.
- APK verify-only refuses missing provenance. No unchanged denied Gradle/ADB
  or production listener attempt was repeated; no physical phone run occurred.
- Controller has no active/live/authenticated claims, requests or queued
  handoffs. Blueprint remains 36 [ ], no [_]/[x]. All launched handles ended.

Current APK/Keystore/Activity execution, signed install/upgrade, API31/34/35,
physical camera QR/direct-LAN bidirectional sync/recovery and full unrestricted
host gates remain required. Source verification is not final acceptance.

```text
PairingViewModel.kt b4153f315e1955778e5f24b08305e703423f5435107a685ac9e259b2fad9a395
ScannerResultTest.kt 6656a00792086fcb0d4997578b3885d446176c694aae03dd9b88c3265683d798
main.jar ebc11a7ffe7e70712f34ce0f6585c99950a9c7953de129b44b1c6e57b823ca43
unit.jar 53579dbfbcf54a05b16f56427cf769a0276f01dda11463a5ed8d1aa958b6d946
instrumented.jar e93a6c17bfb96d8ec65fea61e51009863c38b430503c67d581dff612a56654c7
debug-receipt 05ee6e10185ee5e44b1e0f131ae2d531433788aac2d61955ab9b5df3a7b12ed0
release-receipt eaea698082564e190c5de4577c0c80a9e52fe9783203462762a7e825ab4131f9
```
