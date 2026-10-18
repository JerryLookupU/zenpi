# APK Build Diagnostics

Provisional S2-030/S2-071 progress on 2026-10-04. The preceding RTK request
verified the installed CLI and Rust formatting but did not change the Android
workflow. Current blueprint, execution specification, dirty worktree, builder
and packaging evidence were inspected. No worker or nested agent was admitted;
all 36 authoritative rows remain unfinished.

## Change

The APK builder previously captured Gradle output but discarded it on failure,
leaving only a short permission/error marker. This obscured the build blocker
between the Android source tests and actual camera/LAN device acceptance.

Each completed process now retains raw stdout/stderr in unique owner-only
local files under android/app/build. A diagnostic attempt index records their
hashes and byte counts, exact argv/cwd, times, source-inventory digest, exit code
and timeout flag. Timeout output is explicitly partial and its exit code is
unknown. Existing log files are not overwritten on another attempt. Logs are
local diagnostic data, not published quality evidence; documentation requires
review/redaction before sharing.

This index always has passed=false, including exit-zero execution. It cannot
authorize APK installation. The existing separate source/APK success record
still requires successful Gradle tasks, unchanged inputs and both real APK
hashes. Failure/timeout continues to remove previous authority. Tests verify
private file modes, exact log bytes/hashes, timeout semantics, retained earlier
logs and fail-closed provenance.

## Verification

- python3 tools/test_stage2_android_build.py: 13 tests pass.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': 118 tests pass.
- Lifecycle verifier tests: 21 pass; blueprint verifier tests: 5 pass.
- Strict blueprint and git diff --check pass. All 36 rows remain [ ].
- Revalidated Android current production/unit/instrumented sources using
  zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-terminal-continuation-revalidated-20261004:
  412 JVM tests pass. Instrumented tests compile only; no device execution.
- Editing the shared builder correctly invalidated Rust build attribution.
  python3 tools/stage2_host_build.py completed the real offline locked Cargo
  build and refreshed debug attribution. The Rust binary hash is unchanged.
- APK verify-only still refuses missing provenance. No unchanged blocked
  Gradle/ADB/socket retry, old APK install, physical camera scan or LAN success
  is claimed. The new diagnostic path is unit-tested, not a fresh Gradle run.
- Invoking the lifecycle acceptance CLI without its required desktop/headless
  API-matrix receipts correctly failed argument validation; it was not an
  acceptance run. Those connected matrix receipts remain missing.

```text
stage2_android_build.py a4d28e45c06f7544c77f8a91a10d3929ff23ba3d87c3415251307927fd76ffee
test_stage2_android_build.py 50b7e4b6868bb48ac3bbbbc85c1b52133d6672f3a210a9d20b1549814870853c
debug-host-receipt 06c70100d9496d38e8128dfe1ed06e29ac935cba3b2c2b1f570f9159f50dcfce
debug-host-inputs c55bacffb031a8e324b93971274ebb56615d4f2978c135ab74ec45451d2dd49f
debug-host-binary 57b52b80beda551642c47cd1f404f2fd1400b7a16699f91401402cb77db227a8
```

Goal remains active. Source-bound APK, physical QR/direct LAN bidirectional
sync and recovery, connected API lifecycle matrix, complete parity and signed
release remain incomplete or unverified. No full item acceptance is claimed.
