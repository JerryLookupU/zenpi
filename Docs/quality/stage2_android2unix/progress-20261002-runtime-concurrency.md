# Bounded Runtime Concurrency

Provisional S2-018 progress observed 2026-10-02T06:51:52Z. Previous goal turn
made progress: confirmed RTK and obtained final transcript-navigation results.
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty changes preserved.
All 36 authoritative rows remain [ ]; full goal active. No worker admission,
subagents, commit, push, cleanup or acceptance promotion.

## Implementation

BackgroundRunner now has spawn_with_concurrency with strict 1..64 validation.
Ordinary spawn remains quota one; RuntimeConfig and RuntimeEvent public shapes
are unchanged. The existing real runtime starts multiple independent job
threads up to its quota, retains bounded FIFO pending work and joins a finished
job before reusing its slot. It does not clone Agent or session writers.

try_set_concurrency returns a nonblocking command-application receipt. The
receipt resolves only when the scheduler applies the value, not merely when
the command enters its channel. Invalid limits leave it unchanged. Raising
starts oldest pending jobs without waiting for every active job; lowering
preserves active work, starting nothing more until below the new limit.
An existing live count may exceed a newly lowered quota while draining.

Cancellation targets an active or queued job without stopping others. Panic
still emits one terminal and frees its slot. Shutdown signals all active
tokens before any blocking event send, uses a shared deadline across jobs,
never starts pending work and emits all active terminals before cancelling
queued requests and closing. Detached noncooperative code retains the existing
unknown-side-effect limitation; it is not forcibly killed or rolled back.

Eleven new tests run real runtime threads: exact quota and N+1 exclusion at
1/2/7/64, FIFO growth/drain, lowering while busy, pending queue overflow,
scoped cancellation, invalid limits, delayed application receipt under event
backpressure, shutdown refusing application, one deadline for eight blocked
jobs, panic survival and all-token cancellation with a saturated event queue.
An independent-Agent integration test also exercises real write_file tool
execution in three separate workspaces and session reopen after completion,
with no cross-journal output. Its controlled backend is deterministic, not
provider HTTP or a production host/network acceptance gate.

## Verification

Cwd /Users/mac/Github/zenpi. Final commands exit 0:

- env ZENPI_HOME=/private/tmp/zenpi-runtime-concurrency-final rtk cargo test
  --offline --locked --test runtime --test remote_concurrency
  --test remote_subscription_tls --test remote_workspace --test remote_parity:
  **107 tests**, 5 suites, 9.79 seconds. Includes 11 new tests and existing
  serial-runtime, owner-domain and encrypted rustls memory-stream regressions.
- Same environment, rtk cargo test --offline --locked --test remote_concurrency
  -- --test-threads=1: **11 tests** again, 0.93 seconds.
- Same environment, rtk cargo test --offline --locked --lib
  `remote::headless_host::`: **8 tests**, 389 filtered out.
- env ZENPI_HOME=/private/tmp/zenpi-runtime-concurrency rtk cargo test --offline
  --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit
  --skip production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  **8 tests**, 2 restricted TCP tests intentionally excluded.
- Same final environment, rtk cargo test --offline --locked
  --test tui_project_workspace --test tui_composer: **84 tests**.
- Same final environment, rtk cargo test --offline --locked
  --test headless_project_workspace -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel:
  **19 tests**, 1 restricted TCP test intentionally excluded.
- Same final environment, rtk cargo check --offline --locked: zero errors,
  20 existing warnings. cargo fmt --all -- --check and git diff --check exit 0.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  **63 tests**. python3 tools/validate_stage2_android2unix_blueprint.py --strict
  exits 0 and reports all 36 rows incomplete.

Unique Rust tests above: 226; the serial rerun is not added to that total.
An initial expanded run exposed a test-only TurnInputRequest type error;
corrected before the final 107-test run. A proposed unfiltered legacy workspace
run was stopped during compilation (exit 130) when its TCP fixture was found;
the filtered completed run replaces it, without restricted socket retries.
No Android source changed this turn; the separately recorded previous-page
219-test compilation is not a fresh APK/device result for concurrency.

SHA256 at verification:

```text
src/runtime.rs 86444a83cef941fbbf225c6b6c793d1c64e0dcc678cb2b70656b270b446781a6
tests/remote_concurrency.rs a06bb69e810132272944b8145ab3337ab0cebda1c425d509276e8fb30893e9d5
Docs/stage2_android2unix_protocol.md 5a8de30f33a52fcbc4d20b35bf2ae8a69b6cb8c694824a370426d71d070025c2
```

## Residual Scope

This is real shared-runtime scheduling support, not effective per-worktree
concurrency. Need independent worktree worker ownership/configuration,
host-owned quota persistence plus CAS/receipt reconciliation, admission after
restart, and unified TUI/headless/Android routing. Current headless prompt
ownership remains one Agent per project; concurrency_control stays false.
S2-018 is not accepted. Do not enable remote mutation from this helper alone.
Full native Android camera QR, actual same-LAN desktop synchronization,
API31/34/35 lifecycle, current APK, settings/interactive PTY and signed
reproducible release remain unfinished or unverified. Restricted network,
Gradle and adb constraints were not bypassed. Full objective stays active.
