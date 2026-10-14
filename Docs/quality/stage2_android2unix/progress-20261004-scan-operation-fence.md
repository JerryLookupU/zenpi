# Outstanding Camera Result Fencing

Provisional S2-031/S2-038/S2-052 progress on 2026-10-04. Previous goal turn
repaired typed host-identity failure handling and passed source gates, so it
was progress. Production Activity scanner launch/result, ViewModel restore,
cancel/reset/deep-link transitions and physical QR harness were inspected.
No nested agents, worker admission or authoritative checkbox promotion.

## Repair

beginScan previously cleared an error without recording an outstanding camera
Activity. Duplicate launches were admitted, and a delayed result could replace
state after cancellation or a new deep link. Adding only a Boolean cleared on
cancel would still allow the old callback to be confused with a second scan.

PairingState now tracks scanInProgress until the actual callback returns;
separate transient scanResultAdmitted records whether it may still alter the
pairing. Cancel, local reset or a deep link invalidate admission but retain the
outstanding Activity fence. Late URI/error/cancel callbacks only release the
slot and cannot override the newer invitation or recreate wiped credentials.
No second launch is possible while the previous callback is outstanding.

Foreground refresh, workspace write admission and safety-code confirmation
wait for the camera result. Accepted permission denial/timeout/cancel returns
resume an existing saved host's foreground read when there is no unresolved
operation; invalidated late results do not undo an explicit stop. Camera launch
failure releases admission. State remains transient rather than persisting
invites or camera authority across process death. Existing scanner result tests
now explicitly admit each result instead of silently testing unowned callbacks.

Two JVM tests check scanner admission and write overlap. A prepared device
regression covers duplicate launch, foreground/network return while outstanding,
explicit cancellation, deep-link replacement and late errors/results without
changing encrypted host metadata. Existing held-credential tests release their
test scanner admission before modeling registration. Scanner harness requires
22 exact state-suite test identities and scanner_late_result_fence_verified;
older count-only evidence does not establish the new coverage.

## Verification

- Final command: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-scan-operation-fence-complete-20261004; exit 0.
  Main/unit/instrumented sources compile; 402 JVM tests pass. Existing API
  deprecation and unnecessary-safe-call warnings remain. Earlier runs are not
  the final admitted-return recovery artifact.
- Stage2 Python discovery: 114 passed; scanner verifier tests: 11 passed;
  lifecycle verifier tests: 21 passed; Blueprint verifier tests: 5 passed.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 rows remain [ ]. Rust production/binary are unchanged, and debug host
  provenance verify-only passes.
- APK verify-only exits 1: missing current-source APK build provenance.
  No stale APK install, unchanged blocked Gradle/ADB/TCP retry, real camera
  scan or direct physical LAN run was attempted.

```text
PairingViewModel.kt d5e580059deb793e61723bbccdc8db282d0ccc78e0ebdd7013baf2651c716c8f
PairingConfirmationPolicy.kt edb38edb5399c061abe309306a50a8dd8939672389a14ec6014202bc79f8abc0
WorkspaceDispatchPolicy.kt 7318e3945ea91d94c6329594f214e3476783d8d6114ee815217ffc7632f938d9
MainActivity.kt ee94e254c129bb8f70712ddbade95e8e19727657ce211b567a62f39269deea47
ScannerAdmissionTest.kt d30b86fe79aa9a6479d337553dfac9625fe59d9893a849483c128c3adca8b320
ScannerResultTest.kt c312e5720c0fd5889c989490cfe00b56c96af388e7c5ce16d65596bf70a01e4c
PairingCancelTest.kt c275c2929cf6ee5cd5313c2ab0583250c5bd02dd0463b7f5f71e0ea383dfdc99
stage2_scanner_android_smoke.py a49729b56de74efebcc7b99909eb4f55d74d0b9491948c9fdc0dee90aaca7782
main.jar 7418ef0f6564ef39673fbb3f2c5bd8643226a51c91d51cdf4e1c06685af5be1f
unit.jar 9d07f246c4ef29c39949033ef75a21c136269bef89ddfbca6dc6200d66e1112f
instrumented.jar 83d38c987c56968bbd6c717438f7ab04ee6fe4fb8ab96c9ebea56dc7d4ff66aa
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current-source APK, actual QR/direct physical LAN
bidirectional sync, full parity/API lifecycle and signed release remain
incomplete or unverified. Prepared scanner behavior is not device acceptance
or an entire Blueprint item acceptance.
