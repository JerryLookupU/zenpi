# Android Resource Catalog Interoperability

Provisional S2-019/S2-036/S2-054 progress. Previous goal turn changed model
admission and verified current interop, so it was progress. Current execution
skill, Blueprint/specification, dirty integration, catalog/reload/receipt code,
ViewModel publication, desktop mailbox and resource UI/tests inspected. No
nested agents, commit, push, reset or checklist promotion.

## Change

Added AndroidRustResourceTest and an explicit resource test beside the existing
model desktop fixture. The source checker now runs both tests, rather than
treating ordinary ignored-test output as interoperability. Current Kotlin reads
the real desktop listing/snapshot/catalog and frames its continuation or reload
command. Rust decodes and dispatches through ProjectRuntimeHost/TuiState and the
actual resource loader. Kotlin checks fragmented responses, exact scope,
increased generation and altered-receipt refusal.

The fixture loads 40 actual project prompt templates. It reads a 32-entry first
page and 8-entry tail, validates digest-bound continuation and bounded previous
page retention, edits template metadata, reloads once, refuses Read-only/busy/
stale/altered same-ID requests, and tests a local reload winner. After owner
recreation, exact retry returns the original receipt without reloading or
appending to the post-bootstrap journal. Original journal bytes remain a prefix.
Wire assertions exclude private template bodies and the canonical host path.
This is metadata and local owner evidence, not provider, extension startup,
Android Keystore/TLS, camera or physical-LAN acceptance.

Production resource reload now delegates to the same ProjectRuntimeHost branch
used by the interop lane. Pairing/active-job/queued-input/worker fences remain
computed in the production loop; the owner entry also rejects modal state for
new reloads and refreshes the visible resource catalog. Exact durable retry still
resolves before new-operation busy checks.

Android new reload admission now requires a current scoped catalog, no loading/
error, matching generation and snapshot digest. The added policy test refuses
stale/foreign/missing/failed states and accepts a refreshed command. Exact
pending-envelope retry remains separate under existing authority fences.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-03T23:00:01Z.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-resource-interop-current-20261004: exit 0; 486 ordinary JVM
  tests, production/unit/instrumented compilation, current QR and all explicit
  Rust/Kotlin lanes pass. Combined model/resource target: 2 pass. Instrumented
  tests compiled only. An earlier run compiled production before the admission
  edit and unit tests afterward, failed the new policy test and was superseded
  by this complete unchanged-source run.
- ZENPI_HOME=/private/tmp/stage2-resource-owner-home-20261004 rtk cargo test
  --locked --offline --test remote_workspace --test remote_parity
  --test tui_project_workspace: exit 0, 105 pass across three suites.
- Python Stage 2 discovery: 135 pass; lifecycle verifier tests: 25 pass.
- Rust fmt, strict Blueprint and git diff --check pass; all 36 rows remain [ ].
- Debug host rebuilt after all Cargo lanes; source-bound attribution refreshed.
- APK verify-only refuses missing provenance. No unchanged Gradle/ADB retry,
  permission bypass, stale APK installation or physical QR/LAN acceptance.

```text
src/tui.rs 728bd0b044c409d1036010e660a8e1e2bb217d1f78720dca9727dde56c52310d
remote_android_model_interop.rs df076a4198acc29b7918e0817e1c89129128237ba3c6282a26d487b635bd3e89
WorkspaceDispatchPolicy.kt 81e06b69a6d60a0edafa61b6a6f4db782beeda79028497077b12c70860db85a9
AndroidRustResourceTest.kt 1160d38a5bef6763de8416aa97257ba9d106e5350700d20a4d44c6f77d718189
ResourceReloadTest.kt 60bf84c59eb02497d3fffd1a88f9c093e4ca402633c5d87ec6968e70d5f7737f
stage2_android_source_check.sh 0f141a00850dfaf6b5d377d24db7c92d2e67c7e46013bf79af1dad68b3b8c531
source-log ea0e540f0e094bcb9352da15e0038ab1eda821e368e5c969901a5631d7c884f9
resource-results bafbf9e73bd7c74a99b2b0071a07519db157537546bb86abd638dc751934829f
main.jar 980aa3b50d76c761e2e5f1e188140d5ee601cdc5c8502c677ff968e3918a52fd
unit.jar 9e5e2dcb6d3f77ae39c212ef2ad8be75f9caf2bb063e70d7bd7fbdbb5abb90bd
instrumented.jar 41cc1bd5bc324f7772711c0fe2b5d87c601afab40b618d85d27f3d756c842205
debug-host-receipt 991f5f758b143380b1a3829fa6167301f1b787294dc39cf4e99ecb6210bb780f
debug-host-binary 51cf433e9ebfd59aeebdbf46f844c0603872e798f8e4b6443308aed8627eca67
```

Goal remains active and incomplete. Current source-attributed APK, real Android
camera QR/direct LAN bidirectional sync and recovery, API lifecycle, complete
desktop parity and signed reproducible release remain required unaccepted gates.
