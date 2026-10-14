# Retired ViewModel Job Reset Barrier

Provisional S2-038/S2-055/S2-056 progress. Previous goal turn repaired the
session-history join and was progress. This turn revalidated dirty integration,
execution skill and Blueprint; no nested agents, destructive reset or checkbox
promotion. HEAD remains 1a8ff1acf72e0816fad503a05df2b1d18e07361b.
Observed verification time: 2026-10-03T20:26:34Z, Oct 4 local time.

## Repair

The prior reset barrier captured all current job fields, but panel close,
pairing interruption, cancellation and route changes clear those fields after
cancel(). Cancellation is not completion: coroutine/IO cleanup may still be
running, so a later reset could miss the retired operation entirely.

OwnedJobs registers each non-reset ViewModel launch before execution and removes
it only on completion. A lazy coroutine is registered before start, including
immediate completion and an already-cancelled parent scope. Completion callbacks
and reset snapshots synchronize access to the small job set. Local reset drains
this set before credential removal, while existing main-owner resetting state
fences new operation admission. Initialization and storage reconciliation are
included. Reset itself retains its unregistered launch to avoid self-join.
Current-field joins and generation fences remain unchanged. This is not a
general concurrent-admission mechanism or cancellation of WorkManager reads.

Three JVM regressions cover cancelled-but-not-completed jobs, cancellation of
all pending jobs before joining, immediate completion/cancelled parent scope,
and cancellation of a reset waiter without falsely claiming cleanup completed.
New instrumented regression registers held connection/session-history cleanup,
calls production cancel(), verifies fields are cleared, then calls forget().
Keys and saved host must survive until both jobs complete, then be absent.
It is compiled only, not device-executed.

Desktop base suite increases from 31 to 32 tests, workspace mode 83, with Doze
84. Harness and lifecycle validator additionally require the exact named phase
local_reset_retired_jobs_joined_verified. A validator regression refuses a
missing, duplicate or unstructured phase despite an asserted receipt boolean.
No old APK or runner-count-only receipt is promoted to current evidence.

## Verification

Commands ran from /Users/mac/Github/zenpi against preserved dirty integration.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-retired-job-reset-20261004:
  exit 0, production/JVM/instrumented Kotlin compilation, 469 ordinary JVM tests,
  Rust-to-Kotlin read fixture and explicit Android-to-Rust rename/P256/worktree/
  session lanes pass. Existing compiler warnings remain. This does not validate
  manifest/resource merge, DEX, APK packaging, Android Keystore or device UI.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': 132 pass, exit 0.
- python3 tools/test_validate_stage2_android_lifecycle.py: 23 pass, exit 0.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: 5 pass, exit 0.
- Strict Blueprint and git diff --check pass; all 36 rows remain [ ].
- APK verify-only refuses missing provenance. No unchanged restricted Gradle
  attempt, old APK installation or physical/LAN acceptance was performed.
- Current debug host attribution rebuilt after source-check Cargo tests;
  separate verify-only passes, not release or reproducibility acceptance.

One attempted multi-file patch failed document context verification without
applying edits; source and document were inspected before the corrected patch.
No production source edits occurred after compilation started.

```text
OwnedJobs.kt 2bcdc57f07e8ca1a32d107a91fc6f580e935f7a48731812af089ef3b29c4152d
PairingViewModel.kt 66e46c08a653934838abc1205460d7122ea3a9c5b15229c403f0cfaa7c0ec090
OwnedJobsTest.kt d41fbf6a7ebf1a1026e714988d03f08e183f08b43429876c7f798856f67054bb
LocalResetTest.kt fae884c0eed37a02dd109dad4072528b3e7e7f19f0fc0ebb8412513316709b24
stage2_desktop_android_smoke.py fad6ce7b4761fbc17aa2415c8cdceb1c83ae4c60ee0096598531989131a93eba
validate_stage2_android_lifecycle.py 52c55f803cc0740a5593d501fbd06bd4d57823ee210ea3585b7283827385885b
test_validate_stage2_android_lifecycle.py 68e6f1449d4625f13a8f881afca363fcb8e114f9053fe2e0ff04861b78dacc6d
main.jar 1060b9a6f202282e0ecabde46f172b7ec9c35217d5756e1fed083265f6c01587
unit.jar 991c3dc162ac46e8fe2c2be172dc7fcd2f2cfad74b6bdcf4a586eb41c3126fcc
instrumented.jar 8c22b80093a6d240fc71428f32893dddb9b7a3c7fe1e5b4efaff04ca507bba8d
debug-host-receipt 8cedd02c94712d884978b157ddd9b490e13d5455f73fd45a3706745c47197bbd
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active. Fresh APK, physical camera QR/direct-private-LAN
bidirectional sync/recovery, connected API 31/34/35 lifecycle, full parity,
backup/uninstall and signed release gates remain unverified or incomplete.
This repair does not accept a whole Blueprint item or prove the final objective.
