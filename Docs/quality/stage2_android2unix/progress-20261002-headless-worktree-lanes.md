# Headless Worktree Lane Integration

Provisional S2-018/S2-019/S2-020 progress observed 2026-10-02T08:12Z.
Previous goal turn made progress: shared worker approval route and Android
identity. HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b, dirty changes preserved.
All 36 authoritative Blueprint items remain [ ]. No nested agents, controller
worker admission, commit, push, cleanup or acceptance promotion. Goal active.

## Implementation

HeadlessHost now owns lazy WorktreeHost lanes keyed by project/stable subtab.
Worker Prompt arguments enter the existing durable admission state machine;
ordinary text-only prompts retain their master path. Current Read/Control ACL,
owner/session/workspace and subtab revisions, safe text and private attachment
refusal all precede construction. Managed admission reserves checkpoint/journal
identity before releasing the master execution mutex to prepare independent
workers and submit. The registry authorization guard remains held through
submission and receipt settlement. Exact retries and checkpoint-only ambiguity
do not prepare a worker, including after restart. Post-submit lock/append
failure remains UnknownOutcome and cannot cause resubmission.

Controller polling consumes actual runtime events and persists WorkerProgress.
Workspace listing adds bounded device-scoped worker_statuses separately from
master prompt_statuses, avoiding the old Android strict status codec. Live
values precede durable values across all lanes; a durable result from one lane
cannot mask another lane's live update. Android does not yet consume this field.

Cancel resolves the caller's own target before a foreign matching request and
uses the original typed JobId in its runner generation. Removed targets still
query exact durable cancellation receipts through the existing master fallback.
Master prompt/model admission is fenced while worker lanes exist. Completed
lanes shut down; idle project workers retire when every project lane finishes.
Failed progress appends preserve unknown display but do not prove runtime
termination: runtime_done is independent of persistence status. This prevents
blocking the owner loop by joining a still-running job after ambiguous IO.

Shutdown signals all worker lanes before joining any and shares the master
runtime deadline. Confirmed worker events are observed during join; unresolved
work persists as unknown, not cancellation success, with one bounded worker
progress persistence wait. Detached job closures keep existing runtime claims
and writer leases until the job actually ends.

Two real HeadlessHost mailbox integration tests cover lazy safe admission,
authorization/stale/unsafe refusal without new worker files, master journal
isolation, durable success and immutable admission receipts, restart queries
with byte-identical worker files, revocation, multiple sequential worker turns
and restart. A controller unit test uses an explicit bounded local runtime
closure to hold an active job while cancelling only its queued target; it checks
foreign-device refusal, exact retry and conservative active shutdown status.
An additional tracker unit test ensures failed running-progress persistence
does not imply runtime completion. Checkpoint-only admission regression also
asserts that managed lazy submission is never called.

## Verification

Cwd /Users/mac/Github/zenpi. Final successful Rust checks use
ZENPI_HOME=/private/tmp/zenpi-headless-worker-final (earlier tests use
/private/tmp/zenpi-headless-worker). Commands and observed exits:

- env ZENPI_HOME=... cargo test --offline --locked --test remote_workspace
  -- --test-threads=1: exit 0, 54 tests, 20.33 seconds.
- env ZENPI_HOME=... cargo test --offline --locked --test remote_worktree_runner
  --test remote_worker_owner --test remote_concurrency --test remote_parity
  -- --test-threads=1: exit 0, 37 tests (9/12/11/5).
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  exit 0, 5 tests, 398 filtered, 0.38 seconds.
- rtk cargo test --offline --locked --lib remote::worktree_host
  -- --test-threads=1: exit 0, 1 test, 402 filtered, 0.24 seconds.
- env ZENPI_HOME=... cargo test --offline --locked --lib remote::headless_host
  -- --test-threads=1: exit 0, 8 tests, 395 filtered, 0.76 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit
  --skip production_plain_host_routes_authenticated_tls_prompt_to_the_provider
  --test-threads=1: exit 0, 8 passed, two restricted TCP fixtures excluded.
- env ZENPI_HOME=... cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: exit 0, 103 tests (19/28/56), one TCP fixture excluded.
- rtk cargo check --offline --locked: exit 0, zero errors, 20 existing warnings.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  exit 0, 63 tests. Strict Blueprint checker and git diff --check pass.
  cargo fmt --all -- --check passes. Unique successful Rust cases: 216.

Two initial new-fixture failures came from reading an absent checkpoint after
a no-op commit; a real subtab rename persists the fixture checkpoint. One
existing foreign-tab refusal exposed a changed error precedence; preserve
the existing runner OwnerMismatch before tab lookup. Final tests pass.

Several broad test/utility handles produced no output for prolonged observation;
they were polled before explicit interruption, never assumed terminal. Their
exit 130 partial counts are not successful suite evidence. A raw workspace run
accidentally omitted ZENPI_HOME and failed 19 cases with denied global config
access (35 passed); rerunning with isolated writable ZENPI_HOME passed all 54.
No permission escalation or policy bypass occurred. The final direct commands
completed after interrupted handles. No observation timeout triggered an
automatic duplicate execution. Android source unchanged this turn.

SHA256 at verification:

```text
src/remote/worktree_host.rs 9057cd1437e4cabf6dbd1ee214d0cb2e7baa12e150796b8eea9bf6f8576af63f
src/remote/headless_host.rs f584948f1479c9d499c367783fa6ffb219282cea5de05ec61f4e705ccf9833d7
src/remote/worker_admission.rs ac84c756920aa288ae3c52bf7e2f58158f312c7fe552d1fed0d30dedc2ee0ddb
src/remote/worker_progress.rs e1102f1fdb9da05ac0c1db47ad6c89beafa9bbc74feefee507f199b24cb39257
src/remote/worktree_runner.rs 69b98e8ee3c260fe8a3464a84b447cd43967027e642845929652b897678b5d90
tests/remote_workspace.rs 2d89966d9be3d8254d8782473d12fdaac45b956e658737c0f8c774754eb5dfe4
Docs/stage2_android2unix_protocol.md 4c6eca8fda12bc179d1400f98e43d789822144a58c5b2d9067ad0cab7f58baa8
```

## Remaining Scope

Desktop TUI worker scheduler parity, live effective quota resize/receipts,
Android worker prompt/status/cancel controls, worker transcript/provider event
multiplexing and full settings/interactive PTY remain unfinished. LAN TLS and
Android camera QR must still be validated against the current APK on supported
API levels, with disconnect/replay/process lifecycle and signed reproducible
release. No current TCP/Gradle/adb attempt or device claim. This is provisional
source/mailbox evidence, not physical LAN or G-ANDROID/G-RELEASE acceptance and
not an acceptance receipt with captured stdout/stderr hashes. Full goal active.
