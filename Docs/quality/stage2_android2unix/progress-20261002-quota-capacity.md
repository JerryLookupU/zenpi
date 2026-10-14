# Global Worker Capacity Preflight

Provisional S2-018/S2-019 progress observed 2026-10-02T10:10Z. Previous
goal turn made progress (cross-lane quota serialization). Current worktree
inspected; HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing user
changes preserved. No nested agents, execution-worker admission, commits,
pushes, cleanup or Blueprint acceptance promotion.

## Implementation And Boundary Evidence

Initial WorktreeRunner construction used to prepare slots individually until
the global owner limit rejected a later slot. A failed lane could therefore
consume the last available owner with no runner. Live growth could also reserve
a durable quota intent before discovering deterministic capacity exhaustion,
forcing unknown-outcome recovery for a command that could be rejected upfront.

The pool now provides a whole-range capacity preflight counting only missing
stable project/session/tab/slot identities against the existing 64-owner bound.
Initial runner construction checks its complete range before preparing any
journal or runtime claim. Live quota routing checks its additional range before
reservation and returns ResourceLimit with no receipt or scheduler mutation
when the known capacity does not fit. Preparation failures after preflight
still retain existing lease, checkpoint and ambiguity handling.

Three new behavioral cases cover both sides of the boundary:

- 63 existing owners plus a two-slot initial lane refuse with unchanged owner
  count and directory entry count. A single remaining slot can still be prepared,
  retirement succeeds (no leaked runtime claim), and the two-slot runner later
  starts after retirement.
- Another project owns 63 slots and a live lane owns one. Growing that lane
  to two returns ResourceLimit, preserving checkpoint bytes, journals, pending
  state and applied quota. Retiring and preparing only 62 other slots lets the
  exact unreserved request settle with precisely 64 owners across projects.
- A runner starts 64 independently journaled Agent/Echo writers, holds them,
  observes 64 distinct sessions, and keeps task 65 queued. All 65 complete once
  released, each prompt appears in exactly one worker journal and the master
  journal remains unchanged. The existing 64-owner test also now starts a
  runner using all previously prepared slots without double-counting ownership
  or adding journal entries to the workers directory.

These are local actual Agent/scheduler fixtures, not provider HTTP, Android
camera or LAN acceptance evidence.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-quota-capacity.
Completed commands exit 0:

- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  9 passed, 417 filtered, 4.55 seconds (final exact-64 source).
- rtk cargo test --offline --locked --lib quota_intent_tests -- --test-threads=1:
  13 passed, 413 filtered, 1.56 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host -- --test-threads=1:
  8 passed, 418 filtered, 0.75 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  7 passed, 419 filtered, 0.25 seconds.
- rtk cargo test --offline --locked --test remote_worker_owner --test remote_worktree_runner
  --test remote_concurrency --test remote_workspace --test remote_parity --test runtime
  --test remote_migration --test remote_subtab --test remote_worktree -- --test-threads=1:
  137 passed, nine suites, 37.92 seconds (final prepared-slot reuse source).
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel --test-threads=1:
  134 passed, three TCP fixtures excluded, seven suites, 9.52 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.066 seconds.
- cargo fmt --all --check; git diff --check; /usr/bin/python3
  tools/validate_stage2_android2unix_blueprint.py --strict: pass, all 36 incomplete.

An initial integration-test compilation failed because its new assertion called
crate-private runtime_owned. Removed that assertion; successful retirement
tests the public lifecycle behavior without broadening production visibility.
The final 64-task fixture uses bounded event/command/pending capacities large
enough for 65 queued submissions, and exact quoted prompt matching to avoid
job-1 matching job-10. No production scheduler bounds were changed.

Unique successful Rust cases: 308. All observed command handles terminal;
no observation timeout triggered duplicate execution. Android source unchanged;
no new JVM/Gradle/APK/device claim or restricted
TCP/Gradle/adb bypass. This is provisional evidence, not acceptance-grade
stdout/stderr-hashed receipts.

SHA256:

```text
src/project_workspace.rs 0c564a4a2ea65df9108d62ce19199e0edf1433e58c052fbee8fa34648bab4dd7
src/remote/worktree_runner.rs e2f4b895abade2339dc467ef3062f2f0d10ac5845d1add39a78704c9bfa632ca
src/remote/worktree_host.rs 23c09c6e21a97a75c4ac4aad8aac48b7ad59983878b4cf00578972668373e432
tests/remote_worktree_runner.rs 70027500a12713f2d024343873f8dda9df94154c28bc0a1da2a2b4df55f1d533
tests/remote_worker_owner.rs dbee9f5e38e3a9ffde9cade553e65921d03988aaefdd606e22f34435d8000c17
protocol b91d76d6610b3f4908c718839334b5381d8aa1d7499d7c974df87295704249a8
```

## Remaining Scope

This does not prove all worker fault/lease preparation is atomic, or that every
local TUI workflow shares remote worker routing. Worker controls, full event/
transcript parity, settings and interactive PTY remain unfinished. Actual
API31/34/35 camera QR, LAN desktop synchronization, disconnect/replay/process
lifecycle and signed reproducible release remain unverified. Goal active;
no Blueprint item accepted.
