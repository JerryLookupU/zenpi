# Host Diff Review Progress

Partial S2-019/S2-036/S2-052/S2-054 progress, not whole-item acceptance. The
previous goal turn advanced durable transcript ACK and produced new immutable
headless/desktop receipts. Current sources, dirty state and Blueprint were
revalidated before adding the next missing Android display surface. All 36
authoritative rows remain unfinished; the original full goal stays active.
No commit/push, nested agents or controller worker launch.

## Implemented Boundary

The typed Diff operation is read-only. Both owner-thread and shared adapters
route it to the same project owner and current stable-ID subtab, with exact
session/subtab/revision/path validation and current Read ACL. Diff capability
is advertised only on authorized project entries. Android older-host absence
defaults to false. No workspace write permission, revision mutation, durable
request reservation, provider turn, journal append or checkpoint write occurs.

The host validates the existing selected execution root and reuses the local
bounded/cancellable diff implementation. The selected-tab root is never sent
to Android. Whole-tab review explicitly passes ".", preventing a child tab
from selecting the enclosing repository. Ordinary untracked files are included
within the existing count/output bounds. Diff/status are 64/32 KiB and responses
also enforce the serialized control-frame budget. Future bulk paging is not
implemented. Binary/truncation flags and no-change results are explicit.
The Git diff cancellation budget is one second; the existing worktree root
registration validation has its own bounded Git budget. Filesystem syscalls
cannot be preempted, and this is a point-in-time review, not a snapshot lock.

A new fixture initially failed because porcelain-v1 status reports paths from
the repository root even when invoked in a child directory. This silently
omitted selected-child untracked content. Status now uses short output with
explicit relativePaths=true, preserving the existing quoting/parser while
anchoring paths to the configured workspace. Controlled Git runs disable
fsmonitor/hooks, global/system config, inherited root/index/config overrides
and color. External diff/textconv were already disabled. The existing Unix
process-group cancellation/drop path kills and reaps subprocesses.

Android adds an authenticated cancellable read, strict diff codec and a
scrollable/selectable patch dialog with path refresh, loading, error, binary,
no-change and truncation states. It checks exact project/session/subtab/revision
and normalized requested path. Review is transient, not persisted in the
encrypted transcript cache or WorkManager. Network/background loss, mutation,
selection/revision replacement, re-pair, forget and revoke fence/clear it.
Foreground transcript refresh now preserves current transient view state
instead of restoring a stale captured PairingState.

## Validation

New real-owner Rust fixture verifies selected-child tracked/untracked patches,
no enclosing-project sentinel or root leak, explicit file scope, stale revision,
foreign session/project, escaped/missing paths, busy owner, symlink denial,
fsmonitor refusal, unchanged journal/checkpoint and revocation refusal. Initial
untracked assertion failed and was repaired before a 21-test/four-suite pass.
A subsequent 21-suite remote/workspace/tool/input/extension/approval/slash run
passed 232 tests with five existing ignored/helper cases. The 24 slash-actions
library tests also passed, including native literal paths, cancellation,
deadline/process reaping and truncation. The later output-budget/color edits
and added binary/truncation/clean-file/wrong-subtab/mutation-field assertions
passed a subsequent 21-test/four-suite invocation and all 24 slash library
tests. These later cases are not silently included in the earlier broad count.

Android Debug/Release each freshly passed 51 unit tests, zero failures/errors.
Three new codec tests cover read-only request framing, capability/path refusal,
exact identity/revision/flag/path checks and byte limits. Two independent
DiffScreen instrumentation tests passed on the narrow API-36 display, covering
offline/revoke fences, patch/truncation display, path refresh and close.
Gradle app/test APK assembly passed. Tools Stage 2 Python tests passed 31.
Strict Blueprint/controller validation/status passed without live claims,
transports, requests or queued handoffs. Format/diff checks passed.

headless-android-diff-20261002.json passed against the production non-TUI host
over direct private-interface TLS (192.168.50.14), without adb reverse. Actual
production ViewModel diff icon/path refresh/close showed the fixture patch and
left the file unchanged. Two provider prompts, four concurrent reads, actual
Android process death (PID 7622), encrypted original-request retry, six-turn
durable ACK and listener expiry also passed. Saved provider/ACK command evidence
was independently hash-checked and revalidated. The diff screenshot was viewed:
the actual patch renders clearly on 320x640, though keyboard ergonomics warrant
an additional polish pass. This is emulator UI/direct-interface evidence, not
physical camera or phone-LAN verification.

This initial diff run binds these artifacts:

- Rust: dfeb465f427233ba8d7ee9cd39b5a0648e946c8d1974ef522d12737322cebce7
- App: 945550706b95390aa7c0d94fe7d4be2fffbf154247f782f07da2f4d224ce9253
- Test APK: 31ad78b2b63eec5c3df8eff2bb7d01bb9c2e0bdd641e53e14aea989c856681f3
- Headless harness: 3a6dfc9a98876142dda00d20b17581219a31ed4dfa6ca2ba1848ea6487d951d4

The first desktop QR/worktree regression is preserved as
desktop-android-diff-20261002.json with passed=false and 16 provider requests;
it is not relabeled. Review of the newly added assertion found an invalid
expectation that the old tracked content not appear anywhere in a diff. The
deleted baseline line is required and was incorrectly rejected. The assertion
now checks both the deleted baseline and added worktree content. The next
harness version saves full sanitized instrumentation output even for final
failed receipts, not only early exceptions. Path refresh also hides the keyboard
and clears focus, with an explicit Done action, after screenshot inspection.
The corrected desktop run desktop-android-diff-fixed-20261002.json also failed
and remains preserved. Its complete saved stack identifies a selector ambiguity,
not a failed diff: WORKTREE_ONLY_CONTENT existed in both the underlying tool
transcript and the new patch dialog, so the generic text selector matched two
nodes. The patch now has a stable test tag; instrumentation checks both its
text and visibility through that exact node. No failing receipt was overwritten.
Further runs remain separate; running work is not accepted evidence.
The tagged run also failed and is preserved as
desktop-android-diff-tagged-20261002.json. The harness's generic worktree phase
callback treated the new worktree_diff_ui_verified phase as removal, asserting
that the still-existing tree had already disappeared. Routing now handles diff
explicitly, checks that the selected tree/content remain intact, and only
checks removal for worktree_removed_verified. This is a harness-state correction,
not an invented successful Android run.
The separately recorded desktop-android-diff-routed-20261002.json passed after
those repairs: 24 initial tests, actual worktree patch UI with old/new lines,
17 provider calls, directory selection/tool scope, dirty refusal/clean removal,
approval/steer/cancel, WorkManager, actual process/pending recovery, revocation
process recovery and four independent cache/revoke cases. Full sanitized
instrumentation is now part of the receipt. Terminal QR pixels decoded and
the direct-interface/empty-reverse and listener shutdown gates passed.
This run binds Rust f59f8cb78b9464bb6409ff1e5350cc90b3ef122fa1cf05795109a9012ca2d1b0,
App d95d962091898f853d15e9887c45a72ca4f9e5f979dba4261a8636f98af4a4a4,
Test APK fd50469e021cd1bbeb697d6d3dc5306a4114de77843d143fde792e6d2614e034
and desktop harness d854835cc953707d2f334c04f801a1f1550e99a7820a9a7becbee85cafe3b369.
The three earlier failures remain failed and are not evidence for this result.
headless-android-diff-final-20261002.json then passed with the same final
Rust/App/Test APK hashes. Diff UI/file preservation, two provider prompts,
four concurrent reads, actual process death (PID 10057), encrypted exact retry,
six-turn ACK and listener expiry passed. Its saved command hash and independent
provider/ACK verifiers passed again. Final diff screenshot was inspected and
the patch remains visible on the narrow viewport, but the captured keyboard
is still present despite the refresh hide/focus request; keyboard-dismissal
ergonomics are not claimed verified and remain a UI follow-up.

All fixture harnesses/hosts/providers are terminal. The exact owned emulator
serial was stopped, its session exited 0 and adb reports no remaining device.
Final format/diff and strict Blueprint checks passed. Controller validation
launches nothing and status has zero live claims/transports/requests/handoffs.

## Remaining Scope

Independent worktree session owners/executable concurrency, live provider/tool
event push, diff bulk/application and PTY projections, attachment/resource
parity, complete fault/adversarial tests, API 31/34/35 lifecycle matrix, physical
camera/phone LAN and signed release/privacy/security gates remain unfinished.
Diff read does not replace those requirements or complete the synchronization
goal. Existing receipts and unrelated worktree changes remain intact.
