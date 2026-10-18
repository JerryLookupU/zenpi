# Independent Worker Runtime Limits

Provisional S2-018 progress observed 2026-10-02T08:58Z. Previous goal turn
made progress: idle remote quota controls. HEAD remains
1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty user worktree preserved.
All 36 authoritative Blueprint rows remain [ ]. No nested agents, worker
admission, commit, push, cleanup or acceptance promotion.

## Implementation

BackgroundRunner now publishes the last scheduler-applied execution limit
through an Arc<AtomicU16>, initialized from spawn configuration. The scheduler
stores a new limit before resolving its application receipt. Command-channel
admission, blocked event delivery and ignored shutdown commands do not change
this value. Dropping a receipt does not discard the authoritative observation.
This is a last-applied limit, not active-thread count or runtime liveness.

WorktreeRunner exposes the same observation and a host-only nonblocking quota
adjustment within already leased capacity. quota() still reports that capacity;
scope and durable state are unchanged. Lowering preserves all independent
worker claims and running work; pending starts wait below the applied limit.
Raising reuses the same Agents/journals, never admits beyond leased capacity.
Growing ownership, durable live resize, remote effective receipts and runtime
admission scope migration remain separate unfinished work. The production
remote SetConcurrency path still refuses live lanes, so temporary runtime
adjustment is not advertised as durable Android live resizing.

Two new worker-runner tests cover held real Agent turns over seven queued jobs:
three independent writers, drain to one without cancelling active jobs, FIFO
pending starts, raise back to three using original writer identities, no N+1
start, exactly one terminal and one journal per request, no master turn, no
checkpoint change and retained ownership fences. The second covers event
backpressure and shutdown refusing unapplied changes. Existing detached-job
and restart tests now lower the limit before shutdown: claims/leases survive
until actual detached exit, and restart uses durable capacity rather than
misrepresenting a runtime-only limit as persisted. A new generic runtime test
covers discarded receipt and malformed-limit refusal. Existing runtime quota
tests assert scheduler-applied snapshots after acknowledgement and under
backpressure. Fixtures are local deterministic jobs and the real Agent/Echo
pipeline; they are not production HTTP/LAN or Android-device evidence.

## Verification

Cwd /Users/mac/Github/zenpi. Rust environment
ZENPI_HOME=/private/tmp/zenpi-worker-live-quota. Final commands exit 0:

- rtk cargo test --offline --locked --test runtime --test remote_concurrency
  --test remote_worktree_runner -- --test-threads=1: 38 passed across three
  suites (15/12/11), 3.77 seconds.
- rtk cargo test --offline --locked --test remote_workspace
  --test remote_worker_owner --test remote_parity -- --test-threads=1:
  74 passed across three suites, 24.64 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  6 passed, 400 filtered, 0.39 seconds.
- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  3 passed, 403 filtered, 0.64 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host -- --test-threads=1:
  8 passed, 398 filtered, 0.86 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider
  --test-threads=1: 31 passed, two TCP fixtures excluded, 3.56 seconds.
- rtk cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: 103 passed, one TCP fixture excluded, 6.00 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- env PYTHONPATH=tools python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.046 seconds.
- cargo fmt --all -- --check, git diff --check and
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: pass;
  strict validator reports all 36 items incomplete.

Unique successful Rust cases: 263. Earlier focused 23-case run is not counted
again. All observed process handles terminal, no timeout-triggered restart.
Android source unchanged this turn; no new JVM, Gradle, APK or device claim.
No restricted TCP/Gradle/adb bypass. Provisional progress results do not replace
the Blueprint's acceptance-grade exact stdout/stderr-hashed receipts.

SHA256:

```text
src/runtime.rs 1c792ac47812b47a7dffe83907c6f8c76a4eba53730ec4c8480441fd0a88b56d
src/remote/worktree_runner.rs 20a191cac1c6c51372e369cb60f612795d8d1a32b18daa0b867c017129b3d9e4
tests/remote_concurrency.rs 0128b9c37387bf30935c0d0de0668b8a3f1ca1cccaa1d22cda1194ebc9e13da1
tests/remote_worktree_runner.rs 15232554364c98ab2b7a933b3176a91c812e7131026474f5426238f84cd2bc59
protocol 3411364b9ac085a78bb30d1f0257e6eee7c8e19830935e962f3005040b312b5d
blueprint d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
specification 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
```

## Remaining Scope

Remote live resize requires a durable intent/receipt lifecycle, rollback or
unknown-outcome recovery if runtime application and checkpoint commit differ,
new independent claims for growth and synchronized prompt admission scope.
Local TUI worker controls, full event/transcript parity, settings and interactive
PTY remain incomplete. Current APK API31/34/35 QR camera, actual LAN desktop
sync, disconnect/replay/process lifecycle and reproducible signed release are
still unverified. The full objective remains active, not replaced with these
host primitives, and S2-018 is not accepted.
