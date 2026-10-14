# Exact Worker Slot Transcript Lookup

Provisional S2-019/S2-020/S2-036 progress observed 2026-10-02T11:05Z.
Previous goal turn made progress (Android history navigation). Current worktree
inspected; HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. User changes
preserved. No nested agents, execution-worker admission, commit, push, cleanup
or Blueprint acceptance promotion.

## Implementation And Evidence

Completed worker reads previously scanned up to 64 possible slot journals under
an aggregate byte cap. A large unrelated sibling journal could prevent a small
target task from being read, and each page could parse multiple unrelated
journals. Observed execution now returns typed crate-private HistoryEvidence
containing the manifest digest, exact prepared slot and actual worker session.
WorkerOwner retains its immutable prepared slot; ordinary model/headless
ProcessResults initialize worker_history_evidence absent. Master success stores
the internal slot/session alongside the existing correlation digests.

TranscriptRead accepts only the master-confirmed integer slot 0..63 and worker
session, derives one journal key from host project/master-session/subtab/slot,
and opens only that file under the existing 8 MiB descriptor bound. The actual
journal session and worker binding slot/key must match. Missing or malformed
metadata fails closed without fallback scanning or client path selection.
The typed runtime identity and internal master fields are removed from status
projections, including ambiguous checkpoint output clearing.

A new controller fixture holds slot zero's deterministic local backend while a
second device executes on slot one. After both finish, it extends slot zero's
journal beyond 8 MiB. The second task's history still reads successfully and
does not contain the sibling task. Missing/float/string/out-of-range slot values
and changed worker session are refused. Restoring the original master success
event and pool checkpoint retains the exact read after restart, without creating
new workers. Internal slot/session/history fields are absent from projected
task status. Existing binding tests additionally mutate the worker slot field.

The first fixture replacement Agent lacked its host tool context, so execution
failed before its backend entered; the observed failed handles were terminal.
Added read-only builtins/context matching the production execution contract and
reran the full suite successfully. Failure is not counted as passing evidence.
Tests use actual observed runner/Agent with controlled local backend, not
provider HTTP or LAN/device acceptance.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-exact-worker-slot.
Completed commands exit 0:

- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  final source 12 passed, 420 filtered, 6.21 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  final source 8 passed, 424 filtered, 0.26 seconds (includes progress/admission).
- rtk cargo test --offline --locked --lib remote::transcript -- --test-threads=1:
  final source 3 passed, 429 filtered, 1.41 seconds.
- rtk cargo test --offline --locked --lib quota_intent_tests -- --test-threads=1:
  13 passed, 419 filtered, 2.40 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host -- --test-threads=1:
  8 passed, 424 filtered, 1.12 seconds.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test core_session --test runtime --test remote_parity
  -- --test-threads=1: 142 passed, six suites, 34.46 seconds, before the internal
  evidence field rename and final restart assertion; final focused rerun follows.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner -- --test-threads=1: final source 83 passed, three
  suites, 28.23 seconds (not added twice to unique case count).
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text --test headless_project_workspace
  --test tui_project_workspace --test tui_composer --test remote_concurrency --test remote_migration
  --test remote_subtab --test remote_worktree -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel --test-threads=1:
  168 passed, three TCP fixtures excluded, eleven suites, 17.29 seconds.
- rtk cargo check --offline --locked: final source zero errors, 20 existing warnings.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.061 seconds.
- cargo fmt --all --check; git diff --check; /usr/bin/python3
  tools/validate_stage2_android2unix_blueprint.py --strict pass; all 36 incomplete.

Android source unchanged; no new JVM/device/APK claim. No restricted TCP/Gradle/
adb bypass. Provisional evidence, not acceptance-grade stdout/stderr hashes.
All observed command handles terminal; no observation timeout caused duplicate
execution.

SHA256:

```text
src/core.rs ba5dcd8de9f6947d8c48f82b76c7f0883a1c32a00b461560bda562ca648e90df
src/headless.rs 173394a341fb23010634bb59a2c601d5bc69ee918e3da12256739db134c6917e
src/remote/worker.rs 79cdd395cb03ce2fde9dcc4b547a4e89d88af82e3463fbff02cbbfd3ca51a339
src/remote/worktree_runner.rs 96082f3ffccc5b94c9c361a5acefb4383128aef3217c491f9830c0758b7f955a
src/remote/worker_progress.rs 33ec2ac00def647bb741431c3b4f986b77b982cdccbaa54fbb02e8fbb48081d0
src/remote/transcript.rs 5e7b161a59cab22a228fe45a68bc5727478240dfed1c1865579ebad6c4858379
src/remote/worktree_host.rs c8b41456f4cb43e88779ec76d560618055c0c1c3e182b40d87a3d0f4f02d1f44
```

## Remaining Scope

This removes unrelated-slot scanning, not the selected journal's own size cap.
Legacy evidence without an exact slot, failed/cancelled partial history, worker
event/cursor replay, local TUI worker controls, settings and interactive PTY
remain unfinished. Actual API31/34/35 camera QR/LAN desktop sync, disconnect/
replay/process lifecycle and reproducible signed release remain unverified.
Goal active; no Blueprint item accepted.
