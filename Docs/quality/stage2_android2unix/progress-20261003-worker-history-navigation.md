# Worker History Navigation And Selection

Provisional S2-033/S2-036/S2-054 progress on 2026-10-03. The previous goal work
changed transcript navigation source and tests; this turn revalidated that
source (369 JVM tests) before repairing the adjacent task-history reader.
No nested agents, worker admission, checkbox promotion, commit/push or cleanup.
Existing dirty worktree changes are preserved.

## Product Change

WorkerHistoryNavigation now supplies one live-selection gate for Compose and
ViewModel previous/next/message/tool actions. It requires a current succeeded
task with matching project/session/subtab, no reset, no pending write/revoke,
no message dialog, no loading/error and an intact entire retained history.
Page-only completed tasks still use WorkerReadPolicy's transient read scope;
no task status or write capability is published into the display projection.

The at-most-seven preceding pages and current page must agree on digest,
scope and total, have contiguous offsets and valid continuation flags. Each
page is bounded to 16 turns and the history to 1024 turns; turn roles, UTF-8
preview bounds, IDs and NUL are rechecked. Duplicate turn IDs across retained
pages are refused. Fresh history starts at zero. Previous navigation leaves
state unchanged on invalid history, and message dismissal does not restore a
broken history stack. Read content remains transient and cannot execute tools.

## Verification

Three additional JVM regressions exercise corrupted earlier pages, duplicate
turns, malformed/count/offset/role/preview limits and offline/reset/stale task
selection. Existing Compose WorkerScreenTest cases now supply actual task
projections and check disabled message/tool/paging controls for invalid state.
No device test method count changed: desktop remains 71 tests, 72 with Doze.

- Final source check at /private/tmp/stage2-worker-history-navigation-final2-20261003
  exits 0: main/unit/instrumented Kotlin compile and 372 JVM tests pass. Compose
  instrumentation is compiled only, not executed on a connected device.
- Python Stage2 discovery: 100 passed; desktop harness: 16 passed; lifecycle
  verifier: 18 passed; instrumentation and Blueprint tests: five each passed.
- Rust formatting, git diff --check and strict Blueprint pass. All 36 rows
  remain unfinished; no broad acceptance is inferred.
- Rust debug/release current-source build receipts pass verify-only. Rust
  product code is unchanged in this turn.
- APK verify-only refuses missing source provenance. ADB devices fails before
  device discovery because its smartsocket listener is denied (Operation not
  permitted). No APK installation, current connected Compose execution,
  physical camera QR or direct LAN sync is claimed. Unchanged blocked Gradle
  was not rerun.

```text
WorkerHistoryNavigation.kt e573b2cf449109372afc170638d1da7317104a44d24a852275c94876ad0be442
PairingViewModel.kt ed55536fc7bf329a29328c4c4c70c1e1e69446c2fa8d32aebf728af3fe90a576
MainActivity.kt 13b64740e929027a7d7142c40f8251ac2143117f0841983b6321e023c7010dc4
WorkerHistoryNavigationTest.kt bf650e7b3e62861d7ed18ea2419360e7ad652a8eead58d86f09ad3f3b453c8d5
WorkerScreenTest.kt 7911126a737f6da1470c878a0eb27bfaf64fd3c7e3ab9bde0a325ea9d3dfb061
main.jar c5559d87f4af30559a48e601841792726d7deb66d682ae7611e806786f5c3f36
unit.jar dcc2d71b03c41517df997c2afee93a02929618946aacfc35b727b6bcf1a98cf7
instrumented.jar 4d5c622099ca582089221d14f170bd39fbe4539e13a248c59d56382e4ac4c08c
```

The full goal remains active. Current-source APK, actual QR/direct LAN desktop
sync, supported API lifecycle matrix, complete parity and signed release still
need behavioral acceptance.
