# Pairing Key Creation Shares The Reset Barrier

Provisional S2-031/S2-037/S2-038/S2-056 progress on 2026-10-03. Previous turn
removed implicit transport identity creation and produced source verification,
so it was progress. Current worktree and execution skill were inspected again.
No nested agents, checkbox promotion, commit/push or unrelated rollback.

## Repair

The sole production loadOrCreate caller ran directly in Dispatchers.IO without
the storage mutex. After explicit cancellation cleared connection, local reset
could not join that old job. A still-blocking Keystore creation could finish
after reset wiped the old key, leaving a new credential behind successful reset.

PairingCredentialAdmission now acquires the same storage mutex used for saving
credentials and reset, dispatches to IO, checks cancellation and current
generation, then calls the non-suspending identity loader. An admitted blocking
creation retains the mutex until it returns. A cancelled/stale waiter cannot
create; an already-running creator finishes before the reset deletion. No
network request runs while holding the storage mutex.

Four JVM regressions cover blocked creation cancelled before reset, replaced
generation, cancelled mutex waiter, and generation replacement while waiting.
The first holds a real blocking callback, verifies reset cannot finish while it
is held, releases it, then checks reset removes the result and no stale result
is published. These are deterministic coroutine ordering tests, not Android
Keystore or physical credential-erasure acceptance.

## Verification

- Stage2 Python discovery: 107 passed; lifecycle verifier: 21 passed.
- Physical QR verifier: six passed; instrumentation/Blueprint tests: five each.
- Kotlin source gate /private/tmp/stage2-key-reset-serialization-20261003 exits
  zero: main/unit/device sources compile and 382 JVM tests pass, including all
  four new admission regressions. Device source compilation is not execution.
- Rust debug/release attribution, formatting, whitespace and strict Blueprint
  pass; Rust unchanged and all 36 rows remain unfinished.
- APK provenance remains missing. No stale APK installation, unchanged blocked
  Gradle/ADB retry, physical camera or direct LAN result is claimed.

```text
PairingCredentialAdmission.kt 765cf4b5e8470b1ffc42b1ca6d5af21910cf273653742e2f02143f07800b2ffe
PairingViewModel.kt 1ae70504335ff2ab075c40602498e63d6f99e7955e1371e8020f2dd6e1b12064
PairingCredentialAdmissionTest.kt d240e2e03d6dbfd4df587ceb76361949ff17b5c43333357607b9f5c6767f0329
main.jar bf216a2fcca9dfe913c828e981bb2e204ba6ff8f7e1f71144e57f89d45013d95
unit.jar 6c1ea4777281d1a50855b3db2d415ea814957b56e67d3e687c3a46f1e874f4dd
instrumented.jar 500db487dae058d98e84d09416e6ee052c3d4c730b68161bde5ce95ffbd8292a
```

Full goal remains active. Current-source APK, physical QR/direct LAN desktop
sync, complete parity, supported API lifecycle and signed release remain
unaccepted.
