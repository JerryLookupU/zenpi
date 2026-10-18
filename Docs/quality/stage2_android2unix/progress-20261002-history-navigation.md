# Android Task History Navigation

Provisional S2-036 progress observed 2026-10-02T10:57Z. Previous goal turn
made progress (history integrity and actual tool loop). Current worktree
inspected; HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing user
changes preserved. No nested agents, execution-worker admission, commits,
pushes, cleanup or Blueprint acceptance promotion.

## Fix And Behavioral Evidence

Opening an individual task-history message previously called broad read-view
cleanup, discarding the originating history page and previous-page stack.
Closing full text therefore could not return to the selected history position.
History backward navigation also trusted the last cached page without checking
scope/digest continuity as the master transcript navigation already did.

Message opening now clears only the active transcript read while preserving its
originating validated history page and stack. The history dialog is hidden while
the message dialog is open, not stacked behind it. Its scroll state lives above
the conditional dialog so closing full text returns to the same page/position.
An ordinary message dismissal cancels its read generation and clears full-text
state, restoring history only when current task/project/session/subtab and
selected message membership still match. Broad lifecycle/scope/authority
cleanup remains distinct and clears both views.

The production WorkerHistoryNavigation helper enforces same task/subtab/session/
digest/count, contiguous offsets and no repeated message IDs across retained
pages, bounded to seven previous pages. Backward navigation refuses mismatched
scope/pages; hidden history cannot advance behind an open message. Read request
continuations validate the current task subtab, and selected full-message
continuations retain actual turn ID/role. No persistent history cache is added.

Five new JVM tests cover bounded return stacks, mismatched scopes/digests/offsets/
duplicates, message dismissal restoring the same history, stale scope/message
refusal, malformed earlier stacks and wrong selected-message continuation.
Existing WorkerHistoryTest adds a wrong-subtab continuation check. New Compose
instrumentation source exercises full message opening, no stacked dialogs,
dismissal back to the same history page and disappearance of full-text content;
compiled, not device-executed. The UI state reducer is the same helper used by
the ViewModel and Compose test, not a separate mock navigation implementation.

## Verification

Cwd /Users/mac/Github/zenpi. Completed commands exit 0:

- /bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-history-navigation:
  final main/unit/instrumented source compilation and 245 JVM tests pass,
  1.130 seconds JVM runtime. Cached Kotlin/Compose/API35 compile, not Gradle,
  APK packaging or instrumented device execution.
- env ZENPI_HOME=/private/tmp/zenpi-history-navigation rtk cargo test --offline --locked
  --lib remote::worktree_host -- --test-threads=1: 11 passed, 420 filtered, 5.15 seconds.
- env ZENPI_HOME=/private/tmp/zenpi-history-navigation rtk cargo test --offline --locked
  --test remote_workspace --test remote_parity -- --test-threads=1:
  63 passed, two suites, 21.90 seconds. Rust source unchanged this turn.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.075 seconds.
- cargo fmt --all --check, git diff --check and /usr/bin/python3
  tools/validate_stage2_android2unix_blueprint.py --strict pass; all 36 incomplete.

All observed command handles terminal. No restricted TCP/Gradle/adb bypass.
Results remain provisional, not acceptance-grade stdout/stderr-hashed receipts.

SHA256:

```text
main.jar 5e7dea76372b8dac8f395e4273c97cf739f968807ebf2c717506819bd4f69619
unit.jar bc0c0b11b5b3dfc5bb626d85865d653aee035a97089f18db150292e5350db547
instrumented.jar 667190bfd128b9c889ebb333f4cfeca664fb1cc387f1ef6abe3b963826fedba5
WorkerHistoryNavigation.kt 3a508a45f3e86128fbbd2eb71e2b6a6af2dce54cdc1b3553857f25b74d0ded7b
PairingViewModel.kt 92553ec888df9411cf3e780f6830d8bb5c4f8e54e4cc2fdbb2d00f8c1f3ab599
WorkerHistoryCodec.kt c91090fab25946069c53e1a9b6999bf5a8bba660c7fd61c7e6c0111a1306fb58
TranscriptPageCodec.kt f9bed591bd8d28cd2e5108a7a0da725b043e00e7ea5a40262bf9b6888d44d260
MainActivity.kt 8536afd25a36b3db263d899911aaf34f539a60306614406d39fceaf56bdc27a5
```

## Remaining Scope

Navigation checks do not prove process-death/camera/device/LAN behavior or visual
layout on supported device viewports. Failed/cancelled partial history, worker
event/cursor replay, local TUI worker controls, settings and interactive PTY
remain unfinished. Actual API31/34/35 camera QR/LAN desktop synchronization,
disconnect/replay/process lifecycle and signed reproducible release remain
unverified. Goal active; no Blueprint item accepted.
