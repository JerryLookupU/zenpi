# Completed Worker Output Synchronization

Provisional S2-019/S2-020/S2-036 progress, 2026-10-02T08:33Z. Previous goal turn
made progress: desktop remote worktree route. HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty worktree preserved. All 36
Blueprint rows remain [ ]; full objective active. No nested agents, controller
worker admission, commit, push, cleanup or acceptance promotion.

## Implementation

WorkerProgress now projects assistant content from confirmed successful runtime
ProcessResult into the bounded device/request/project/master-session/subtab
progress record. Redact the entire result before UTF-8-safe display truncation,
not a sliced raw prefix whose secret assignment/PEM marker could be fragmented.
Raw results up to 64KiB use the existing registered/conventional secret and PEM
redactor; safe output is capped at 4096 bytes. Larger or NUL bodies produce only
output_withheld=true. Text and boolean output_truncated occur together.

The master progress journal persists redacted output, so the same headless and
desktop worker listing survives restart without resubmission or opening worker
journals. Durable listing revalidates and redacts output again. Raw worker
journals remain untouched and private; an integration fixture checks that raw
private content remains locally present while remote output excludes it.
Failure/cancellation/unknown state has no output. Ambiguous append removes
success text and all output flags from live UnknownOutcome. Device-scoped
listing and current authorization remain unchanged.

Android's strict WorkerStatusCodec accepts output_text plus boolean truncation
or sole true withheld flag only on succeeded tasks. Partial/mixed/string-flag,
oversized/NUL and output on running/failed tasks are refused. Task display shows
the result and explicit truncation/withholding state. Subscription reduction
shows completion output without inventing master turns or advancing transcript
cursor. Cache v3 still discards all worker display/authority; no output restores
an offline execution permission. Admission receipts are unchanged.

New Rust unit test checks secret/PEM redaction, a credential crossing the display
boundary, multibyte UTF-8 truncation, oversized/NUL withholding. A real worker
integration runs Echo in independent leased owners, checks device isolation,
redacted output persistence/restart and unchanged raw worker journal. Existing
contention/restart and ambiguous append tests now check output survival/removal.
Actual headless mailbox and desktop route success fixtures require synchronized
output in their listing. One new JVM test covers exact output fields and cache
exclusion; subscription test now includes completed output and atomic invalid
follow-up. One added Compose test checks result/truncated/withheld display at
320dp without send/cancel controls; it was compiled, not device-executed.

## Verification

Cwd /Users/mac/Github/zenpi. Rust environment
ZENPI_HOME=/private/tmp/zenpi-worker-output-final (initial checks use
/private/tmp/zenpi-worker-output). Final observed exit 0 commands:

- rtk cargo test --offline --locked --test remote_workspace
  --test remote_worktree_runner --test remote_worker_owner --test remote_parity
  --test remote_concurrency -- --test-threads=1: 93 tests (56/9/12/5/11), five
  suites, 34.91 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  6 passed, 398 filtered, 0.26 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host
  -- --test-threads=1: 8 passed, 396 filtered, 0.81 seconds.
- rtk cargo test --offline --locked --lib remote::worktree_host
  -- --test-threads=1: 1 passed, 403 filtered, 0.28 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands
  --test remote_auth --test remote_approval_wakeup -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit
  --skip production_plain_host_routes_authenticated_tls_prompt_to_the_provider
  --test-threads=1: 21 passed, two restricted TCP fixtures excluded, 3.42 seconds.
- cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: 103 passed (19/28/56), one restricted TCP fixture excluded.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-worker-output-android-final: main/unit/instrumented sources
  compiled; 230 JVM tests passed, 1.143 seconds. Not Gradle/APK/device execution.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  63 passed. cargo fmt --all -- --check, git diff --check and strict Blueprint
  validation pass, 36 incomplete items. Unique successful Rust cases: 232.

Focused output integration and initial Android checks also passed, not added
again. All live test handles observed to completion; build lock/observation
waiting did not cause duplicate execution. No new device or socket attempt.

SHA256 at verification:

```text
src/remote/worker_progress.rs 3d30f21d9271edbbe52964d60ecd639ddc2eabc7a516ab608d23a88168a97b5e
tests/remote_workspace.rs 1866ce4ef7be27e0ca06cf9cf659469f0ba64802543f915a8218b403bbcbf484
android/app/src/main/java/zenpi/transport/WorkerStatusCodec.kt f32188af453d4d1aa397c78d982983172a4bbc381a9379782bebb73bdf2dc64a
android/app/src/main/java/zenpi/MainActivity.kt f7a41fc8822c4a545cd69ccd9db80ffb24f9b008481bad85fa5508acae36f212
android/app/src/test/java/zenpi/WorkerControlTest.kt 2bf24636d9fc8fcd7d61d0056f33d6c55653832b50a2f3a99eea19a79a89fc84
android/app/src/test/java/zenpi/SubscriptionTest.kt 23bf442465d0b77ce6aa65427eb21bd93bd8f78b24f68a4d42e46d917757089c
android/app/src/androidTest/java/zenpi/WorkerScreenTest.kt 781e102563655b5c7949240cca5df03fe8cdf892138d4f0f9f019f4717969e51
Docs/stage2_android2unix_protocol.md 4ae7f402bb5dadddfa7d35dcdf4175478fd6eb7be5cb5a90032127668c5bf331
main.jar 05e868c726eb3294c594c1e2cc7ea4fc059a210e8bc74dbcb749b416bb4b1c22
unit.jar 6cc34d7c1bde3a99419ae19c87c1538aaebe8eec05babe1c41e5db662a4ceff9
instrumented.jar 727ff8b3c5fafa45acc14e4af32da5006df31a2fe75db60f447578d62a94d511
```

## Remaining Scope

This is completed output synchronization, not live streaming, complete worker
transcript paging or tool-event parity. Local TUI worker task controls and exact
worker-owner approval UI, live effective quotas/stepper, full settings and
interactive PTY remain unfinished. Shared redactor does not promise detection
of arbitrary unknown/obfuscated secrets. Current APK/API31/34/35 camera QR,
actual LAN desktop sync, disconnect/replay/process lifecycle and reproducible
signed release remain unverified for this state. No restricted TCP/Gradle/adb
bypass. Provisional evidence lacks acceptance-grade stdout/stderr receipts;
the full objective remains active.
