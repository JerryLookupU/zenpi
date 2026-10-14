# Read-Only Pending Approval Projection

The preceding goal turn made status-retention progress. This turn advances
S2-019/036 by connecting approval visibility to the existing desktop approval
rendezvous and Android workspace projection. This is a partial implementation
of the required full approval workflow, not a replacement for it. All 36
Blueprint items remain unchecked; no commit, push or completion is claimed.

## Implementation

ApprovalCoordinator exposes a bounded metadata-only snapshot, without revealing
or draining pending local prompts and without answering them. SHA-256 hashing
streams serialized request bytes directly to the digest rather than allocating
another full arguments/preview copy. ProjectOwnerPool's existing control handle
provides the coordinator while the Agent execution lock is held. Authorized
workspace listings return at most 16 summaries per project/session: request,
project, session, turn and call identity; redacted tool name; host side-effect
category; origin; and full-request digest. Raw tool arguments, preview, content,
paths and policy/lease detail fields are absent. Normal project Read ACL and
revocation checks govern visibility. Answered requests disappear on reread.

Android requires exact fields, bounded identities, matching project/session,
valid side-effect/origin enums, lowercase digest, unique IDs and capacity. The
existing workspace-status callback can update this projection even if a later
transcript read is Busy. The UI shows Awaiting approval plus tool/effect/request
identity, no unimplemented Allow/Deny controls. Offline views hide the pending
summary, and the encrypted display cache deliberately omits it without changing
its version or importing new decision authority.

## Evidence And Limitations

The Rust integration test uses the real coordinator request wait, actual owner
control handle and authorized HostAdapter workspace read while holding the
Agent lock. It checks exact nine-field summaries, no secret arguments/path,
stable digest, repeated non-consuming reads, unauthorized project invisibility,
desktop drain behavior, local Deny resolution, removal and revoked-device
refusal. An additional 17-pending-request case checks the 16-summary cap,
different complete-request digests and preservation of all 17 desktop prompts.
This is coordinator/adapter behavior, not yet a production provider-generated
tool approval received by Android over TLS.

The first test build tried a private adapter method; it was repaired to use the
existing public owned dispatch, without widening production visibility. The
initial Android test function declaration had a typo; it was corrected before
the final passing build. Rust full remote suites passed 72 tests in 11 suites;
the extended workspace test rerun passed 16. Existing approval unit tests
passed seven. Android debug/release suites each passed 38 tests and debug/test
APK assemblies passed. Rust build passed with 20 existing warnings, no errors.
Twelve harness/blueprint tests, Python compilation, strict validation,
controller validate-only, formatting and whitespace checks passed.

`desktop-android-approval-summary-20261002.json` passed the production desktop
PTY/API 36 emulator private-interface run without adb reverse. All 22 initial
instrumented tests and subsequent actual Android process-death/network/revoke
recovery, WorkManager and independent cache fixtures passed. The new approval
screen test uses an explicit projection fixture, verifies no decision controls,
and hides the summary offline. `approval-summary-narrow-20261002.png` was
inspected: a 109-character request ID wraps at 320 dp without overlap. This
production run proves compatibility/regression and fixture rendering, not a
complete live remote approval workflow.

Current unchanged artifact hashes match the receipt:

- Rust: `e58c28935e344ca4ca0ca5069fe95e995e4859202da139da639da228edc067f4`
- APK: `b3c4b343a42e4247efe0a8cc4c30dd7b9715b45ac8f8ad3f0b04ce82de4b97ad`
- Test APK: `a48873488ff313d1d751b1c4830ca674e7295962a714252a6f37b09e0447f333`
- Harness: `75f1c16cf1611eea363fa6522c5e593a6667e5b3c6774f5d5a469ad09e97b944`

Owned emulator PID 3645/session 65395 exited; radios were enabled and reverse
table empty. The harness checks temporary desktop listener closure.

## Remaining Full Scope

Approve capability/digest-fenced decision admission, durable request receipts,
local-vs-remote answer races, safe previews and real provider/tool/Android
Allow/Deny/cancellation acceptance still remain. Steer, selected-root execution,
streaming/ACK, diff/PTY, quotas, worker/host interruption reconciliation, global
request identity, Git crash transactions, physical camera/LAN peer, API matrix,
release/migration and execution-controller acceptance remain required. QR decode
and emulator private-interface TLS are not physical phone/LAN verification.
