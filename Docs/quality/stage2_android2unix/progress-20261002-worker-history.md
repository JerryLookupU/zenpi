# Task-Scoped Worker History

Provisional S2-019/S2-020/S2-036 progress observed 2026-10-02T10:42Z.
Previous goal turn made progress (durable worker output binding). Current
worktree inspected; HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. User
changes preserved. No nested agents, execution-worker admission, commits,
pushes, cleanup or Blueprint acceptance promotion.

## Implementation

The observed worker pipeline now records its newly appended task turns as an
ordered ID/role/raw-content-digest manifest and records the admitted input ID.
Master success metadata independently retains that input ID, excluded from
Android status projections. History reads verify the manifest against a
contiguous journal range beginning with the original user input and ending
with the already dual-bound final assistant output. The rest of a reused
worker journal is never included merely because it belongs to the same slot.

TranscriptRead adds two strict task-scoped forms:

- request_id/view=history/offset/digest: up to 16 messages per page, at most
  1024 task turns, 2048-byte redacted UTF-8 previews, digest/count continuity.
- request_id/turn_id/offset/digest: full message byte pages only for a turn in
  the validated task manifest, preserving the ordinary 8192-byte content pages.

Existing device/project/session authorization, succeeded task requirement,
8 MiB journal scan/reader bound, content limits and final-output correlation
remain unchanged. Unknown arbitrary turn IDs and replaced manifest content
are refused. Failed/cancelled/legacy tasks without manifests are not guessed.

Android adds a task-history icon and dialog with roles, next/previous controls,
seven bounded prior pages, and individual full-message opening through the
task route. Strict Kotlin decoding validates exact fields, integer bounds,
request/subtab/session scope, message roles/IDs, byte limits and continuity.
Current role is retained when opening a user or tool message rather than
assuming every worker message is an assistant answer. Lifecycle/scope/authority
loss cancels/invalidates the history view; local reset joins its outstanding job.
Pending writes and offline state disable read/page controls. No new persistence
of full task content or Android execution is introduced.

## Behavioral Evidence

The production observed worker test now executes two devices' tasks sequentially
through the same slot. Only the first device's two-message task range is
returned; the other device's marker is absent. Arbitrary turn and foreign-device
reads are refused. The selected user message can be read in full. Replacing
input content invalidates the history manifest, and checkpoint restore retains
the original history. Existing no-worker/read-only/output checks remain.

A new Rust unit fixture pages 35 tool messages across the 16-message boundary,
checks UTF-8/redaction/preview bounds and rejects stale digests/invalid offsets.
Two new JVM cases cover read-only history requests, opening individual user
messages, wrong scope/role/limits, and digest-bound continuation. A new Compose
instrumentation source fixture exercises role display, selected-message routing,
next-page callback and offline disabling; compiled, not executed on a device.
Actual provider tool-history behavior is not proven by the Echo fixture.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-worker-history.
Completed commands exit 0:

- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  final source 10 passed, 420 filtered, 5.21 seconds.
- rtk cargo test --offline --locked --lib remote::transcript -- --test-threads=1:
  final source 3 passed, 427 filtered, 1.36 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  8 passed, 422 filtered, 0.59 seconds (includes worker_progress/admission).
- rtk cargo test --offline --locked --lib quota_intent_tests -- --test-threads=1:
  13 passed, 417 filtered, 2.96 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host -- --test-threads=1:
  8 passed, 422 filtered, 1.17 seconds.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test remote_concurrency --test remote_parity --test runtime
  -- --test-threads=1: 115 passed, six suites, 37.57 seconds. Final added
  cross-device/replaced-input assertions are covered by the final lib rerun.
- rtk cargo test --offline --locked --test remote_workspace -- --test-threads=1:
  final source 58 passed, 30.17 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text --test headless_project_workspace
  --test tui_project_workspace --test tui_composer --test remote_migration --test remote_subtab
  --test remote_worktree -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel --test-threads=1:
  156 passed, three TCP fixtures excluded, ten suites, 16.64 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- /bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-worker-history-android:
  final main/unit/instrumented source compilation and 240 JVM tests pass,
  1.127 seconds JVM runtime. Cached Kotlin/Compose/API35 artifacts, not Gradle,
  APK packaging or device execution.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.065 seconds.
- cargo fmt --all --check; git diff --check; /usr/bin/python3
  tools/validate_stage2_android2unix_blueprint.py --strict pass, all 36 incomplete.

Unique successful Rust cases: 313 (repeated 58 workspace cases not added twice).
All observed command handles terminal; no observation timeout started duplicate
execution. No restricted TCP/Gradle/adb bypass. Results provisional, not acceptance-grade
stdout/stderr-hashed receipts. Android artifacts:

```text
main.jar 29a0f8ad2575e6afff555a8468fb571193f91c8e5b76f94cf00325d14e6c08aa
unit.jar 73c9f9da6b08b6044d202ef00e73dc4c7d284954b25d50d8dd3e55e5298c5129
instrumented.jar cce8a68b101f80e1111445188e741dd280493ef08833b1f6e8f9fc1a1748b8a6
src/remote/worktree_runner.rs 64a7600aeda7d64bd99cd3d0d25c190428bf95bd2dc79aae0ec85c38c3b77d59
src/remote/worker_progress.rs b2d4c21cc1027b332d68250bc5b60001daaf69529e60d0474cde5510542d20f7
src/remote/transcript.rs 814a77209d9138f373408fa2fbcc96d247cf639a34396578eb70a5c645db1a9d
src/remote/worktree_host.rs fed4e6c4fb593d1f24f916ae9ddbad905538e1e8aea01308b0364b9ea773ce06
WorkerHistoryCodec.kt c19ad740aeb25acd3de4776560e8f1155cbb9b154524d27d4efe0502a1eede82
PairingViewModel.kt e50e14de1e057e9182095f728950e7344c95e086ff6cb4279c53ac17d24fc3f3
MainActivity.kt 5202a3925d50a97e8a8a0c8e153d8689383bf6279fda8b2e66b8e8870dcdaff9
```

## Remaining Scope

Successful task history is not failed/cancelled partial history, tool event/cursor
replay, live branch projection, manifest migration or full parity acceptance.
Local TUI worker controls, settings and interactive PTY remain unfinished.
API31/34/35 camera QR, actual LAN desktop sync, disconnect/replay/process
lifecycle and signed reproducible release remain unverified. Goal active;
no Blueprint item accepted.
