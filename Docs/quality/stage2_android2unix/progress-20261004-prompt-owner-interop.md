# Stage 2 Kotlin Prompt And Real Owner Interop

Recorded 2026-10-04 UTC. Related requirements S2-014 owned headless adapter,
S2-019 routing, S2-036 client control and S2-054 parity. Partial evidence only;
all 36 authoritative Blueprint rows remain [ ], no whole-item acceptance.

## Authority And Scope

Cwd /Users/mac/Github/zenpi; integrated dirty-worktree HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push, permission bypass or stale APK installation.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
specification SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

Added AndroidRustPromptTest and an explicitly invoked ignored test in
remote_headless_commands. The source checker excludes this Kotlin class from
ordinary tests and invokes it through Rust only with freshly compiled classes.
Kotlin consumes real host listing/snapshot and emits production prompt frames;
HeadlessHost dispatches to the existing Agent and cooperative BlockingBackend.
Kotlin consumes its actual running stream preview and emits cancellation.
Rust refuses Read-only identity, another Control device and foreign session,
cancels the original owner, then verifies stopped status/no transient stream.
Exact prompt/cancel receipts match at admission, after stop and host restart.
Backend invocation count stays one; restart receipt lookup does not rewrite
the session journal. Kotlin revalidates fragmented responses through actual
ResponseReader/WorkspaceReceiptCodec and cannot construct a new cancellation
from the final cancelled status. Exact persisted retry remains admissible.

This is a controlled backend and in-process host adapter, not a real HTTP
provider, tool effect, TCP/TLS client, camera, LAN or Android Keystore proof.
The new lane covers prompt and cancellation, not steering execution; steering
remains covered by existing host queue tests and still needs cross-language
runtime/connected evidence. Full objective is unchanged.

## Failures And Correction

First complete checker reached the new lane but failed when the test presumed
an immediate retry under running owner contention must return Accepted. Actual
host returned UnknownOutcome/null. Revised the fixture to retain that
uncertainty and compare receipts only once the owner is available after stop.
Second checker failed when Kotlin expected ResponseReader to return a page for
that null result; production correctly refused Host response result missing.
The test now asserts both receipt validation and reader rejection. Neither
production implementation nor acceptance policy was relaxed. Both failed logs
remain under /private/tmp, not counted as passes.

## Verification

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-prompt-owner-interop-20261004:
  exit 101 at new retry assertion, ordinary/previous lanes passed.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-prompt-owner-final-20261004:
  exit 101 at null-result validation assertion, ordinary/previous lanes passed.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-prompt-owner-verified-20261004:
  exit 0, 529 ordinary JVM tests, production QR corpus, all source compilations,
  every existing explicit lane and the new one-test Rust/three-phase Kotlin
  prompt/cancel lane passed. Main and instrumented sources unchanged this turn.
  No source/test edits after verified run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint verifier: exit 0.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  current debug host attribution is not reproducible release evidence.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance; unchanged Gradle/ADB permission failures were not retried.

Commands ran from cwd above. Source checker controlled ZENPI_ANDROID_* fixture
paths and a new isolated ZENPI_HOME=$OUT/prompt-home. No user config/credentials
modified. Evidence JSON at
/private/tmp/stage2-prompt-owner-verified-20261004/android-prompt-results.json.

```text
remote_headless_commands.rs a2f612a3fbdcc159a87ef058542bf80e3156b8c264e5bcf4dd86b1130ee6d340
AndroidRustPromptTest.kt f21d5d4da145e7ab2c39d5a8e0e90d95570e01530f46e13a27650af3485a10e2
source-checker 45610008713f68d16aa9df5429b8bb43c9f6432e805296b834a6caaec86d9dd5
verified-log 356d4fb070ab54e7818335fee55f5e4d8b7de851421815502d386997ee5e60e5
prompt-results a029b1cb32e865aa092416f81d3288d746dd636f897769c09ec39aeda8236f3e
python-log 3d3a0a5fd20bba70c6886ad1a4dc7135de07fe0d9eb4f3580db565e64bec1f00
main.jar abea70cf5f840925ca447dc6d31d78c25380f670153cef4c0d4a2b7a6952fdeb
unit.jar d7787cf8dff0661acab09a8ce5fb9a0866af022517a205c1e274d49c15011b84
instrumented.jar f09ebc6d879a8b2d75da1b21208ee8900766935b362c7e0bf12d47077cbb47c8
debug-host-receipt 69a05f3f8011554d02c1606b11119d5f6c43b156c0539265e775b6792c4574fd
```

Goal active and incomplete. Current APK, physical camera QR/direct-LAN
bidirectional synchronization/recovery, connected API31/34/35 lifecycle,
complete desktop parity and signed reproducible release remain required.
