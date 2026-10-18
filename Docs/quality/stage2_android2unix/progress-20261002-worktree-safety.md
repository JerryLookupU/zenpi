# Managed Git Worktree Safety

## Repair

The previous goal turn made verified directory-tab progress. Current source
inspection found the production TUI removed a Worktree tab before attempting
`git worktree remove --force`, then ignored its result. This could delete dirty
files or hide a failed removal. Removal is now non-forced and the view is
changed only after Git succeeds. Failures retain the tab, selection and files;
the local command reports the concrete refusal reason. Busy local discussion
or arch state refuses creation/close.

Add/remove paths are restricted to direct children of the canonical project's
managed directory. Symlink containers/targets, traversal, invalid branches,
existing add destinations and unregistered remove directories are refused.
Removal verifies the common Git repository and rejects modified, staged,
untracked and ignored files before ordinary Git removal; locked worktrees are
also refused by Git. Branch refs are retained. Original-project aliases such
as macOS `/var` versus `/private/var` are normalized without relaxing target
symlink checks. An initial regression exposed that alias boundary and was fixed.

## Evidence

Six `remote_worktree` tests exercise tracked/untracked/staged/ignored contents,
locked and clean removal, main/foreign/unregistered/symlink targets, invalid
branches/destinations/traversal and production TUI failed-close retention/busy
creation refusal. Seven workspace/session suites passed 95 tests with one
existing ignored test before the final added busy-creation assertion.

`tools/stage2_worktree_safety_smoke.py` drives the real desktop binary through
a PTY with `/worktree add` and `/worktree close`. Its final receipt
`worktree-close-safety-final-20261002.json` checks dirty tracked, untracked,
ignored and locked refusal while the host-owned subtab checkpoint retains its
selection. Clean unlocked close must remove the directory/tab but retain the
branch. The fixture uses a temporary repository, no real project cleanup.

The Android direct-interface regression exercises all twelve initial tests,
network-process recovery and independent cache-process stages. The final run
is recorded separately as `desktop-android-worktree-safety-final-20261002.json`.
The Android app is unchanged by this safety repair; Git controls stay disabled.
The authoritative current-binary local receipt is
`worktree-close-safety-current-20261002.json`; it passes all local gates and
matches the Android final receipt's Rust SHA-256
`b571e1c0b87981a1bab23fc75d3e1c9fce83da7c4a2bab299291f99fcb649931`.
The final six-test worktree suite passed including busy-creation refusal.
Android's twelve initial tests, one post-death restore test and two independent
cache-process tests passed. Formatting, strict blueprint validation, five
tooling fixture tests, four harness tests, Python compilation and whitespace
checks passed. Owned emulator and temporary PTY/provider hosts were cleaned
up; no reverse mappings remained and Wi-Fi/mobile-data settings were enabled.

## Remaining Scope

These are local safety checks, not atomicity across Git side effects and owner
checkpoint commits. A crash or checkpoint failure after a successful Git action
still needs durable intent/reconciliation. Arbitrary concurrent filesystem
replacement, bounded Git subprocess cancellation, Git creation rollback,
provider/session cwd routing, scheduler quota and remote Git admission remain
required. No `worktree_control` capability was enabled and no broader S2-017
acceptance is claimed. Physical camera/LAN, session parity, event push and API
lifecycle/release requirements remain open. All 36 blueprint items stay
unchecked; no commit, push or full-goal completion is claimed.
