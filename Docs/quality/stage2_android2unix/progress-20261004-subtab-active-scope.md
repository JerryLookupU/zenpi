# Stage 2 Secondary Tab Active Scope And Lifecycle

Recorded 2026-10-04. Partial S2-035/S2-038/S2-054 evidence. Previous goal turn
made progress (workspace identity guards); this turn changes production Kotlin
scope admission and rendering. All 36 Blueprint rows remain [ ]. Canonical cwd
/Users/mac/Github/zenpi; dirty-worktree HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. User changes preserved; no nested
agents, reset, push, permission bypass or stale APK install.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Change And Coverage

Secondary editing rendered by indexing the active tab and the concurrency
stepper used single on its ID. A missing or duplicate active ID could therefore
throw if an inconsistent in-memory projection reached the screen. Production
wire decoding already rejects malformed tab arrays; no demonstrated wire
decoder bypass is claimed. The stepper also lacked the scanner/reset fences
already used by ordinary workspace mutation buttons.

SubtabProjectionCodec.current checks selected project/session identity, scoped
tab project, 1..32 capacity, unique tab IDs and exactly one existing active ID.
Secondary command construction, fresh dispatch and Compose use it. Invalid
scope keeps tabs read-only and hides indexed editing/concurrency controls.
The concurrency stepper now uses workspace mutation lifecycle guards. Valid
main/directory/worktree commands, 1..64 quotas and moving to the end remain
covered by the explicit Rust/Git owner lane. Saved exact retry still takes the
dedicated pending path after lifecycle guards; invalid current display does not
generate a replacement command or discard its original identity.

JVM regression checks foreign session/project, duplicate project, empty/missing
active tabs and duplicate tabs across construction/dispatch/retry. Current
Kotlin with actual Rust/Git snapshots repeats malformed-scope refusal before
each generated command. A Compose regression checks no-crash rendering,
disabled scanner/reset stepper and valid restoration. Instrumented compilation
passed; that regression has NOT executed on a device. Workspace instrumentation
count increases 52 -> 53; full desktop+workspace+Doze now requires 89 named tests.
Historical device receipts cannot satisfy this current suite.

## Verification

Commands from canonical cwd, source checker controls isolated OUT and
ZENPI_HOME fixture directories. No provider calls/config or user secrets used.

- Intermediate zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-subtab-scope-20261004: exit 0. Actual Rust/Git negative
  assertions were edited during this run; final evidence is the repeat below.
- Final zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-subtab-scope-final-20261004: exit 0; 532 ordinary JVM tests,
  main/unit/instrumented compilation, production QR corpus, all explicit
  interop lanes, six-test desktop target, real worker-tool loop and seven-phase
  prompt/approval/cancel owner lane. No source/test edits after final run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-subtab-scope-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass;
  validator fixtures, not connected API31/34/35 execution.
- cargo fmt --check, git diff --check and strict Blueprint validation: exit 0;
  36 unfinished rows, zero self-tested/accepted rows.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes.
- python3 tools/stage2_android_build.py --verify-only: exit 1 missing current
  APK provenance. Unchanged Gradle/ADB permission blockers not retried/bypassed.

Final log: /private/tmp/stage2-subtab-scope-final-20261004.log.
Rust/Git exchanges: /private/tmp/stage2-subtab-scope-final-20261004/android-worktree-results.json.

```text
SubtabProjection.kt 5a11681db3b741511fedc4f1863e38c40e12495298180b62b2578451bfab9b10
HostClient.kt f2fd9063d645c26ac8acf8786e5565610a4aca8d727ed8e5061e124329cf26f1
WorkspaceDispatchPolicy.kt bec3c4d97f68cef8b7e82b2c11b1d702225739765855966c50e8ff1b98b0a978
MainActivity.kt 360ce6e43f61f5ba701735480e7b5e11fcf0553fbbfe558c7b12a5f55a49d568
SubtabProjectionTest.kt 1e2973eaee60bf16412278ccc4864bd02384c75ed21d19edfebad227d0b9c8ec
AndroidRustWorktreeTest.kt 6df208982f743cecb9ab22b32e4c6978e58466cb10ed89aa270d781820ba60ab
SubtabScreenTest.kt 92b9fa4c45a02f5e53e4b61ec20c631e5e78895a60a97a89e70fec4c577e1998
stage2_desktop_android_smoke.py 0461f744f00e2177548ee4bc931664349e9bf6ddacfb51e49d0b151edc444ca3
final-log 9a7263d783488c906af8961d3c1b5fb26ef7ebb59a5f36e86e024e5928f56f86
worktree-results a904643b9921c2b13f524e8c8c2cc9e5b66113abf729e6186a5c7036a1a4b31a
python-log 5b1768633ce0ca5bed29e8ff0b3d02932151e2d9485bbb9c78c833b480ebc0ef
main.jar 75eb5c26de0ee99ec598c51db3c7021d1e89bbf28c17ff6308f1ae9a7425db26
unit.jar e1461fa889dbacf657ca47517e053cc7996e155fb5da7b0415d82aef0795caba
instrumented.jar 736f32507e89d499709508edc86ac5222115927737dfde67dd451ef054ec90f6
debug-host-receipt 9058a49fc3d3d9b06dafa982551e52962f8271833a15af2f556ac71a424aed4d
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35 lifecycle, full desktop parity and signed
reproducible release still require authoritative evidence. These source/JVM
results do not substitute for those product gates.
