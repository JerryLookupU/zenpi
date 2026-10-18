# Request-Targeted Remote Cancellation

The previous turn verified local RTK/Cargo availability and the current Rust
check, providing toolchain evidence rather than completing Stage 2. This turn
advances S2-019/036/051 with production cancellation verification and UI gates.
All 36 authoritative checklist items remain unchecked; this is provisional
uncommitted work, not full-item acceptance.

## Behavior

The desktop routes cancel through its existing owner mailbox and runner. The
request targets an original remote prompt ID and must match its authenticated
device, project and session, current workspace owner/revision and Control ACL.
It cannot cancel another device's prompt or a later local job. Runtime-bound
exact retries return the same acceptance without another cancellation; changed
envelope reuse conflicts, and revoked devices cannot retrieve an accepted
receipt. Acceptance does not assert terminal cancellation: the worker journals
the eventual outcome through the existing Agent writer.

Android saves the exact pending command before transmission and displays Stop
only for accepted/running status with live authority. Pending writes, connecting
state and unresolved revocation disable it. Cache restoration strips authority
and converts nonterminal status to unknown. Prompt/cancel admission now resolves
after workspace/subtab projection without waiting for the busy transcript lock;
ordinary foreground replay continues separately.

## Verification

The initial cancel receipt remains failed. Its test observed running through
the status callback before the pending admission had finished resolving. The
test now waits for running, pending resolution and an enabled Stop target;
it does not substitute receipt acceptance for terminal cancelled status.

`desktop-android-cancel-final-20261002.json` passed with the production Rust
desktop PTY and API 36 Android emulator connected to `192.168.50.14` directly,
without adb reverse. Its 21 initial instrumented tests include real Send/Stop
and two additional narrow-screen/control-fencing tests. Production journal
checks require three reserved/accepted pairs and progress in this exact order:
running/succeeded/running/failed/running/cancelled. Six desktop provider requests
cover three remote requests and three subsequent/ordinary desktop updates.
Network loss recovery, WorkManager read, real process-death replay, unresolved
write recovery, revoke-process interruption/recovery and four independent
cache/intent fixtures pass. The intentional force-stop phase is not reported
as an ordinary successful instrumentation test.

The standalone UI test initially used an invalid `working` host phase; strict
cache validation rejected the fixture. Correcting it to `idle` produced the
final 21-test pass. `prompt-stop-narrow-20261002.png` and the final transcript
screenshot were inspected: request label, running status and Stop do not
overlap at 320 dp; success/failure/cancelled rows and composer also fit.

Current tested artifacts match the final receipt:

- Rust: `b32443fbb05507f11f897a4353cd5c37f5c01a2dfbdd346c61294a887738fbd8`
- APK: `c7b7ea731b8f254e212342dc2d165c92023fc6a812addfd74571fe4bd7964db8`
- Test APK: `05d42f9a2f4452fc269401c417ca5a4d0e7bf220ad6085631397bd00f2315bd3`
- Harness: `d95b24ea5e3588d3a05602f33f943a222da3ef70769aa2322f8c341dc6bc0433`

`rtk cargo test --test remote_workspace --test remote_headless` passed 13 tests.
Android debug/release unit suites each passed 36 tests and both APK assemblies
passed. Twelve harness/blueprint unit tests passed with `PYTHONPATH=tools`;
the earlier root invocation without that import path failed and is not counted.
Strict blueprint validation, controller validate-only (launching nothing),
Python compilation, Rust formatting and whitespace checks passed.
Owned emulator PID 22745/session 90000 exited; radios were enabled and the
reverse table empty before shutdown. Temporary desktop listener closure is
verified by the harness.

## Remaining Full Scope

Cancel receipts are bounded runtime memory, not durable restart receipts.
Cancellation before worker start, worker panic/progress-write failure,
host restart during execution and global cross-operation request identity
remain open. Steer/approve, selected directory/worktree execution, live delta
streaming/ACK, diff/PTY display, scheduler quotas, Git crash reconciliation,
physical camera/LAN peer validation, API 31/34/35, release/migration and full
execution-controller acceptance still require implementation or stronger gates.
Terminal QR decoding and emulator private-interface TLS do not prove a physical
phone can scan and communicate on LAN. No commit, push or completion claimed.
