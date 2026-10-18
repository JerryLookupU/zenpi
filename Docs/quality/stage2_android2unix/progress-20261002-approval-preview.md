# Android Approval Diff Preview

The full Stage 2 objective remains active. This increment implements bounded
host-owned diff display in the real Android one-shot approval confirmation;
it does not accept S2-036 or any complete Blueprint item. All 36 authoritative
checkboxes remain unfinished. Existing user changes are preserved; no commit
or push was made.

## Implementation

ApprovalCoordinator summaries may carry kind/path/patch/changed/truncated/
redacted/withheld. The digest still hashes the original complete request and
the coordinator compares it atomically when claiming a decision. Display
redaction does not change the request or authorize any capability.

The host uses the existing secret registry and assignment/header/JSON key
redaction, including individual formatted JSON property lines. Credential
paths and private-key blocks are withheld completely. A redacted path that is
blank, over 4096 bytes or contains control characters is also withheld, so a
valid Unix filename cannot poison Android's strict projection decoder. Patch
controls except newline/tab are removed, then the patch is bounded to 2048
bytes at a UTF-8 boundary. Truncation is explicit. These are defensive
heuristics, not a guarantee of detecting arbitrary unregistered secrets.

Android accepts only the exact optional preview schema and strict Boolean
flags, path/patch limits and withheld markers. The confirmation shows path,
monospace patch, redacted/truncated/withheld state and complete request ID and
digest in a bounded scrollable region. Captured target equality and existing
live authority/offline/pending-operation fences still control confirmation.
Projection cache serialization excludes previews, pending approval targets
and approval authority.

## Verification

Rust full remote plus approval_owner suites passed 85 tests in 12 suites.
After the path-boundary repair, the 29 workspace/approval_owner tests and the
dedicated remote_diff_preview unit test passed again. Rust build succeeded
with the existing 20 warnings. Formatting and whitespace checks passed.
Android Debug and Release JVM suites each passed 40 tests; both app/debug-test
APKs assembled. The expanded existing Compose case checks long diff scrolling,
redacted/truncated labels, withheld display and offline confirmation fencing.
Thirteen Python harness/Blueprint tests, Python compilation, strict Blueprint
validation and controller validate-only passed. Validate-only launched nothing.

desktop-android-approval-preview-20261002.json passed the complete production
PTY/API-36 emulator harness, including live provider write_file Allow/Deny,
path/patch display, file side effects, receipt/journal correlation, same-owner
transcript replay, default-network recovery, actual process-death pending-write
and revoke receipt recovery, WorkManager and cache/intent fixtures. It used
192.168.50.14 directly over pinned TLS without adb reverse. All 23 initial
instrumented tests passed. Ten desktop provider requests were recorded.

The next run passed all 23 initial tests (including the expanded Compose case)
and process recovery, but the final artifact guard rejected it because a
concurrent Cargo integration-test command relinked target/debug/zenpi. No
attributable passing receipt was emitted for that run. No test or build command
is run concurrently with the final unchanged-artifact harness rerun.

That unchanged-artifact rerun passed and emitted
desktop-android-approval-preview-stable-20261002.json. All initial 23 tests,
including the expanded preview Compose case, and subsequent process/network/
revoke/cache recovery gates passed. android_approval_preview_verified and
android_live_tool_approval_verified are true; listener closure was verified.
Final attributed hashes are:

- Rust: `16427e89e8e5ab8613f7ea34b5034f7d2ddffbee7658bfbcebd5ea6f4ff195cb`
- App APK: `d18249e1b70c3c673c314d1562f7a91b0ea0378b4002ae33e451e5e9662b63d4`
- Test APK: `0432aed6c4027c59647785bf32d54b9fd5e12cbad4241c2e662dfeb570c1bea7`
- Harness: `449ec6e382c61d04e2023c6b84e8e199041b0c33cde8aa42b73c948cf59aaee3`

The captured approval-preview-allow-20261002.png and
approval-preview-deny-20261002.png were visually inspected at 320 dp. Diff,
path, wrapped request ID/digest and both confirmation buttons were legible and
nonoverlapping. These images belong to the first passing run, not a later
artifact generation.

## Remaining Scope

Broader sensitive formats/tools, approval cancellation/timeout, simultaneous
local/remote races, killed-host/storage faults, device audit and global request
identity still need acceptance. An earlier intermittent workspace-result
timeout remains a separate reliability investigation. Selected-root execution,
steering, streaming/ACK, PTY, scheduler quotas, Git crash transactions, physical
phone camera/LAN, supported API matrix, release/migration and controller
acceptance remain unfinished. Emulator direct private-interface TLS and
independent terminal QR decoding are not physical phone scanning evidence.
