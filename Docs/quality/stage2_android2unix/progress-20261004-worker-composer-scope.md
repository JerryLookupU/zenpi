# Stage 2 Worker Composer Shares Complete Active Scope

Recorded 2026-10-04. Partial S2-035/S2-036/S2-054 evidence. Previous goal turn
made progress (secondary scope/concurrency guards); this turn changes production
worker composer admission. Canonical cwd /Users/mac/Github/zenpi, dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. All 36 Blueprint rows remain [ ].
Existing changes preserved; no nested agents, reset, push or permission bypass.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Behavior

workerPrompt construction, PromptControlPolicy.workerSend and Compose task
composer now use SubtabProjectionCodec.current. Previously they independently
checked the active ID, which could ignore duplicate inactive tabs although
fresh dispatch already rejected the complete malformed scope. This is a
construction/UI consistency repair, not a demonstrated host authorization
bypass. Missing active IDs, foreign tab projects, wrong selected session and
duplicate inactive tabs refuse fresh task construction and hide its composer.

JVM regression proves valid tab switches route new task input to the new tab,
old captured submit refuses after the switch, and explicit running-task cancel
can still address another tab. Saved exact retries retain their identity under
existing lifecycle fences. Expanded existing WorkerScreenTest checks malformed
scope, valid composer restoration and other-tab cancellation; it compiled but
has NOT executed on a device. No instrumentation method/count was added.

The real headless owner lane now obtains production SubtabList before initial
prompt. Current Kotlin decodes that snapshot and validates worker composer
construction, duplicate-inactive refusal and exact pending retry. The generated
worker envelope is deliberately not dispatched: these assertions do not claim
a second real worker task. Existing actual prompt/steer/guarded tools/cancel and
fresh recovery continue running unchanged. A preliminary conditional assertion
in the metadata-only worktree lane was removed before final attribution and
replaced by unconditional checks against actual worker-enabled headless state.

## Verification

Source checker controls isolated OUT and ZENPI_HOME fixture paths; cached
compiler/SDK compilation does not create an APK. No live external provider,
user credentials or terminal configuration used.

- Intermediate zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-worker-composer-scope-20261004: exit 0; additional current
  headless snapshot assertions landed while it ran. Not final attribution.
- Final zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-worker-composer-scope-final-20261004: exit 0; 533 ordinary
  JVM tests, main/unit/instrumented compilation, production QR corpus, explicit
  interop lanes, six-test desktop target, real worker-tool loop and seven-phase
  prompt/approval/cancel lane. No source/test edits after final run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-worker-composer-scope-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass;
  validator fixtures, not connected API matrix.
- Initial cargo fmt --check failed new assertion wrapping; corrected via
  apply_patch, repeated check exit 0. git diff --check exit 0.
- Strict Blueprint validation exit 0: 36 unfinished, zero self-tested/accepted.
- Host verify-only correctly failed after Cargo test changed the attributed
  binary. python3 tools/stage2_host_build.py rebuilt after final Cargo lanes;
  --verify-only exit 0, original input/binary hashes restored.
- python3 tools/stage2_android_build.py --verify-only: exit 1 missing APK
  provenance; unchanged Gradle/ADB permission failures not retried/bypassed.

Final log: /private/tmp/stage2-worker-composer-scope-final-20261004.log.
Owner exchanges: /private/tmp/stage2-worker-composer-scope-final-20261004/android-prompt-results.json.

```text
HostClient.kt 198e2ca8c1fe3015c9462a6304def961552e7fa3de622437325c67d22fabcb46
PromptControlPolicy.kt bdfef3b3f6ccf4dc700bbf67c9cfb39968582e913ad55bf90074fb5f065b39a0
MainActivity.kt 8ea1421401d9f69258eb5c79077a54160b2a39943e16d77129b4626d7bff2d06
PromptControlPolicyTest.kt d80a4fe48ae3c5b27ffc421e4455be035faf05081508de02eb146551769efdfb
WorkerScreenTest.kt b27d5db710d0e0a6708caf2bcc40b24fa772eb788b1fac0b19869253de00c786
AndroidRustPromptTest.kt 9e86e6f102d836da85fba751b1af5aec45f5d718cef9abdb072045b423d54d58
remote_headless_commands.rs 743d83dd6dbb0e6f052a0a62b00992426d62391e8dfee0e61e8a6735e8871900
final-log 42133b3fc090aefcd22b0dcfe5713b8803c53d5ce00d8856e982a0e0e709523f
prompt-results ae27f3cec238999fd9b4138434cbb6b8fc4cbc479df2781087c6d29034ae067c
python-log 3bd7339e98609af5080296019a07ea34228a5be70baea13a4f5ecc07148dfcb7
main.jar e6aee6f2fcbba10ca597dc9c245dfcf8d2929a39f2e9272409fb1114fa20cff0
unit.jar 127c201bcd1901a70ad493f16de5d17fbdbb420b4eb4193c37325821342cd3e0
instrumented.jar 8209b561ef4d6014afc36483cc949644727f7135b43e4aac48b543480604984d
debug-host-receipt e66b5e40e038ce9cd82a8e1266b8f927f3fc8defb6467347c65c9a7cf8973906
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35 lifecycle, complete desktop parity and
signed reproducible release remain unverified requirements. None is replaced
by these current-source composer assertions.
