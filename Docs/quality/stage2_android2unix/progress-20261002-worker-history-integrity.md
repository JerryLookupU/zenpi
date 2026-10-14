# Worker History Integrity And Tool Execution

Provisional S2-019/S2-020/S2-036 progress observed 2026-10-02T10:51Z.
Previous goal turn made progress (task-scoped history and Android UI). Current
worktree inspected; HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. User
changes preserved. No nested agents, execution-worker admission, commit, push,
cleanup or Blueprint acceptance promotion.

## Implementation

The worker history manifest was previously checked against its own journal but
not independently authenticated by the master success record. The production
observed runner now hashes its ordered manifest, persists that hash alongside
the worker binding, and returns the hash as crate-private ProcessResult
evidence. WorkerProgress copies it into internal durable master metadata;
ordinary model/headless results initialize it absent. Wire status projection
removes this metadata and checkpoint failure clears it with the other output
fields. No Android schema or client changes are needed.

Task history reads require the worker manifest hash and master success hash to
both match the current manifest. Merely editing worker history and recomputing
its hash cannot extend the authorized range. Every non-input message must have
the original input as parent, in addition to contiguous bounds, role/hash/ID
validation and final-output correlation. The existing production test now
modifies the worker manifest and recomputes only its local hash, checking refusal.

A new production observed-runner test prepares a deterministic controlled local
backend on an independent worker Agent, invokes the actual built-in read_file
tool, then completes a second model iteration. Its task history must contain
user, intermediate assistant, real tool result, and final assistant messages,
including actual file content but excluding its secret assignment. The master
has no conversation turns, and the worker retires. This verifies the core tool
loop and history parent contract, not an Echo-generated pretend tool event.
The backend performs no provider HTTP; LAN/device acceptance is not implied.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-history-integrity.
Completed commands exit 0:

- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  final source 11 passed, 420 filtered, 5.50 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  8 passed, 423 filtered, 0.41 seconds (includes worker_admission/progress).
- rtk cargo test --offline --locked --lib remote::transcript -- --test-threads=1:
  3 passed, 428 filtered, 1.37 seconds.
- rtk cargo test --offline --locked --lib remote::worker_progress -- --test-threads=1:
  4 passed, 427 filtered, 0.14 seconds, also included in the worker substring run.
- rtk cargo test --offline --locked --lib quota_intent_tests -- --test-threads=1:
  13 passed, 418 filtered, 1.61 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host -- --test-threads=1:
  8 passed, 423 filtered, 0.84 seconds.
- rtk cargo test --offline --locked --test core_session --test remote_workspace
  --test remote_worktree_runner --test remote_worker_owner --test remote_parity --test runtime
  -- --test-threads=1: 142 passed, six suites, 35.38 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text --test headless_project_workspace
  --test tui_project_workspace --test tui_composer --test remote_concurrency --test remote_migration
  --test remote_subtab --test remote_worktree -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel --test-threads=1:
  168 passed, three TCP fixtures excluded, eleven suites, 17.16 seconds.
- rtk cargo test --offline --locked --test remote_events --test session_recovery
  --test stage1_session_tree --test stage1_context_checkpoint -- --test-threads=1:
  reports 74 passed, two ignored, 26 filtered, six suites, 3.39 seconds. Two
  ignored child helpers are invoked by parent tests; RTK aggregates nested
  process suite reporting, so this is not a unique parent-case count.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.070 seconds.
- cargo fmt --all --check; git diff --check; /usr/bin/python3
  tools/validate_stage2_android2unix_blueprint.py --strict pass; all 36 incomplete.

The first compile observed missing initialization in two headless ProcessResult
constructors; both now initialize the evidence to None. The first tool fixture
used the wrong builtin name read instead of read_file; corrected to the actual
registered name, then the full controller suite passed. Failed runs are not
counted as passes and terminal states were observed before final reruns.
Android source unchanged; no new JVM/device/build claim. No restricted TCP/
Gradle/adb bypass. Evidence provisional, not acceptance-grade stdout/stderr hashes.
All observed command handles terminal; no observation timeout caused duplicate
execution.

SHA256:

```text
src/core.rs c86bf96227d99d8f70d0c46430bfac10cb4a346b1ed8428eb5d96e7ab6a7fe6e
src/headless.rs 585a7dc1125025b1bb594910e2e1f7f6a06ff6c5e1cd89028d2282eb6710de3a
src/remote/worktree_runner.rs 3b0be03f32f9d24ddc22eba31d1000548b80a4c4721f42bf49cc083e57e9582e
src/remote/worker_progress.rs 297dce6dc9f8a052dd63575680bcf9a40d49053453dabc5b1700fc0b64c39c87
src/remote/transcript.rs f24a0095431eb45d28a90662c5b4d7ad2999c971274d080d27298fb4d7f566fe
src/remote/worktree_host.rs 7f9fce9fc94ceffba36e6940cd8b23698dde688b9fc19f3375cca08472e1ef8c
protocol a6888b2dc695717eda72937b59032498568d19a5c337da2ca7e65fd37e206f07
```

## Remaining Scope

This does not protect against a local operator rewriting both master and worker
journals. Failed/cancelled partial history, worker event/cursor replay, legacy
manifest migration, local TUI worker controls, settings and interactive PTY
remain unfinished. API31/34/35 camera QR, actual LAN desktop synchronization,
disconnect/replay/process lifecycle and signed reproducible release remain
unverified. Goal active; no Blueprint item accepted.
