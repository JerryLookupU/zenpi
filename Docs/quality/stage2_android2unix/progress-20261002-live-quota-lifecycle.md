# Live Quota Lifecycle Regression

Provisional S2-018/S2-019/S2-035 progress observed 2026-10-02T09:47Z.
Previous turn made progress: remote live-quota routing, but broad regression
was incomplete. Current source inspected; HEAD remains
1a8ff1acf72e0816fad503a05df2b1d18e07361b and dirty user changes preserved.
All 36 Blueprint items remain [ ]. No nested agents, execution-worker admission,
commit, push, cleanup or acceptance promotion.

## Changes And Behavioral Evidence

Two new WorktreeHost tests exercise the real controller entry rather than only
the low-level scheduler. A quick real Agent/Echo task completes before explicit
quota confirmation: event polling must keep the pending lane/intent, exact retry
settles, and the next poll releases all claims and retires the lane. Original
receipt and completed task status survive restart without new workers or master
turns. This directly verifies the production pending-lane guard added last turn.

The lowering test holds three independent writer sessions and two queued tasks.
Remote SetConcurrency commits one while all three are held. Retained capacity
stays three, configured/applied limit becomes one, and released slots do not
start queued work while the live count remains at or above one. Queued jobs
then start one at a time, all five original requests succeed and normal lane
retirement releases retained workers. This verifies preservation and the no-new-
start counterexample at the remote entry, not just a configuration assertion.

A new Android JVM test sends fragmented quota response frames through
ResponseReader and WorkspaceReceiptCodec. Unknown/accepted/running responses
with otherwise valid-looking results cannot confirm; succeeded alone confirms.
Restoring the saved envelope preserves exact identity. No production behavior
changed this turn. Protocol records these boundaries without claiming device
or LAN validation.

## Verification

Cwd /Users/mac/Github/zenpi, Rust environment
ZENPI_HOME=/private/tmp/zenpi-live-quota-lifecycle. Completed commands exit 0:

- /Users/mac/.rustup/toolchains/1.98.0-aarch64-apple-darwin/bin/cargo test
  --offline --locked --test remote_workspace -- --test-threads=1:
  all 58 passed, 21.75 seconds. This closes the prior interrupted workspace
  regression, including post_git_checkpoint_conflict and desktop live quota.
- Same direct-toolchain cargo test --offline --locked --lib remote::worktree_host
  -- --test-threads=1: 7 passed, 415 filtered, 1.62 seconds, including both new cases.
- rtk cargo test --offline --locked --test remote_worktree_runner --test remote_worker_owner
  --test remote_concurrency --test remote_parity --test runtime -- --test-threads=1:
  55 passed, five suites, 6.26 seconds.
- rtk cargo test --offline --locked --lib quota_intent_tests -- --test-threads=1:
  11 passed, 411 filtered, 1.35 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  7 passed, 415 filtered, 0.36 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host -- --test-threads=1:
  8 passed, 414 filtered, 0.69 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider
  --test-threads=1: 31 passed, two TCP fixtures excluded, 3.53 seconds.
- rtk cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: 103 passed, one TCP fixture excluded, 6.08 seconds.
- rtk cargo test --offline --locked --test remote_migration --test remote_subtab
  --test remote_worktree -- --test-threads=1: 22 passed, three suites, 6.78 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- /bin/zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-live-quota-lifecycle-android: main/unit/instrumented sources
  compiled, 235 JVM tests passed, 1.142 seconds. Not Gradle/APK/device evidence.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.073 seconds.
- Direct toolchain cargo fmt --all -- --check, git diff --check and
  /usr/bin/python3 tools/validate_stage2_android2unix_blueprint.py --strict pass;
  strict checker reports 36 incomplete items.

Unique successful Rust cases: 302. No interrupted/partial test totals included.
All observed handles terminal; no observation timeout triggered duplicate work.
No restricted TCP/Gradle/adb bypass. Results are provisional, not acceptance-grade
exact stdout/stderr-hashed receipts.

SHA256 before this documentation addition:

```text
src/remote/worktree_host.rs df08d2570fd1b2fd32810cc15970468cf639a2ec698fa718708b3cde35cd33ed
ConcurrencyControlTest.kt 6b80577f946aaa882a4c216479ed81ad971ced3220f6abc7996351ef88db55b9
blueprint d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
specification 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
main.jar 190aa105034acc696349a8f71d9d42dbf4e0f0aaf4972ff31958e9f9ae271c26
unit.jar fb204aae471f37450602329bed26e3dd2c3c187b5abf35a8db63e927733707b2
instrumented.jar 1c3f80d495e2a6f72d40beeebc8767ab5ba73338d2561d91bb616b5d596905ac
```

## Remaining Scope

These tests close specific live-quota lifecycle and queue gaps, not full
64-owner/cross-device/API acceptance. Full local TUI worker controls, event/
transcript parity, settings and interactive PTY remain unfinished. Current APK
API31/34/35 camera QR, actual LAN desktop sync, disconnect/replay/process lifecycle
and reproducible signed release remain unverified. Full objective remains active;
no Blueprint item accepted.
