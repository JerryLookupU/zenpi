# Headless Shutdown Result Retention

Provisional S2-014/S2-019 hardening, observed 2026-10-02T01:40:24Z.
Previous goal development turn made concrete project-lane progress. RTK user
request was handled separately; no completeness claim for the Stage 2 goal.
All 36 authoritative rows remain incomplete and the full goal stays active.

## Changes

The existing shutdown helper discarded terminal events, and HeadlessHost then
appended unknown_outcome to every active lane even when its prompt had already
completed successfully. Add BackgroundRunner::shutdown_and_join_with_events:
it observes events during admission retry and shutdown, joins the worker, then
drains remaining buffered events. The authoritative closed bit alone is not
evidence that the host has consumed the buffered terminal result. Existing
shutdown helper signatures and defaults remain compatible.

Headless shutdown observes matching local job IDs, retains confirmed runtime
success/failure, and persists them through the original Agent session writer.
Missing results, join failure and Cancelled retain unknown_outcome: generic
runtime cancellation still cannot distinguish cooperative return from detach.
No new claim that detached Rust code stops its side effects. All project lanes
are signalled before joining, as before. Admission now uses MAX_PROJECT_TABS
instead of duplicating the literal host bound. Protocol frontier updated.

## Verification

Cwd /Users/mac/Github/zenpi. Commands use --offline --locked; no network or
device acceptance is inferred from local fixtures.

- cargo test --offline --locked --lib
  remote::headless_host::tests::shutdown_retains_success_from_a_completed_real_prompt_before_owner_poll:
  **1 pass**. Actual prompt_runner executes Agent + Echo + configured host
  tools, without owner-loop terminal polling. Shutdown preserves succeeded,
  and reopening the journal retains it. This is not a real HTTP provider gate.
- env ZENPI_HOME=/private/tmp/zenpi-shutdown-progress-20261002 cargo test
  --offline --locked --test runtime --test remote_workspace: **37 pass**
  (15 runtime / 22 workspace). Three new runtime observer tests cover late
  semantic completion, buffered success after runner already exits, saturated
  command/event queues and ordered terminal delivery. Existing detach test
  now checks cancellation observation and waits for released fixture return.
- cargo test --offline --locked --test remote_headless --test remote_parity
  --test remote_subscription --test remote_subscription_tls --test remote_workspace
  --test runtime: first invocation passed the first four suites **25 tests**,
  then workspace had 4 configuration-I/O sandbox failures and did not reach
  runtime. Corrected ZENPI_HOME invocation above passed all workspace/runtime
  tests without changing product behavior. TLS suite uses actual in-memory
  rustls, not a TCP listener or physical LAN.
- Focused remote_headless_commands tests headless_cancel, parallel_project
  and independent_project: **3 pass**. Cancel test now restarts HeadlessHost
  over the same owner and checks shutdown unknown_outcome is durable without
  a third provider invocation. Total distinct passing Rust tests: **66**.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint: **50 pass**. Initial
  invocation omitted PYTHONPATH and failed module imports; corrected command
  resolves repository sibling imports, no test code changed.
- Strict blueprint validator passes with 36 [ ] rows; cargo fmt --all --check
  and git diff --check pass. Existing Rust warnings are unchanged.

Initial new Agent fixture lacked host tools, so selected-workspace execution
correctly refused and the fixture's completion wait timed out. Fixture now
uses the same tool setup as configured headless owners; no permission bypass
or product fallback added.

SHA-256:

```text
src/runtime.rs 40dfb8b5660a38661b9cd44529a37cef8ef61d3406a1d68e8fb4191a64609feb
tests/runtime.rs e91cbcc2138e99dd0310460b1b29a2c3e730ef8c9d8c2f9b2db0b20d88ae2155
src/remote/headless_host.rs 522525aaa8db06c60fa6aa1b30b409ab774e794ba0626aff6404d9fa5f402cbb
tests/remote_headless_commands.rs d726fad4c4faa075224f7fe78ce86139199e03bba33892864cb86fb94550b724
```

Android sources unchanged; no fresh APK, API31/34/35 device, camera QR or LAN
run in this turn. Per-worktree scheduler quotas, full TUI/headless parity,
stream delta replay/backpressure, interactive PTY, physical QR/LAN sync and
release/migration gates remain. No agents, commit/push or acceptance promotion;
existing user changes preserved.
