# Durable Cancellation Admission

The preceding goal turn made concrete cancellation/UI/production-test progress.
This turn advances S2-015/019/051 by replacing runtime-only cancellation
deduplication with receipts in the existing project owner checkpoint. No new
Agent/journal writer or execution runner is introduced. All 36 Blueprint items
remain unchecked; full command parity and release acceptance are not claimed.

## Implementation

Each cancellation receipt binds device/request/fingerprint/project/session/
target and admission state/code. Before the runner is touched, an atomic
compare-and-replace checkpoint commits unknown_outcome. After cancellation is
queued, accepted is committed; a definite runner queue refusal commits failed
with its Busy code. Other uncertain errors or final-checkpoint failure preserve
the reservation and return unknown_outcome. Exact retries return the saved
state without re-dispatch, including after owner reconstruction with a new
owner epoch. ACL/revocation still precedes receipt lookup. Admission receipts
are not terminal execution outcomes and do not advance workspace revision.

The existing private/no-follow/flock/CAS writer now syncs the checkpoint's
parent directory after rename, in addition to syncing file content. All existing
checkpoint-producing paths carry the cancellation ledger through their cloned
RemoteWorkspaceState. The ledger is capped at 128; legacy checkpoints without
it migrate to an empty list. Restore rejects invalid identities, unsupported
state/code combinations, duplicate request IDs (including workspace receipts)
and over-capacity lists. Workspace/subtab and prompt admission reject IDs
already reserved by cancellation, and cancellation rejects workspace IDs.
A complete cross-operation/session request ledger is still required.

## Verification

Rust tests prove accepted receipt restoration, new-owner exact retry without
another cancel, other-device/refused/stale-target behavior while the Agent lock
is held, and revocation even for accepted retries. Additional cases inspect the
on-disk unknown reservation before the side effect, inject a competing commit
after cancellation, reconstruct the owner and prohibit reexecution. Busy and
uncertain execution results also survive reconstruction. A competing checkpoint
before reservation never invokes cancellation. Legacy migration and five
invalid-record variants fail closed. These are deterministic owner-rebuild/
storage-conflict tests, not an OS power-loss or killed-desktop test.

`rtk cargo test --test remote_auth --test remote_events --test remote_frame
--test remote_headless --test remote_listener --test remote_pairing
--test remote_protocol --test remote_subtab --test remote_tui_pairing
--test remote_workspace --test remote_worktree` passed 70 tests in 11 suites.
`rtk cargo test --lib project_workspace` passed six tests. Targeted workspace/
headless suites passed 15. Check/build passed with 20 existing warnings and no
errors. Formatting, whitespace, Python compilation, 12 harness/blueprint tests,
strict Blueprint validation and controller validate-only passed.

`desktop-android-durable-cancel-20261002.json` passed the production PTY/API 36
emulator run against `192.168.50.14` directly with no adb reverse. Its added
cancel_checkpoint_receipt_verified gate requires exactly one accepted receipt
matching the terminal journal event's device/project/session/target identities,
even after subsequent workspace/directory and ordinary desktop changes.
All 21 initial instrumentation tests, actual Android process-death resume and
revoke interruption/recovery, WorkManager read and independent cache fixtures
pass. The controlled forced process termination is intentionally not treated
as an ordinary successful test. The final screenshot was inspected and has
separate succeeded/failed/cancelled rows with a nonoverlapping composer.

The unchanged final artifacts match the receipt:

- Rust: `878a1c3bec9453535bacbc9ef1e0502ade62c97ee32270b4d73fa78d759cbe68`
- APK: `c7b7ea731b8f254e212342dc2d165c92023fc6a812addfd74571fe4bd7964db8`
- Test APK: `05d42f9a2f4452fc269401c417ca5a4d0e7bf220ad6085631397bd00f2315bd3`
- Harness: `d84b3eae9380d2698bdd80dbff332d6dd196e1fde851541bf7a83eef07e194ee`

Owned emulator PID 61397/session 58489 exited, with radios enabled and reverse
table empty before shutdown. The harness proves the temporary listener closes
with its desktop. No commit or push was made.

## Remaining Full Scope

Host OS/process interruption during admission/execution, interrupted-reservation
operator resolution, worker-start/panic/progress-write reconciliation and
global ID binding remain unverified or unfinished. Steer/approve, selected-root
execution, streaming/ACK, diff/PTY, scheduler quotas, Git crash transactions,
physical camera/LAN peer, supported API matrix, release/migration and execution
controller acceptance still remain. Successful terminal QR decode and emulator
private-interface TLS do not establish physical phone scanning/LAN acceptance.
