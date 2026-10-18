# Android/Desktop File Metadata Interop

Provisional S2-036/S2-054 progress. Previous goal turn completed read-only file
browsing policy and was progress. Current execution skill, specification,
Blueprint, worktree and current Kotlin/Rust file paths inspected. Existing
changes preserved; no nested agents, push, reset or checkbox promotion.

## Added Evidence

AndroidRustFileTest writes current FrameCodec/WorkspaceFilesCodec requests and
reads fragmented actual Rust owner response frames through ResponseReader and
WorkspaceFilesCodec. The new explicit remote_android_model_interop file lane
uses ProjectRuntimeHost/TuiState, the production file listing dispatcher and an
authenticated Read-only software P256 device. No network/provider is invoked.

Fixture has 140 ordinary files plus a nested directory, with private names,
hidden entries and a symlink excluded. Verified first/middle/tail pages of
64/64/13 entries, complete ordered coverage, back-page request, nested file,
foreign session refusal, size-change snapshot conflict and revoked-device
denial. Reads do not append to the actual host journal. Response frames contain
neither fixture file bodies nor host absolute roots. Current Kotlin policy
allows navigation while rejecting attachment insertion; malformed response
scope/snapshot variants are rejected.

First source-check run failed because the new test fixture omitted subtab_list;
FileReadPolicy correctly denied browsing without selected-subtab authority.
Fixed the fixture to obtain the actual host SubtabList response and decode it
with SubtabProjectionCodec, not by weakening policy or inventing authority.
Complete repaired run supersedes that failed attempt.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; date 2026-10-04 Asia/Shanghai.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-files-interop-repaired-20261004: exit 0. Main/unit/device
  Kotlin compiled, 494 ordinary JVM tests pass, current Rust terminal QR images
  decoded, all explicit interop lanes pass. The model/resources/files target
  now executes 3 explicit ignored-by-default integration tests, all passing.
- Python Stage 2 discovery: 135 pass; lifecycle verifier tests: 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass; 36 rows [ ].
- Debug host rebuilt after Cargo validation and verify-only passes.
- APK verify-only still refuses missing provenance; no Gradle/ADB permission
  bypass or stale APK installation. Instrumented UI tests compile only.

```text
remote_android_model_interop.rs 901233fd0174b200239af37a397f069244afda43c983a9a64a7f74955dd312af
AndroidRustFileTest.kt cac9395b33b3522c1d8618d4b20b2e0d8dcbb9ce7b8ed9e7dd25c7df2dcbdd53
stage2_android_source_check.sh 6a682f90e5dfc2a576e3202853667208fec3d18843a265e5926977ddf121e61d
source-log 02a5a45af204fbb886cc3e8a54488446f376669b1e8c175e10bb84414a1bb653
file-results 27047eeaef7c2ba70f336f1cd2d314a8bb51e5d6dc14d4653d2a9db77feedb46
main.jar ffa3b7dde0f1065f565f7cf2b0819464d3db86ee82b3bd37b4193f18ba3f7e29
unit.jar 9eecfe4258072e4695aa474ab85de056704b018b5fd85f2cb7268df8c20cf8dc
instrumented.jar b26bf6b73ea948a77668310bce33a6f3ab3ced9d6a754c1f75b8e3d0cb730a63
debug-host-receipt fa660a086f77f1a6a91f91b8eebf5952e2795276fab13b6f257bf8922b2f66de
debug-host-inputs 404f2e9839daf95bc19114225349e4ccfa78c94b539941804f15e682bf8b5a83
debug-host-binary 51cf433e9ebfd59aeebdbf46f844c0603872e798f8e4b6443308aed8627eca67
```

Goal active, not complete. This is JVM framing/codec and actual desktop owner
evidence, not production TLS/listener, Android Keystore, current APK or physical
QR/direct-LAN bidirectional synchronization. Connected API lifecycle, complete
desktop parity and signed reproducible release also remain required gates.
