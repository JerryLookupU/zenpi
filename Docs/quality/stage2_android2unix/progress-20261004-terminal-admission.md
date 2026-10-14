# Android Terminal History Admission

Provisional S2-036/S2-038/S2-054 progress. Previous goal turn added executed
diff interop and was progress. Current execution skill, Blueprint/specification,
worktree and terminal codec/UI/ViewModel/interop paths inspected. User changes
preserved; no nested agents, reset, push or checkbox promotion.

## Defect and Fix

Terminal history entry/dialog and ViewModel reads did not consistently reject
scanner state. Codec bound project/session/tab revision and terminal instance,
but did not require the active tab to exist uniquely in the advertised tab list.
Previous navigation also did not require an open dialog, unlike forward reads.

Added TerminalReadPolicy shared by UI and ViewModel. It fences scanner/reset,
offline/loading and pending write/revoke, requires current unique project/
session/selected-tab/terminal scope, and permits Read-only and exited terminal
history without borrowing input/resize capability. Navigation additionally
requires open, error-free matching metadata and a valid bounded history chain.
Stale/broken/failed pages can refresh against current authority, not navigate.
TerminalPageCodec now rejects missing/duplicate active tabs before requests or
selection matching. Existing generation/network/cancellation/publication fences
remain. No new PTY is created or input admitted by a read.

Three JVM tests cover read-only/exited/failure refresh, lifecycle/unique scope,
pending write, malformed/stale/closed page and duplicate-tab request refusal.
Existing TerminalHistoryScreenTest was extended without new named device tests.
Current AndroidRustTerminalTest checks policy against actual host PTY pages
using a Read-only projection and rejects input construction. That projection
check is not a new actual Read-only device transport/ACL receipt.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-03T23:49:56Z.

- First source run failed instrumented test compilation due to cross-module
  Kotlin smart-cast; fixed with a local let binding, no production policy
  relaxation. Repaired complete source run supersedes it.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-terminal-admission-repaired-20261004: exit 0; 500 ordinary
  JVM tests, current QR decoding, main/unit/instrumented compilation and all
  explicit current Kotlin/Rust lanes pass, including real PTY command/history
  lane with new policy assertions. Model/resources/files/diff target 4 pass.
- Python Stage 2 discovery 135 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass; 36 rows [ ].
- Debug host rebuilt after Cargo lanes and verify-only passes.
- APK verify-only refuses missing provenance. Instrumented UI tests compile
  only; no permission bypass, stale APK install or physical QR/LAN claim.

```text
TerminalReadPolicy.kt c09e0a4cf2be4a1c8317b3eed6c0863054930dec45152bdbdce90c750dea5b8e
TerminalPageCodec.kt fc8bf531b6f1a9e4c78d83670ff7ead3f2c81fa025e9bf0ab517018d3922c223
MainActivity.kt d13ecca975648896da12ccafe82ddb8a569c3a3c025b609fe6e4b6e4f20ad365
PairingViewModel.kt aeb0dd63ec57db5517114c29a8d1a9c8e10d040febec98690739c1334a6d9156
TerminalReadPolicyTest.kt 77ec93c95f95db6da2d83798e960c60e5461c223fd9d61957d8d7c67812c2416
TerminalHistoryScreenTest.kt de55c24b8c3d7e75f728aa09887301ac59d21c0d86031d14e8bc8eb37995282f
AndroidRustTerminalTest.kt 22bce7783bf046aac2e0fa2ab83d0e873d59ff3e2f85040dabdc7713a6930d91
source-log d7db55b287ac5e6184985124b05a1b547ef4a32d09cd792d56fa05688566b802
main.jar 33c124ef9c6a414a245da931de98bcd80595804a150851704a0943399410efd7
unit.jar d4e28b6b5beee1531ef64a60d5078544ce81c1b1ab1a6de2a21ecc1f85ee0319
instrumented.jar 0f6b390e07104440701bd0d3a0edf5f1569e7734f954aff4be2d23b0ffb57ae5
debug-host-receipt 8835524c4ff52968ef4446d586ebc5e9a614a1a024322d7425d2db25a4a46cf9
debug-host-binary 51cf433e9ebfd59aeebdbf46f844c0603872e798f8e4b6443308aed8627eca67
```

Goal active and incomplete. Current APK, actual Read-only device terminal LAN,
physical QR/direct-LAN bidirectional sync/recovery, connected API lifecycle,
complete desktop parity and signed reproducible release remain required gates.
