# Android To Rust Mutation Interoperability

Provisional S2-019/S2-032/S2-036/S2-054 progress on 2026-10-04. Previous goal
turn added current Rust-to-Android replay/subscription/gap interoperability and
ran it; it was progress. Current execution skill, dirty worktree, Blueprint,
session control/rename, actual receipt validation, source checker and workspace
tests inspected. No nested agents, workers, checkbox promotion or user reset.

## Changes

Added explicit cross-language session-rename lane to remote_android_interop.
A fixture-local actual Rust owner supplies current listing/snapshot/subtabs;
compiled Android SessionCommandCodec and FrameCodec produce the request bytes;
Rust read_frame and HostAdapter dispatch execute them; Android ResponseReader,
WorkspaceReceiptCodec and SessionCommandCodec validate actual response bytes.
The request changes the host journal/name once, duplicate receipt is exact and
does not change the journal, a later desktop rename survives old-request retry,
new-ID stale journal requests get revision_conflict, same-ID changed payload
gets request_id_conflict, and a current Read-only identity gets unauthorized.
A final actual workspace listing is decoded on Android and must retain the
newer desktop name with session control removed. No provider/socket invocation.

Rust drives two bounded JVM phases (15 seconds each) with task-local public
fixture files. The write lane is ignored in ordinary Cargo runs because it
requires compiled Android classes. Its Android phase helper assumption-skips in
ordinary Gradle and is excluded from ordinary cached JVM counts; the source
checker explicitly runs both phases through Rust and requires their success.
It retains public frame results under its output directory only after complete
validation, without claiming APK/device/LAN authority.

## Verification

Commands ran from /Users/mac/Github/zenpi against current dirty integration.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-android-rust-write-verified-20261004:
  exit 0; Rust read fixture 1 pass/1 ignored; Kotlin production/unit/instrumented
  source compilation; 456 ordinary JVM tests pass; explicit ignored Rust write
  lane 1 pass with Android generate 1 pass and validate 1 pass. Existing warnings.
- Initial in-flight source check included the assumption-skipped helper in its
  displayed 457 count and encountered a shell line-offset error after hashes
  because the running script was edited. That run is not exit-zero evidence.
  Subsequent final and verified runs were unedited, exit 0 and exclude helper.
- rtk cargo test --locked --offline --test remote_workspace --test remote_android_interop:
  default-home run exit 101; workspace 47 pass/25 fail, interop 1 pass/1 ignored.
  Failures include denied configuration writes and downstream admission/state
  assertions; they were not masked or changed to pass.
- env ZENPI_HOME=/private/tmp/stage2-workspace-write-home-20261004 rtk cargo test
  --locked --offline --test remote_workspace: exit 0, all 72 pass. The owned
  writable temporary config home avoids default-home sandbox writes; no user
  configuration is changed. This is not a full Rust-suite pass.
- Python Stage 2 discovery 132 pass; lifecycle verifier 21 pass; Blueprint
  verifier 5 pass; backup-policy verifier 5 pass.
- Rust format, strict Blueprint and git diff --check pass; all 36 rows remain [ ].
- Debug host rebuilt after Cargo's executable changes; verify-only passes.
- APK verify-only: missing provenance, exit 1. No unchanged restricted Gradle,
  ADB/socket retry, stale APK installation, camera or connected LAN acceptance.

```text
remote_android_interop.rs 9c7c9dcc12cd8185b8e723dfbd42de85a3181f8050611c3d95688f666497b7cd
AndroidRustWriteTest.kt b7470a0adc884f146331945f63f713c6e2bc678f2ba8978d404c2f9ce3424cf4
stage2_android_source_check.sh 1318bfb3e056dbf7094547314af6204333c44de630035aa3dff149652a540af6
android-write-results.json 08d31d7b8b4407eb12dd6a3a84a6872bdec0c2d8714f1d09f7b054fb082099da
main.jar f4a19f7e4072678db59c6dabb5267166ddb4d31950252c9fad9600dcfd5130b5
unit.jar 7432871a44df6896e144910f076e8777cfc69eff17dd2e22062fccc05896043f
instrumented.jar 49185cec7ed267be124e9ab6a9dc3bfbfaffcf0fe3dbf1794b10b32f75a49eba
debug-host-receipt 3ce9ed0ea614bc803c75d5d1cd36a0a5f3f64d15936e60d10e716aedbd4ec063
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active. This lane proves current session-rename request/receipt
compatibility and host side-effect/owner fences, not complete operation parity,
Android TLS/Keystore/socket behavior or physical QR/direct-LAN acceptance.
Current APK, actual camera/LAN bidirectional sync and recovery, connected API
lifecycle matrix, complete parity and signed release remain incomplete or
unverified. Meaningful implementation and behavioral evidence progressed.
