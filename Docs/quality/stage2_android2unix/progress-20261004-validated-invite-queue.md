# Validated Cold-Start Invitation Queue

Recorded 2026-10-04. Partial S2-031/S2-038/S2-052/S2-072 evidence, not Master
acceptance. Previous goal work changed pairing/Activity state; the intervening
RTK availability check did not advance Stage 2. Current skill, Blueprint/spec,
dirty checkout, controller status and pairing paths were revalidated. The old
source-check handle 59622 is missing; it was not treated as a live process or
restarted on an observation timeout. This change uses a new verification run.
No nested agents, permission bypass, destructive checkout operation or
acceptance checkbox changes.

## Repair

Before local metadata recovery, acceptUri queued the unvalidated external URI
string. Parsing occurred only after recovery, so an oversized/invalid payload
could be retained unnecessarily and overwrite an already queued valid invite.
The later parse also established a new monotonic admission time.

The queue now contains at most one validated PairingInvite. URI admission
parses before queuing; malformed replacements leave the existing queued object
untouched. Recovery passes that same object to the common admission path, which
rechecks freshness and pending/reset/in-flight ownership fences without parsing
again. An observed expired invitation cannot enter confirmation after restore.
The parser checks UTF-16 length before allocating UTF-8 bytes, while retaining
the existing UTF-8 byte limit for shorter multi-byte inputs.

Two connected-test sources assert bounded queue type/object identity, malformed
replacement/redaction, original deadline retention, observed-expiry refusal and
unchanged saved host/cache. The cold test helper waits for metadataLoaded rather
than assuming an absent error means recovery finished. Scanner acceptance now
requires 28 named state tests and both exact/unique queue phase markers; Python
negative tests reject missing/duplicate markers. Matrices record device gates.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-validated-invite-queue-20261004: exit 0. Main/unit/
  instrumentation source compiles; 593 ordinary JVM tests and all explicit
  Rust/Kotlin QR/proof/workspace/worktree/session/ACK/approval/PTY/model/
  resources/files/diff/history/worker/prompt/cancel lanes pass. Existing
  warnings remain. This is not Gradle, APK packaging or connected execution.
- Python Stage 2 tool suite: 167 tests pass; scanner verifier 20, physical QR
  verifier 10, lifecycle validator 25 and controller fixtures 15 pass. Controller
  fixtures include their expected temporary Gantt-generation diagnostic.
- Strict Blueprint, cargo fmt --check and git diff --check pass. 36 [ ], zero
  [_]/[x]; controller has zero claims/transports/requests/handoff queue.
- After source lanes, debug verify-only detected changed artifact attribution;
  normal host-build regenerated the receipt successfully. Release verify-only
  passes with unchanged host inputs. Rust production source is unchanged here.
- APK verify-only refuses missing build provenance. Retained Gradle failure is
  diagnostic only (daemon local TCP Operation not permitted), not current APK
  evidence. No physical camera/direct-LAN/device run occurred in this turn.
  Signed packaging/install/upgrade, API31/34/35 lifecycle, physical bidirectional
  synchronization/recovery and full supported-host gates remain open.

```text
PairingInvite.kt 6308aebf5e7a9bef1759608ffad1682e1fc3b2b357f046bfde7e1c8673cfcab5
PairingViewModel.kt 3cb62ba6655e49f69b2cc92a6121bc06135e324eb9bdbf51f15747628f806a49
ScannerResultTest.kt 90dba5d1894bea4568fc07638cd86068751945110202a2a0965165bbfbaef09c
scanner-harness f3528bd0f9ca4383f02476726e892838184d75dc7fe3cd071532f881994f58f2
scanner-harness-tests dd10a07d59b08cbbdb30e700097a92ac292077b87b942a0310b187ef78f31d28
main.jar ddaed7c6079763cf64e80c385de3bffb90a3a6a9b4c0582ee69a805afbdcfc03
unit.jar 53579dbfbcf54a05b16f56427cf769a0276f01dda11463a5ed8d1aa958b6d946
instrumented.jar 0ea507e2bbb476244340954af9970c88177a16a110e4c2cdedf1bd7b54414e2b
debug-receipt 691bba6d0748ec0c3cb27f5ec2f73420f5b74cc2694cacfed582deb03046ebb3
release-receipt 8f32146bb045e5da8f3025a8db3a5f3d56982b309b9dd187e615fc72a56948ff
```
