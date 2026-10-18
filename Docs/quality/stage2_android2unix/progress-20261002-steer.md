# Desktop / Android Steering Progress

Scope: S2-019 and S2-034 partial implementation, not Master acceptance of either
item. The authoritative 36-item checklist remains unchanged and unfinished.

## Implementation

- Desktop owner mailbox routes `steer` to the existing project's `InputPort`.
  The target is the same authenticated device's active remote prompt, with
  Control ACL, project/session, owner and workspace revision fences.
- The existing owner checkpoint reserves ambiguity before submission. Its
  bounded, non-evicting steering receipt ledger shares request identity checks
  with prompt, workspace/subtab, cancellation and approval, including restore
  and legacy prompt-journal import. No second SessionStore writer or runner.
- The TUI retains and polls input tickets. `accepted` follows the session
  writer's durable input acknowledgement, not just in-memory admission. Exact
  retries, restarts and expired scopes never requeue or target replacement jobs.
- Android's live composer selects steering for accepted/running remote prompts.
  An execution-locked transcript read can place the projection in ERROR while
  its authenticated live status remains useful; steering, like Stop, is allowed
  in that state. Cached/offline state still drops authority. Pending/revocation
  markers disable writes. Receipt polling reuses the exact persisted envelope.
- Android receipt validation checks exact request/target/project/session shape;
  only an accepted receipt resolves the encrypted pending write before reads.

## Verified Evidence

`desktop-android-steer-ledger-20261002.json` is a passed full production-binary
desktop/API-36 Android harness run using direct TLS to 192.168.50.14 without
adb reverse. Terminal pixels were independently QR-decoded. The 24 initial
instrumented tests passed, followed by actual process death, authenticated
network replay, pending-write recovery, revocation receipt recovery and four
independent cache/revocation processes. Twelve directory selection receipts
were independently checked against host checkpoints.

Steering evidence independently joins the checkpoint admission to its target
prompt, deterministic device-scoped input ID, exactly one journal enqueue and
application, and provider context containing both original and steering text.
The desktop made 11 controlled HTTP provider requests, including the steering
continuation. This is not an Echo shortcut. Camera scanning and a physical
Android LAN peer remain unverified. Live delta push remains unimplemented.

Artifact hashes from that run:

- Rust: `7ea49ced44bff2cce25f024c9469b7e99e8ce9d3e218b1315b7abc00e0f954aa`
- App: `2bc9d5a5eb448c210524359e66893a8c0d6449a700029617cfac6a7a56de4728`
- Test APK: `90ab86b3ad475c139559ec5d5e35ee7eab604bfece1fb9dfcc7f31729890a9b9`
- Harness: `229cfe9563e0c4452e0e39fd17662ecdf4f36b498470d6b6ce32fb4fa7147f54`

Additional gates: cargo check/build and fmt passed; two new Rust steering tests
passed; 75 remote integration tests across 11 suites passed; Android Debug and Release
unit suites each passed 44 tests; three narrow-screen PromptScreen instrumented
tests passed; Python smoke-evidence tests passed 11 cases and blueprint tests
passed five cases; strict blueprint and controller validate-only passed.
Existing Rust dead-code/vendor warnings and Compose icon deprecations remain.

## Failed Attempts Preserved

The first attempt exposed the execution-locked transcript/ERROR steering
availability problem and a harness failure-reporting index bug. The diagnostic
receipt records the first failure. After fixing UI availability, the live-control
run verified steering but failed directory selection with an unresolved write;
dependent WorkManager evidence also failed. It is not claimed fixed here.
The repeat run passed all 24 instrumented tests but the harness still expected
23 tests and ten provider calls; it therefore truthfully records overall failure.
The expectation was updated for the additional test and real steering provider
continuation, without reducing any earlier gate. The subsequent full ledger run
passed all gates. Failed receipts remain in the evidence directory.

## Remaining Scope

No whole Blueprint item accepted. Full headless/TUI/Android parity, selected
directory/worktree execution, scheduler concurrency, PTY/bulk channels, event
delta/ACK, local-turn steering policy, interrupted-admission recovery, physical
camera/LAN tests, API matrix and release gates remain unfinished. No commit or
push. The owned emulator was shut down and its process/session exit verified.
