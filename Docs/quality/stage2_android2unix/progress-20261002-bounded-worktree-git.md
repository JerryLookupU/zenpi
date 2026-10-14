# Bounded Managed Worktree Git

## Progress And Scope

The preceding development goal turn made verified worktree safety progress.
The intervening RTK installation check did not change product behavior. This
turn advances S2-017 by replacing unbounded `Command::output()` in managed
worktree operations, rather than accepting the current partial Android client
as the final product.

Managed list/add/remove now use the existing host `SupervisedChild` and
nonblocking pipe helper. The Git checks and final action in each operation
share one 15-second deadline. Stdout and stderr each retain at most 256 KiB;
overflow refuses the operation while reading. Alternating reads prevent a
flooding stream from starving the other stream or deadline checks. Stdin is
closed. Every return path cleans the dedicated Unix process group and reaps
the direct child, including ordinary inherited-pipe holders and hooks.

The existing minimal tool environment removes inherited Git repository/config
overrides and provider credentials. System/global Git config is disabled for
managed operations; repository-local config/hooks remain active. Errors retain
the view checkpoint and explicitly describe uncertain filesystem outcomes on
timeout/overflow. No automatic deletion/rollback follows an ambiguous add.

## Verified Evidence

- Six new library tests passed: success/error/non-UTF-8, exact stream boundary,
  stdout/stderr overflow, simultaneous pipe drainage, silent/inherited-pipe
  timeout, expired-before-spawn and owned leader/descendant cleanup.
- Seven workspace/session suites passed 95 tests with one existing ignore.
  Eight remote transport/auth/replay/pairing suites passed 39 tests.
- `rtk cargo build --locked` passed with 20 existing warnings and no errors.
  Formatting, whitespace checks, Python compilation, strict blueprint
  validation, five blueprint tests and four Android harness tests passed.
- The production PTY safety harness passed tracked/untracked/ignored/locked
  refusal and clean close, then exercised actual `post-checkout` hooks. Stderr
  overflow was refused; a sleeping hook timed out in 15.45 seconds and its PID
  was no longer live. Both partial add directories survived without a new tab.
  A subsequent ordinary add/close proved continued desktop usability. Ambient
  `GIT_DIR` and config injection did not redirect the managed Git commands.
- `worktree-bounded-git-current-20261002.json` and
  `desktop-android-bounded-git-current-20261002.json` both pass and match the
  current Rust binary SHA-256
  `fa459145d023b2678ab257053521e3992ae89c78848bf9c46ad418c7c145ce35`.
- Android regression passed twelve initial instrumentation tests, one actual
  process-death network restoration test and two independent cache-process
  fixtures. Direct private-interface TLS used no adb reverse. Production
  workspace/subtab/directory controls, host checkpoints, foreground recovery,
  WorkManager read, pending-write retry and host revocation were checked.

An initial PTY run failed because the harness observed an old refusal while
the next command was still being submitted, causing commands to concatenate.
The harness now waits for the submitted draft to clear before another command;
production refusal behavior was not weakened. The first successful local
receipt uses an earlier binary hash because later Cargo tests rebuilt the
executable. Both local and Android gates were rerun against the final matching
hash; earlier receipts are not promoted to current-binary evidence.

The final Android screenshot was visually inspected: transcript and controls
render, but the scrolled workspace-label area is clipped. This is not full
responsive/accessibility UI acceptance. Owned emulator PID 75662/session
20789 exited cleanly. Wi-Fi/mobile data were enabled and reverse mappings were
empty before shutdown. Temporary desktop hosts/hooks were cleaned up.

## Still Required

This does not provide atomicity between Git side effects and owner checkpoints.
Durable Git intent/reconciliation, creation rollback, filesystem replacement
races and asynchronous owner-thread Git work remain open. Blocking filesystem
syscalls are outside the subprocess deadline; hooks are not sandboxed and a
descendant deliberately escaping its process group is not contained here.
Remote Git remains disabled (`worktree_control=false`). Per-tab real scheduler
quota, correct execution cwd, prompt/approval/diff/PTY parity, live events/ACK,
physical camera/LAN and the full API/release matrix remain required.

No worker/controller acceptance, commit, push or full-goal completion is
claimed. All 36 blueprint items remain unchecked and the original goal stays
active.
