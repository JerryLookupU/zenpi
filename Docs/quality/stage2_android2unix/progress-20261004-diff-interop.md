# Android/Desktop Git Diff Interop

Provisional S2-036/S2-054 progress. Previous goal turn implemented diff read
admission and was progress. Current skill, Blueprint/specification, worktree,
host diff tests/dispatcher and current Kotlin codec inspected. Existing user
changes preserved; no nested agents, reset, push or checkbox promotion.

## Added Evidence

AndroidRustDiffTest generates current FrameCodec/DiffProjectionCodec paged
requests from actual host workspace/inspect/subtab responses. A new explicit
remote_android_model_interop lane dispatches them through ProjectRuntimeHost
against a temporary real Git repository and Read-only software P256 fixture
device. Kotlin ResponseReader consumes fragmented response frames, decodes
patch pages and validates DiffReadPolicy navigation without prompt authority.

Four pages at offsets 0/8192/16384/24576 carry a complete UTF-8 patch. Rust
reassembles the actual patch and checks its length against total_bytes and its
terminal line. Kotlin verifies bounded valid UTF-8 pages, page continuity,
retained previous-page navigation, scope and digest mismatch rejection.
Changing the file after the first page refuses the old digest with
revision_conflict; restoring content permits continuation. Each page rejects
a foreign session; revocation after the final page refuses the same read.
Owner journal remains unchanged and frames omit absolute roots/provider keys.

The source checker excludes this test from ordinary JVM discovery, then
explicitly executes it with freshly compiled Kotlin classes and actual host
responses. It is never counted as passed merely because its default fixture
assumption skipped execution. No production socket/provider request is made.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-03T23:40:28Z.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-diff-interop-20261004: exit 0. Main/unit/instrumented
  Kotlin compiled; 497 ordinary JVM tests pass; current terminal QR decoding
  and all explicit current Kotlin/Rust lanes pass. The model/resources/files/
  diff integration target now executes 4 tests explicitly, all passing.
- Python Stage 2 discovery 135 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass; 36 rows [ ].
- Debug host rebuilt after Cargo lanes and verify-only passes.
- APK verify-only refuses missing build provenance. No stale APK install,
  permission bypass, device UI run or physical QR/direct-LAN acceptance claim.

```text
remote_android_model_interop.rs 5e7fd1fad039c68625e4756e6264eaa4b600b5249c71e4a4fdd55260521d10d5
AndroidRustDiffTest.kt 3daa3c21acfe4d04b0e5c845d95f02f4a5809224c0752ec92ba45f6127ce3dcc
stage2_android_source_check.sh 8b9092f5ddb651a61c33996c90d807d28bc4745e2f78d507d2e05a6ad5347ce4
source-log 282d5859df3d6c0f92459d8dc72e1bfc280bd43aef50ceaefdd638266ecb5900
diff-results 582088abd20c6f0701965e793fb3bb4095cfb2d152475790b5361426289b093d
main.jar a790c484137137cce03a8c9157c6f50f77cdaa46f52913d81cd8b03fedea6265
unit.jar 60e113f106f210feafeffffa4019f9c96034491aa4fdb8e520cec0a6a6e606ba
instrumented.jar 7e7bd4b278e953570fc13cc5dde3764b63727e1f206b839b5a148c0d39ee18f4
debug-host-receipt 79339419e2be06178c055908dcb5aee6435e171e39ed4d708893b613cee28374
debug-host-binary 51cf433e9ebfd59aeebdbf46f844c0603872e798f8e4b6443308aed8627eca67
```

Goal active and incomplete. JVM framing/codec and desktop owner/Git evidence
does not prove listener TLS, Android Keystore, current APK, physical QR/direct
LAN synchronization or recovery. Connected API lifecycle, complete desktop
parity and signed reproducible release remain required unaccepted gates.
