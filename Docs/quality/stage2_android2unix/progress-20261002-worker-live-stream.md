# Correlated Worker Live Text

Provisional S2-019/S2-020/S2-036 progress observed 2026-10-02T08:40Z.
Previous turn made progress: completed worker output. HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty worktree preserved. All 36
Blueprint rows remain [ ]; goal active. No nested agents, controller worker
admission, commit, push, cleanup or acceptance promotion.

## Implementation

WorktreeHost now submits WorkerPrompt containing text and immutable envelope/
device correlation. Its observed runner calls the real Agent process_with_cancel_
and_events pipeline under the existing independent worker/root execution owner.
Provider TextDelta/TextDone update a lane-owned LivePromptText map. Keys are
device/project/master-session/request, separate from worker journal identity.
String-input public WorktreeRunner callers remain supported by the original
path. Worker admission takes scope/quota independently of input type, preserving
the same ACL, checkpoint binding, exact receipt and revision gates for both.

Listing decorates only accepted/running statuses with bounded stream_text and
optional true stream_truncated. Reuse the existing complete-line redactor/raw
4KiB cap/NUL withholding/ResponseCreated reset. Partial secret lines are not
published. Live preview is never appended to master progress journals or used
as a transcript cursor. Runtime completion/rejection removes its map entry;
terminal status uses the previous confirmed-result output projection. Lanes
drop live maps on shutdown, and restart does not fabricate preview.

Android reuses PromptStatusCodec's strict state/text/truncation validation inside
WorkerStatusCodec. UI renders running worker text and explicit preview truncation.
Cache v3 drops previews and worker authority; malformed subscription updates are
refused atomically and text previews do not advance transcript/journal cursors.

Two new Rust controller tests cover actual production Agent/Echo provider-event
delivery with redaction and empty master turns, plus two concurrently held tasks
owned by different devices. The second checks independent previews, split secret
redaction, no durable stream field, one-target cancellation cleanup and surviving
other-device stream. Its held job closure is a bounded deterministic local fixture,
not a provider HTTP or LAN fixture. One JVM test checks live field/state/flag/byte
refusal and cache exclusion; subscription and compiled Compose tests now include
worker preview. Existing LivePromptText tests exercise PEM byte splits, overflow,
UTF-8, reset and capacity. No device-run claim.

## Verification

Cwd /Users/mac/Github/zenpi; Rust environment
ZENPI_HOME=/private/tmp/zenpi-worker-stream-final. Successful commands exit 0:

- rtk cargo test --offline --locked --test remote_workspace
  --test remote_worktree_runner --test remote_worker_owner --test remote_concurrency
  --test remote_parity -- --test-threads=1: 93 passed, five suites, 32.77 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  6 passed, 400 filtered, 0.35 seconds.
- rtk cargo test --offline --locked --lib remote::worktree_host
  -- --test-threads=1: final 3 passed, 403 filtered, 0.52 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host
  -- --test-threads=1: 8 passed, 398 filtered, 0.82 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands
  --test remote_auth --test remote_approval_wakeup --test remote_live_text -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit
  --skip production_plain_host_routes_authenticated_tls_prompt_to_the_provider
  --test-threads=1: 31 passed, four suites, two TCP fixtures excluded, 3.40 seconds.
- cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: 103 passed (19/28/56), one TCP fixture excluded.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-worker-stream-android-final: main/unit/instrumented source
  compiled; 231 JVM tests passed, 1.121 seconds. Not Gradle/APK/device evidence.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  63 passed. cargo fmt --all -- --check, git diff --check and strict Blueprint
  checker pass; 36 incomplete items. Unique successful Rust cases: 244.

Earlier focused tests/JVM run passed and are not added again. All live sessions
were observed terminal; no timeout caused duplicate execution. No restricted
TCP/Gradle/adb bypass or current device claim.

SHA256:

```text
src/remote/worker_admission.rs 321aabe28c7cd3cc1b42f0d8e58012bba337e76ecf84a441e3e94f81236e309d
src/remote/worktree_runner.rs 947c151c7dd8ac8e0116ef6102131009270fd34a72d2b95cbdd8ddef3159a4e2
src/remote/worktree_host.rs a0d4534fbe73907ffdd74f3578ebf45aee9c9e28c5e685d1939ba0285d995034
src/remote/worker_progress.rs 7cafc6b9d321d3e3b47eae8e70b89248704f1298ff6a12b4b7d1a1d679723b0b
android/app/src/main/java/zenpi/transport/WorkerStatusCodec.kt e247dcf5525b1b22cc19f57d221e68650ed07353db27aaafc8b87241a99ffe9d
android/app/src/main/java/zenpi/MainActivity.kt 77222bd89e7048c98b8061066c8b20c3c3443d46ecb650c054c20612405ae2d2
android/app/src/test/java/zenpi/WorkerControlTest.kt cf78e9027a3f8c766a5aeea12995f00fcd9f774714e9d2b1275c7b936ba4bee0
android/app/src/test/java/zenpi/SubscriptionTest.kt 0d84be744b2937784677dc9990e04fab8efd3b99ea626f17aa15bac8823a407e
android/app/src/androidTest/java/zenpi/WorkerScreenTest.kt ad8d294676b976b024fb9261b1be8eca98778130641b1174e074bde2e16e25c5
Docs/stage2_android2unix_protocol.md e7671fa06a6cafe6f3d686def21d384cec123bf2230113845c19647336694f06
main.jar dd0d09c4eeb2aa6493692fa0d771de2c5edd20ee8e6d9e402eccadab5d6356e3
unit.jar 01c7b24bdbd1c3604a5ccd55d60b8e81aa82344743652253b90789626fb98145
instrumented.jar 55b1a263a82ae57022288bb51bc56b1b85618afafdf7928138e2802315434468
```

## Remaining Scope

This adds worker text preview, not complete worker transcript paging or arbitrary
tool-event parity. Effective live quota controls/stepper, local TUI worker task
and exact worker approval UI, settings and interactive PTY remain unfinished.
Current APK API31/34/35 camera QR, actual LAN desktop sync, disconnect/replay/
process lifecycle and reproducible signed release remain unverified. Shared
redaction cannot guarantee detection of arbitrary unknown/obfuscated secrets.
Provisional evidence lacks acceptance-grade stdout/stderr receipts; full goal
remains active and no Blueprint item is promoted.
