# Worker Output Pages

Provisional S2-019/S2-020/S2-036 progress observed 2026-10-02T10:22Z.
Previous goal turn made progress (global quota capacity). Current worktree
inspected; user changes preserved. HEAD remains
1a8ff1acf72e0816fad503a05df2b1d18e07361b. No nested agents, execution-worker
admission, commit, push, cleanup or Blueprint acceptance promotion.

## Implementation

Completed Android worktree tasks previously exposed only a 4096-byte status
preview, or withheld oversized output. The production task pipeline now saves
an internal final assistant turn correlation with its succeeded progress event.
TranscriptRead accepts exactly request_id/offset/digest as a separate worker
form, retaining the existing master turn form. Current Read ACL, project and
master session checks plus the originating device's durable succeeded progress
event resolve the final output. Paths are derived from host scope/slot keys,
never client paths or arbitrary worker session IDs. Legacy events without
correlation fail closed rather than guessing a later answer.

Retired worker journals are opened read-only without Agent preparation, lease
acquisition or directory creation. Canonical/private/user-owned/nonaliased
directory and journal checks, an 8 MiB aggregate scan cap across at most 64
derived keys, per-file cap, and post-read identity/size/mtime checks apply.
Whole-output redaction precedes digest and UTF-8-safe 8192-byte slicing; the
existing 4 MiB content cap and new NUL refusal bound unsafe content. Internal
output_turn_id is removed from live and durable status projections.

Android has a completed-task output icon and reuses the bounded transcript
dialog/navigation through a distinct worker route. Request, subtab, actual
assistant turn, scope, digest, role and byte offsets are strictly decoded.
Seven prior pages remain bounded; task/project/session changes, lifecycle
suspension and authority loss invalidate the open read. The worker marker is
also cleared by display-only authority loss policy. No execution occurs on
Android, and cached projections do not confer read authority.

New Rust behavioral evidence runs an actual production observed worker pipeline
with a multi-page UTF-8 answer and secret assignment, retires the lane, restores
the pool, reconstructs the redacted answer, rejects another device/revocation/
wrong digest, routes through HostAdapter and verifies unchanged worker/master/
checkpoint bytes and no new workers. A new worker filesystem test rejects
missing/nonprivate/symlink/hardlinked/oversized journals without creating data.
Three JVM tests validate worker page construction, continuity, cross-scope
refusal and authority invalidation. New Compose instrumentation source checks
the output button and next-page callback use worker rather than master routing,
including offline disabling; compiled, not device-executed.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-worker-pages.
Completed commands exit 0:

- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  10 passed, 418 filtered, 4.99 seconds (final router test/journal checks).
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  8 passed, 420 filtered, 0.29 seconds.
- rtk cargo test --offline --locked --lib remote::worker_progress -- --test-threads=1:
  4 passed, 424 filtered, 0.14 seconds.
- rtk cargo test --offline --locked --lib remote::transcript -- --test-threads=1:
  2 passed, 426 filtered, 1.26 seconds (NUL test included).
- rtk cargo test --offline --locked --lib quota_intent_tests -- --test-threads=1:
  13 passed, 415 filtered, 1.52 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host -- --test-threads=1:
  8 passed, 420 filtered, 0.77 seconds.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test remote_concurrency --test remote_parity --test runtime
  --test remote_migration --test remote_subtab --test remote_worktree -- --test-threads=1:
  137 passed, nine suites, 37.86 seconds. Later journal/NUL hardening is covered
  by the final lib tests and focused workspace rerun below.
- rtk cargo test --offline --locked --test remote_workspace -- --test-threads=1:
  58 passed, 30.75 seconds, final Rust source.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel --test-threads=1:
  134 passed, three TCP fixtures excluded, seven suites, 9.58 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- /bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-worker-pages-android:
  final main/unit/instrumented source compilation and 238 JVM tests pass, 1.210
  seconds JVM runtime. This uses cached Kotlin/Compose/API35 artifacts, not
  Gradle/APK packaging or device execution.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.064 seconds.
- cargo fmt --all --check; git diff --check; /usr/bin/python3
  tools/validate_stage2_android2unix_blueprint.py --strict pass; all 36 incomplete.

Unique successful Rust cases: 316. All observed command handles terminal;
no observation timeout caused duplicate execution. Initial Rust fixture expected
an echo prefix, but the actual Echo backend returns
the supplied text; corrected the expectation. Initial Kotlin compile lacked the
explicit OpenInFull import; fixed and reran. Neither failure is counted as a pass.
Evidence remains provisional rather than acceptance-grade stdout/stderr-hashed
receipts. No restricted TCP/Gradle/adb bypass.

Final Android artifact SHA256:

```text
main.jar f4df8e13da4f7d7a1e2b6d31e582ff4dae500a846e6f09b4cf07f9490a7fb0d4
unit.jar 0d4608eeef1ad0869d6a9ccf978016107d3126cffaf4e3e8d0d3c9e953f0e396
instrumented.jar bb75831d0bf3d449f861ad094b6ca4c1ed201335bc4586587c55a7e5974279fb
src/remote/transcript.rs 01a8d55ef62e1596910f70bc912ab93e29126c6f80f60044f35b6fb0779348d6
src/remote/worker.rs 066896e0230065c197c4c63b8495717c6131d06b180c916c912034d3c7f41f71
src/remote/worker_progress.rs 0bbca13cbe29ca221bf950f6b5375678b4005e04d65ff011f2f0d6b9d8551774
src/remote/worktree_host.rs 6361be83cf9d14e6511393a0967aa93e5fefe0083e0dd5af24bbc6be6ee5f1d8
```

## Remaining Scope

This exposes completed final assistant output, not full worker user/tool history,
failed/cancelled partial output, branch paging, event/cursor parity or legacy
correlation migration. Journal scans above the bound are explicitly refused.
Local TUI worker controls, settings and interactive PTY remain unfinished.
API31/34/35 camera QR, actual LAN desktop sync, disconnect/replay/process
lifecycle and reproducible signed release remain unverified. Goal active;
no Blueprint item accepted.
