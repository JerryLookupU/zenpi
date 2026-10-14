# Real Worker Tool Loop Android Interoperability

Provisional S2-036/S2-054 progress. Previous goal turn added real desktop
worker read interop and was progress. Execution skill, Blueprint/specification,
dirty worktree, WorktreeHost/Agent/tool paths and Android codecs inspected.
Existing changes preserved; no nested agents, reset, push or checkbox promotion.

## Actual Tool Chain

Added an explicitly run Rust unit-test interop lane and AndroidRustWorkerToolTest.
The test installs a deterministic local backend on one actual worker Agent,
then dispatches a real prompt through WorktreeHost, observed runner, worker
execution policy and production read_file tool. Nine distinct fixture file
reads succeed under Read-only policy, producing twenty durable turns: user,
nine assistant/tool pairs and final assistant. No fabricated journal history
or tool-result messages replace execution.

Kotlin generates framed history and selected tool-call argument requests.
Rust returns actual task-bound transcript results; Kotlin ResponseReader
handles three-byte fragmentation and production codecs validate them. History
pages span 16+4 entries, preserve unique IDs/calls and support previous-page
navigation. All nine tool messages contain actual fixture file content.
Secret assignments are redacted before wire output. A valid file path longer
than the 512-byte preview verifies argumentsTruncated versus the exact full
argument JSON read. Foreign device, wrong call ID and stale history digest
are refused and decoded as failures. Scanner state disables history/detail
navigation. Master/worker journal bytes remain unchanged across these reads;
master conversation is empty after worker completion.

Development failures are retained and not counted as success: missing isolated
ZENPI_HOME attempted forbidden configuration I/O; added fixture-local writable
home, consistent with other lanes. A 1400-byte alias path exceeded macOS path
limits; shortened to 600 bytes, still beyond preview length. Repeating identical
reads triggered the production doom-loop guard; changed the fixture to nine
distinct files instead of disabling or relaxing that protection.

## Verification

Canonical cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b.

- Final complete source checker /private/tmp/stage2-worker-tool-verified-20261004
  exit 0: 515 ordinary JVM tests, 54 production QR cases, main/unit/device-test
  source compilation, every existing explicit Kotlin/Rust lane (six-test
  desktop target) and the new real-tool unit-test lane (1 passed, 0 ignored).
  New Android test is excluded from ordinary discovery and explicitly run
  from current seed/command/reply data, not silently skipped.
- Python Stage 2 discovery 137 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass; 36 rows [ ].
- Debug host rebuilt and attributed after Cargo lanes. APK verify-only refuses
  missing provenance. No unchanged Gradle/ADB retry, permission bypass or device
  acceptance claim.

```text
worktree_host.rs c4921f6c4d3c57a6952f96fc0308d909a0253b2802c1cc753e2111c1fa9c08b6
AndroidRustWorkerToolTest.kt 162cb077ed524280a50fe5052802735d4be0b0226de58c315196122378f3e4f4
source-checker 7c221eda32f1ddf353c53f060274092be8a495a733a44a98e3c1a803c4fda95f
final-source-log 26e25c6cffd65041faa7a0c36d292f14e969d09154c33b94b70dc8dc63663d32
tool-wire-results 0d8454f948badca1fa8eec641a24a49992e656f061f7bf1d48181cb24aa4922f
main.jar d0ba8f4168d9217d9df7775d2a368ac453d3e30c5614562b4c7af4732934110a
unit.jar c69649b946cd90f8815b79dacb64160452485f567567bd58d1fa8df475866b96
instrumented.jar a5deb9ca7f74c5438968a4ec710572adee8a563d7ce2f37450a52b50af7abab7
debug-host-receipt 83fafdf22ee04e9adfce8d763d50899e1d667cfddbaf92e155c5cbfa439704af
```

Goal active and incomplete. This proves local production worker/tool/JVM
interop, not the desktop UI tool-loop, real provider service, physical camera,
TLS/LAN or device lifecycle. Full arguments fit one page in this fixture.
Current APK, physical QR/direct-LAN bidirectional sync/recovery, connected
API31/34/35 lifecycle, complete desktop parity and signed reproducible release
remain required gates.
