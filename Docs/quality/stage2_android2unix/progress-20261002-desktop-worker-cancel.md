# Desktop Cancellation Of Phone Worktree Tasks

Provisional S2-019/S2-054 progress at 2026-10-02T11:37Z.
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty worktree and current
blueprint inspected. Previous goal turn made progress (tool argument paging).
User changes retained; no nested agents, execution-worker admission, commit,
push, cleanup or acceptance/checklist promotion.

## Implementation

The production TUI's worktree-busy input guard previously rejected /cancel as
well as ordinary input, leaving phone-created tasks without a usable desktop
slash cancellation entry. Both /cancel and ordinary TuiAction::Interrupt now
call ProjectRuntimeHost's local cancellation path. It validates the selected
view root/kind against host-owned active subtab state, then uses WorktreeHost's
live lane rather than a fabricated authenticated device or remote ACL exception.

Cancellation is project/subtab scoped and covers running and queued jobs from
all originating devices, including equal request strings belonging to different
phones. It uses runtime job IDs in the existing runner generation. Wrong project,
missing tab, foreign session/generation or stale local selection fails closed.
Jobs with ambiguous persistence remain cancellable until runtime completion.
The entry does not acquire the executing Agent or master journal lock.

The result counts commands admitted versus deferred by the bounded runtime
queue. The TUI says it is waiting for task exit. No terminal status is synthesized,
no admission receipt is rewritten, no draft erased and no claim released. Normal
controller polling persists actual outcomes for each originating device. Local
force-kill, shutdown and general submissions are not changed; non-cancel input
still stays fenced while worktree tasks run.

## Verification

Cwd /Users/mac/Github/zenpi; ZENPI_HOME=/private/tmp/zenpi-desktop-worker-cancel.
Completed exit-0 commands:

- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  final 13 passed, 423 filtered, 5.31 seconds. New controlled-runtime fixture
  holds two independent subtab workers; two phones submit equal IDs to main
  (one running/one queued), while sibling continues. Cancelling main works while
  master lock is held, never starts queued work, leaves running state/claims
  intact until explicit fixture release, never cancels sibling, preserves original
  receipts/checkpoint, and eventually persists actual per-device terminal states.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  eight passed, 428 filtered, 0.39 seconds (includes progress/admission). Ambiguous
  journal failure remains an active cancellation target until runtime terminal.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test core_session --test runtime --test remote_parity
  -- --test-threads=1: 143 passed, six suites, 37.11 seconds. New production desktop
  host fixture exercises exact slash routing, rejects malformed/lookalike input,
  refuses an uncommitted local tab, retains draft, receipt and independent master
  transcript; actual completion/cancel race is accepted without rewriting outcome.
- rtk cargo test --offline --locked --test remote_workspace desktop_local_cancel
  -- --test-threads=1: final one passed, 58 filtered, 0.19 seconds, not counted twice.
- rtk cargo test --offline --locked --test tui_composer --test tui_project_workspace
  --test remote_concurrency --test remote_auth -- --test-threads=1:
  108 passed, four suites, 5.60 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.064 seconds.
- cargo fmt --all --check; git diff --check; python3
  tools/validate_stage2_android2unix_blueprint.py --strict pass; all 36 unfinished.

Android source unchanged; no new APK/JVM/device claim. No restricted TCP/Gradle/
adb bypass. Test fixture uses controlled local runner jobs, and integration uses
actual desktop host/observed Echo Agent, not LAN/provider HTTP/camera acceptance.
No observed command was restarted because of an observation timeout. Provisional
evidence is not an acceptance-grade receipt with captured stdout/stderr hashes.

SHA256:

```text
src/tui.rs da3f9fdbbd7481c249ede45b86ffff963812d6a771d7c714b49cd54bb18a594d
src/remote/worktree_host.rs 3f344db9f992cad5a4adc87a73838d30cad96e0ad199248a79702dd341c92875
src/remote/worker_progress.rs 3b112473ea99e657d43df451d4ec6b8fa87d0eb26cdf2d4375cc8867ebe19a2c
tests/remote_workspace.rs f13e98ed5e358fa2c4ceffa64331355b48f80d9c022dc2388a7f9329490e51d4
```

## Remaining Scope

This repairs local interruption, not full local worktree task submission/status
or quota UX. Force-kill, live/failed partial history, omitted invocation paging,
settings and interactive PTY parity still need work. Real API31/34/35 camera QR
pairing/LAN desktop sync, disconnect/replay/address changes, process lifecycle,
signed APK and reproducible release remain unverified. Full goal stays active.
