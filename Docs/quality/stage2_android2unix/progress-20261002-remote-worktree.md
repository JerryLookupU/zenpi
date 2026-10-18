# Remote Git Worktree Progress

Partial S2-017/S2-019/S2-035/S2-054 integration, not whole-item acceptance.
All 36 Blueprint rows remain unfinished. No commit or push; dirty user state
was preserved. No nested agents were launched. Controller status showed no
claims/transports/outstanding requests/handoffs; validate-only launched nothing.

## Implementation

The host advertises worktree_control for checkpoint-backed, Read+Control project
authorization. Android accepts that live capability and exposes Add Git worktree
and confirmed Remove Git worktree controls. The narrow-screen toolbar wraps with
FlowRow. Branch input has an explicit Done IME action. Cache restoration discards
worktree authority, and pending/offline state fences new actions.

Creation accepts only kind=worktree, a bounded ASCII fresh-branch name and subtab
revision. No remote path is accepted. The host derives a direct child of its
canonical .zenpi-worktrees directory, checks both revisions/capacity/duplicate
names and discussion/arch busy owners, then calls the same bounded Git helper
as desktop. Close preflights registration, shared Git directory and tracked,
staged, untracked and ignored cleanliness. Ordinary removal retains the branch;
preflight failure retains the tab/selection/files without checkpoint changes.

Before Git, the exact device/request/fingerprint is durably reserved as a null
workspace receipt. Success replaces that reservation atomically with the tab
state/result and increments both revisions once. Null results are unknown_outcome
in both subtab and workspace adapters, never succeeded. Exact retry, including
restart, cannot rerun Git; changed request contents or cross-command reuse fail.
Git errors/timeouts and post-Git checkpoint conflicts retain uncertainty and any
filesystem outcome. The Android encrypted pending envelope remains fenced.

This does not provide a complete operator reconciliation UI, independent
worktree session owner/concurrency, OS isolation of repository hooks or a full
crash-window matrix. Existing local Git hooks/config retain their existing
execution semantics under the bounded supervisor and minimal child environment.

## Production Evidence

`desktop-android-worktree-dirty-20261002.json` passed the complete production
desktop/API-36 Android harness on direct 192.168.50.14 TLS, no adb reverse.
The real terminal QR was independently pixel-decoded; physical camera/phone LAN
remain unverified. All 24 initial instrumentation cases passed, followed by
actual force-stop/cached-cursor/pending-write/revocation recovery and four
independent cache/revoke process cases. Seventeen controlled HTTP provider calls
were recorded, not Echo.

Android's real buttons created android-feature, read keep.txt through the shared
desktop Agent, attempted removal while dirty, observed explicit host refusal
with the tab retained, refreshed after the harness restored only its tracked
fixture bytes, and removed the clean worktree. Independent checks inspect Git
registration, host canonical root/stable ID, provider/tool outputs, dirty contents
and final absence plus branch retention. Worktree bytes WORKTREE_ONLY_CONTENT
differ from main TRACKED_WORKTREE_CONTENT, so a main-root read cannot satisfy
the checker. The commands artifact records create/dirty_refusal/remove, six
prompt bindings, twelve reserved/accepted admissions and terminal progress.

Hashes from the final passed harness:

- Rust: `2c26973b47cca9f4944628279dc0a5b5c8e4751b39c7e9dca541d73212446f2a`
- App: `08a888a3dd46991340bd51d4cceb4cb0f6851d78684652c8dc1bfb7a6dcd89d0`
- Test APK: `871288a282f7c4a7a18b1ac36e4af916bad1aa09bb8f08eba9ba9e68d5cf33b0`
- Harness: `b548bb604fdce3021e00e8b79928045f078fcfa421b7b565044e3f07abdc076b`

The owned emulator was stopped and its execution handle returned exit 0.
The desktop listener closure is independently checked by the harness.

## Regression And Failed Attempts

The six targeted Rust suites passed 47 tests: remote_workspace, remote_worktree,
remote_subtab, remote_headless, remote_parity and remote_auth. New real-Git tests
cover create/close/restart deduplication, dirty refusal, read-only denial,
invalid branch names, busy owner, ambiguous Git failure, cross-command ID
conflicts and a real post-checkout hook inducing a post-Git checkpoint CAS
conflict. Externally removing the fixture worktree/branch before retry proves
the unknown reservation does not execute add again.
Android Debug/Release unit suites each passed 45 tests; APK builds passed.
The final combined 12 core/tool/approval/input/extension/remote suites passed
203 tests with five existing/helper ignored tests, retaining the prior
extension-workspace coverage alongside the new Git paths.
Python evidence tests passed 14; Blueprint tests passed five. Build, fmt check,
diff check, strict Blueprint and controller validation passed with existing
vendor/dead-code/icon-deprecation warnings.

Failed receipts are retained, not promoted:

- desktop-android-worktree-20261002: new branch input had default IME action;
  production Done handling was added.
- The IME retry also hit a narrow-screen move click/WorkManager failure and
  artifact hash guard; no attributable JSON pass was produced. A test invocation
  can rebuild Cargo artifacts, so live harness/build overlap is not valid evidence.
- desktop-android-worktree-scroll-20261002: all instrumentation/process gates
  passed but the verifier still expected five admissions rather than six.
  The exact count was corrected; the subsequent evidence run passed.
- desktop-android-worktree-evidence-20261002 passed the basic create/read/remove
  workflow; the later dirty receipt strengthens it with distinct-root bytes and
  destructive refusal rather than relying on identical checkout contents.
- New test compilation initially missed dispatch_prompt's boolean argument;
  cross-cancel assertions initially called the metadata adapter instead of
  CancelAdmissions. Corrected production admission routes passed.

## Remaining Scope

Full headless/TUI/Android command parity, independent worktree owners and quota,
operator reconciliation, resource/attachment scope, PTY/bulk output, live delta
push/ACK, supported API lifecycle matrix, physical scanning/LAN and release
security gates remain required. This is concrete workflow progress, not full
Stage 2 completion or a reason to change checklist marks.
