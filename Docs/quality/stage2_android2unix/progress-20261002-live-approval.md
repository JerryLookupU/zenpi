# Live Desktop Tool Approval From Android

The preceding goal turn implemented one-shot remote approval with layered tests.
This turn closes its missing production provider/tool/TLS/Android acceptance
path for Allow and Deny. The full Blueprint goal is unchanged and active; all
36 checklist items remain unchecked because broader gates remain unfinished.
No commit or push was made.

## Production Flow

The existing PTY harness grants the already paired device Read/Approval through
the local CLI, submits an ordinary desktop message and serves a real Responses
HTTP function_call for built-in write_file. The desktop's existing worker
blocks on its approval coordinator. Android receives the pending request via
authorized pinned TLS workspace polling while the Agent lock is held, opens
the real Allow once/Deny confirmation, and calls its production ViewModel.
After the worker consumes the decision, Android receives the provider's final
answer through transcript replay and clears its encrypted pending envelope.
No fixture summary substitutes for this live path.

Allow writes approval-allow.txt with exactly approved-allow; Deny must leave
approval-deny.txt absent. The harness independently checks two accepted
checkpoint receipts and two approval_resolved journal records in Allow/Deny
order. Device/project/session/target request correlation, fingerprint/digest
shape, UUID request IDs, remember=false and host_answer source are required.
Exactly one approval_consumed record belongs to Allow; none may belong to Deny.
The local CLI then restores Read/Control, without treating Approval as Control,
and the ordinary workspace, desktop-message, background and recovery gates run.
Ten desktop provider calls cover three remote prompts, three ordinary updates
and two approval turns with two calls each.

## Repairs And Verification

An early Kotlin test build used an invalid cross-module smart cast and was
repaired to use local nullable projection references. The first live run
completed both decisions but compared restored subtab capabilities before the
Read/Control refresh finished. It now waits for the whole selected workspace
and subtab projection before cache comparison. That run also rebuilt a test
APK during validation, so the harness correctly rejected artifact attribution;
it is not counted as passing and did not emit a receipt.

Retry/final/visual runs passed the behavioral gate. Screenshot inspection found
dialog animation frames, so captures now wait for window settling and include
explicit live Allow/Deny images. A later verified run retained a failing receipt
after an unrelated workspace pending-result timeout and downstream WorkManager
failure; it is not called successful. The unchanged-artifact rerun
desktop-android-live-approval-stable-20261002.json passed all 23 initial
instrumented tests and subsequent actual Android process-death replay/revoke
recovery, pending-write recovery, WorkManager and four cache/intent fixtures.
android_live_tool_approval_verified is true. Earlier receipts remain.

The final live Allow/Deny and transcript images were inspected at 320 dp:
confirmation title, wrapped full request ID, digest and explicit buttons are
readable and nonoverlapping. Animation-frame images from earlier receipts are
not used as polished-layout evidence.

Rust full remote plus approval_owner suites passed 85 tests in 12 suites.
Thirteen harness/blueprint unit tests, including negative identity/consumed/
source evidence cases, passed. Instrumentation APK assembly, strict Blueprint
validation, controller validate-only, Python compilation, Rust formatting and
whitespace checks passed. Production Rust/app code and app APK were unchanged
this turn; prior unit-build results are not represented as rerun.

Current stable-run artifact hashes:

- Rust: `05b0e5410371ea913c4f48256b6b9574a53de2fca0b86d9fc5b450d1d932f7be`
- APK: `2c46add70a76fa87919687997f34bd6c86e286b026f3897f351d1887f45eb150`
- Test APK: `63a3639ed80f979392af39ce681ff1ebd98488768abdb1f11e78c39ac951b28a`
- Harness: `1dd261a8a5ce3cdf72687deda79f8bb497695228a80c3fc8aae50f4fbebdd348`
- Instrumentation helper: `0bacb1abb459edbaceab7544c45146b2b64500bc5c98aab76704c65c1869ad93`

Owned emulators PID 46269/session 86791 and PID 76423/session 72957 exited.
Radios were enabled and reverse tables empty before shutdown. The harness
checks temporary listener closure with desktop exit.

## Remaining Full Scope

Safe detailed previews, broader tool/cancel/timeout and simultaneous local/
remote approval race acceptance, device audit correlation, checkpoint faults,
killed-host recovery and global request identity remain. The workspace unknown
result timeout seen in one run still needs dedicated reliability investigation.
Steer, selected-root execution, streaming/ACK, diff/PTY, scheduler quotas, Git
crash transactions, physical camera/LAN peer, API matrix, release/migration and
controller acceptance remain. API 36 emulator direct private-interface TLS
without adb reverse and terminal QR decoding are not physical phone scanning
or physical LAN acceptance. No full-item acceptance is inferred from this gate.
