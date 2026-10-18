# One-Shot Remote Approval Decisions

The preceding goal turn made read-only approval projection progress. This turn
implements the required decision path across coordinator, owner checkpoint,
remote adapter, local grant CLI and Android. It does not narrow the full
Blueprint objective or claim full approval acceptance. All 36 items remain
unchecked; no commit/push/completion is claimed.

## Behavior

Approve requires a separate Approval device capability, exact project/session,
workspace owner/revision and request ID. Arguments are exactly original
approval request_id, full-request digest and allow/deny decision. Coordinator
digest comparison and answer claim share one lock; local first answer, missing
target or changed digest cannot be overwritten. Remote answers always have
remember=false and no custom message/standing policy grant. The existing Agent
worker audits the accepted decision before a tool side effect; remote accepted
means answer admission, not successful tool execution.

The existing owner checkpoint holds a separate bounded 128-entry approval
receipt ledger. Unknown is committed before answer delivery, then accepted or
definite refusal is saved. Exact retries return the original response after
owner reconstruction without answering again. Unfinished reservations remain
unknown. Restore validates identity/digest/state/code and request collisions
across workspace/cancel/approval ledgers. Existing checkpoint writes preserve
the ledger and approval does not increment workspace revision. Global prompt/
other-operation identity remains unfinished.

`--grant-device-approval <device> --project <project>` explicitly grants
Read/Approval, not Control. Like the existing grant actions it replaces the
device's capability/project set; it is not additive. The CLI parser tests
separate it from Control. Android reads approval_control independently, saves
the exact pending command before transmission and validates the matching
receipt. Check/Close icons open explicit one-shot confirmation; a changed
summary/digest, pending write/revoke, offline or connecting state fences it.
Cache drops approval authority and pending targets. Unknown outcomes offer only
explicit same-envelope retry through the existing pending-write workflow.

## Evidence

Rust remote tests cover Allow and Deny with actual coordinator waits while the
Agent lock is held, separate ACL refusal, wrong owner/revision/digest, exact
retry, changed-choice conflict, on-disk receipt without secret arguments,
owner-epoch reconstruction, unknown reservation restoration, malformed digest
restore rejection and revocation. The coordinator test checks remote-first and
local-first answers plus wrong digest without consuming the request; it is
deterministic ordering, not a simultaneous two-thread race stress test.

`approval_owner` uses the actual configured Agent/provider fixture and built-in
write_file: wrong digest never writes; one-shot Allow creates note.txt with the
expected content, Deny creates no file, and approval_resolved records have the
exact decision and remember=false. All 12 approval_owner tests passed. Full
remote suites passed 73 tests in 11 suites; final workspace rerun passed 17.
Existing approval module tests passed seven and the added digest-fence test
passed separately. Android debug/release each passed 39 tests; both APK builds
passed. Rust check/build passed with 20 existing warnings, no errors. Twelve
harness/blueprint tests, strict validation, controller validate-only, formatting
and whitespace passed.

`desktop-android-approval-decision-20261002.json` passed production desktop
PTY/API 36 emulator private-interface TLS regression without adb reverse.
Its 23 initial instrumentation tests include explicit Deny confirmation and a
changed-digest disabled Allow confirmation using a UI projection fixture.
Subsequent actual Android process-death/revoke recovery, WorkManager and cache
fixtures pass. This run does NOT produce a provider tool approval and answer
it from Android over TLS: that full-chain acceptance is still required.

Receipt hashes match current production artifacts:

- Rust: `05b0e5410371ea913c4f48256b6b9574a53de2fca0b86d9fc5b450d1d932f7be`
- APK: `2c46add70a76fa87919687997f34bd6c86e286b026f3897f351d1887f45eb150`
- Test APK: `38a370e1d5aba76fb2fd93b4e822d3153b8846c22a4128f86fbc2122c7753012`
- Harness: `b6ad587c6876d8d9e7ae1b05652babf979c83eda20cc32ef2f66efde93d33cab`

Owned emulator PID 23121/session 4429 exited; radios enabled and reverse table
empty before shutdown. Temporary listener closure is tested by the harness.

## Remaining Full Scope

Safe detailed previews, actual provider/tool/TLS/Android Allow/Deny, cancellation
and simultaneous answer races, checkpoint faults around answer delivery,
approval audit correlation to remote device/request, killed-host recovery and
full global request identity still require stronger implementation/gates.
Steer, selected-root execution, streaming/ACK, diff/PTY, quotas, Git crash
transactions, physical camera/LAN peer, API matrix, release/migration and full
controller acceptance remain. Emulator TLS and QR decoding are not physical
phone scanning/LAN acceptance. The overall user goal remains active.
