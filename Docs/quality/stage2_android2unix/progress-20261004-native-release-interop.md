# Current Native Release And Android Interop

Recorded 2026-10-04. Partial S2-050/S2-052/S2-054/S2-070/S2-099 evidence,
not whole-item acceptance. Previous goal work was progress: native extension
resume fixture repair and parallel regression evidence. Current worktree,
execution skill, Blueprint/spec, controller and production QR/LAN client
paths inspected before verification. No nested agents, checklist acceptance,
source rollback, old receipt rewrite or repeated denied Gradle/ADB attempt.

## Source And Release

Commands ran from /Users/mac/Github/zenpi at HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b with the existing dirty worktree.
Blueprint SHA-256 f47dc136879fb2489885edbb212f4fbb1c443da2a4ecf1df66dd75b19de922b8;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.
All launched validation handles reached terminal state.

- python3 tools/stage2_host_build.py --profile release: exit 0; current
  locked/offline source attribution. --profile release --verify-only passes.
- python3 tools/stage2_host_reproducibility.py --receipt
  /private/tmp/stage2-native-resume-release-compare-20261004.json: exit 0.
  Two fresh empty targets, temporary directory and repository target, match
  byte-for-byte. Each build exits zero; no normalization or signature stripping.
- Same receipt --verify-only --binary target/release/zenpi: exit 0. Published
  artifact equals both retained artifacts; current source/toolchain/log/path
  identity verified. codesign --verify --strict passes for all three.
- Cargo source-check lanes rebuilt the debug binary and correctly invalidated
  its prior receipt. After all Cargo lanes ended, guarded debug builder and
  --verify-only pass. No stale receipt was manually stamped.

```text
core.rs 2365e727aaca23d7cdc6ce73c767eea4ca976548b8b0386e4002e7bbcbc9adb9
worker.rs 8f7474df0561c42afe7eb08bb12b621d0ac704540ac13664f568d5640c7d2197
host-inputs dfb981e3cd20b6fea88156f5739deb2197d2de132483c6d2637e51f1eb16a621
release-binary ba38f1b03712475e47cf22e9a63cd1977eaec46965fe10746eb1c78cd6453d47
release-receipt 8f32146bb045e5da8f3025a8db3a5f3d56982b309b9dd187e615fc72a56948ff
repeat-inputs 583a853ec219faf13466e69be14de7731d4516c950c8f1e66424c628095dfbf9
repeat-receipt 6ca4ad1f566beb75e78734e94cab5cd9df94b41032ceeaff6ed54edc7e0858b7
debug-binary c93f6d6385f31c092b3abaf34a6e5f959d041ad9a1e671aacdf69add23da63bd
debug-receipt ab97dc2c99f9ebb4146748802ab25e9b60fe2c0e234694325cb5e9d35d00294f
```

## Current Android And Rust Validation

zsh tools/stage2_android_source_check.sh
/private/tmp/stage2-native-resume-final-interop-20261004 exits 0. Main, unit
and instrumentation sources compile; 592 ordinary JVM tests pass. Every
explicit Rust/Kotlin lane finishes, including P256 pairing/challenge proofs,
workspace/worktree changes, session replacement/restart, ACK, approval,
actual owned PTY input/history, model/reasoning, resources/files/diff/history,
worker reads/tool loop and real-owner prompt/cancel/worker cancellation.
This does not run Android SSLSocket, Keystore or camera hardware, nor produce
APK/package/connected-device evidence.

```text
source-checker 6defdb3dac7cdddff21909ad655cbed9284019c628e1cb828b06578caa166d40
main.jar abfe729defd0c3cd22d3d281a9e28c4bb24c2b19cd6856c08ebf8a5f50cc56c0
unit.jar 9997450eb178a900f574f406fbfa41098d34f0d06c6c53bf18ee2eee19b385d8
instrumented.jar b1f668eb5d59377c9047dea40ee4c187f4b67d9d40c99c9bbc796b655bd2d65c
android-sync-wire.json dbac0a365cd9a143fb9bf56acc1343c492ec0f97f4af9cfb4be8ca96b2a66c4d
```

Direct cargo test --locked --offline with --test remote_subscription_tls,
remote_subscription, remote_protocol, remote_pairing, remote_events,
remote_frame and remote_security exits zero: 45/19/18/8/9/10/13 primary
tests respectively, 122 total, one existing loopback-timeout test ignored.
The pairing FIFO isolation test also invokes one child libtest execution.
RTK summarized this as 123 tests/eight suites with seven filtered: that includes
the subprocess and is not 123 distinct primary tests or a newly filtered gate.
Raw Cargo was rerun to confirm the exact seven-target scope.

rtk cargo check --locked --offline exits 0 (20 warnings, no errors).
Python Stage 2 tool tests 163 pass, lifecycle validator tests 25, Blueprint
tests five and execution-controller fixture tests 15. Controller validate/status
shows no live/admitted/authenticated worker, outstanding request or handoff;
validate launches nothing. Strict Blueprint, cargo fmt --check and git diff
--check pass. All 36 authority marks stay unfinished.

APK verify-only still refuses Missing APK build provenance. Physical API31+
device availability query has no answer at recording time. Current real-camera
QR/direct-LAN bidirectional sync/recovery, connected API31/34/35, APK signing/
install/upgrade, supported-host runtime and unsuppressed full Rust/network
regression remain required. No completion or whole-item acceptance claim.
