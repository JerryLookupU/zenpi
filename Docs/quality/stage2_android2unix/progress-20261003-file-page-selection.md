# File Page Selection Boundaries

Provisional S2-033/S2-036/S2-052/S2-054 progress at 2026-10-03T20:36:36+0800.
Previous goal turn was verified terminal-scope progress. Current file-reader
inspection found arbitrary continuation offsets and UI attachment buttons that
did not explicitly recheck the displayed list's scope. No nested agents,
worker launch, checklist promotion, commit/push or cleanup.

## Changes

WorkspaceFilesCodec limits continuation to adjacent next/previous offsets,
refuses files repeated against the current page, and requires consecutive
snapshot/directory/scope history before retaining up to seven prior pages.
Forward replies repeating paths in that retained window are rejected. Requests
cannot read after the terminal page or start at an unanchored offset. Legacy
initial read-only responses remain supported.

PairingViewModel calls the history validator before publishing files, fences
page navigation on error or mismatched current selection, and closes a stale
successful picker read rather than leaving it loading. File/parent/page buttons
require exact selected project/session/active-tab/revision and no error/reset.
The file-reference operation still only inserts quoted draft text, never sends
a prompt or reads file content; host prompt admission revalidates the path.
No cursor/cache, host authority, private-path policy or write identity changed.

Two new JVM tests cover skips, terminal/unanchored offsets, duplicate paths,
scope mismatches and the bounded history chain. Existing Compose tests now
verify stale list/error attachment fences; their test method count is unchanged.
The prior paging UI fixture now includes the actual preceding page it claimed
to navigate to. This bounded client check does not recompute the whole host
directory digest or detect repeats outside the retained window.

## Verification

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-file-page-selection-final-20261003:
  exit 0; main/unit/instrumented Kotlin compilation, 348 JVM tests pass.
  Existing warnings remain; Compose tests compile only, no device execution.
- ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home cargo test --offline
  --locked --test remote_workspace
  files_pages_reach_every_entry_and_reject_changed_snapshot_without_read_side_effects
  -- --exact --test-threads=1: exit 0, 1 pass. Host-domain fixture, not the
  production listener, Android or physical network.
- Python Stage 2 discovery: exit 0, 86 pass; Blueprint validator unit tests:
  exit 0, 5 pass. Strict validator, cargo fmt --all -- --check and
  git diff --check pass. All 36 authoritative rows remain unfinished.
- Host release --verify-only passes for unchanged Rust source. Android build
  --verify-only refuses missing APK build provenance. No stale APK used or
  unchanged blocked Gradle/socket attempt repeated.

```text
main.jar 22657afe1eaa452aa50e0eff7f008fb39712d12c3b5064145189d656a820b4e0
unit.jar 2c1becaae9f2d249472695c40e7cb14d16e4b5e4df4cba707cd1043ad56e0b2c
instrumented.jar ac939a3ec46cb04298d50833ca8ea3ccfc5c1c7247d8c0df007422dc43daa68a
WorkspaceFilesCodec.kt 4af5d15f04612538bcdee3646b931e2d4d594064d3f837568cd72ae498c4d706
PairingViewModel.kt 4818d4660fd3e8b58af423f8466bb420d9c0a3b85f2eb7d6daf1f43cdf094a96
MainActivity.kt 6760ac812eaa6e18201582f5a11f80b25cd363598acc70fcc240fc659866b90f
WorkspaceFilesTest.kt 67c46e8b1e606c0d8f9cd53537efe289b371c706dd2cba31eb130ec567fa3a2d
WorkspaceFilesScreenTest.kt c6c364b269501d7de125962dc355ec1c574f8ac4ba4dd90316be29a73edc7a8f
```

Goal remains active. Current-source APK, production desktop-to-Android TLS sync,
physical QR/direct LAN, API 31/34/35 connected lifecycle and signed install/
upgrade still need their own current behavioral acceptance.
