# Stage 2 Worker Read Metadata Admission

Recorded 2026-10-04T09:34:35Z; partial S2-036/S2-054 evidence, not acceptance.
Canonical dirty worktree preserved. Blueprint remains 36 [ ], zero [_]/[x].
Previous goal turn was progress: a real private-cache Gradle attempt established
daemon local-listener permission denial rather than resource shortage, and
diagnostic classification/tests were repaired. No repeated denied Gradle/ADB
attempt, permission bypass, nested agent, reset or stale APK install this turn.

## Finding And Change

WorkerStatusPageCodec validated retained metadata-only task pages, but worker
history/detail selection from a summary or already captured WorkerReadScope
checked mainly succeeded state and project/session identity. Malformed subtab,
contradictory runtime/failure/output/stream metadata or a corrupted saved scope
could therefore construct a read request. TranscriptPageCodec's previous task
selection filtered by succeeded before uniqueness, allowing mixed-state
duplicate request IDs to evade its initial uniqueness check.

WorkerStatusCodec now has a display predicate matching decoded summary rules:
scope, bounded request ID, hex subtab ID, known state/failure code, runtime
evidence and bounded/NUL-free output/stream combinations. Metadata-only pages
reuse this predicate while retaining their stricter no-preview rule. Summary
decode also checks the final assembled display state. Valid success output,
output truncation and withheld output remain readable; no Control grant is
created by this predicate.

WorkerReadPolicy checks summary and saved scope metadata before selection;
scope capture requires current valid transcript target admission. History/
detail request construction uses the same validation via retainsTurn, which
now rejects duplicate IDs before state filtering. Existing current owner/
revision, project/session, task/subtab and exact continuation checks remain.
No protocol/schema/cache or mutation retry behavior changed.

Two WorkerReadPolicyTest regressions cover malformed summary/scope, mixed-state
duplicates and healthy preview/withheld success. AndroidRustWorkerReadTest
mutates actual Rust task metadata and checks preflight refusal before requests
or scope capture. These mutations are client validation evidence, not extra
host-dispatched tasks, connected device or production TLS/LAN behavior.

## Verification

- Source checker /private/tmp/stage2-worker-read-metadata-20261004: process
  23863 polled through exit 0; 584 ordinary JVM tests, main/unit/instrumented
  compilation and all explicit Rust/Kotlin lanes passed, including six-test
  real model/read/worker target, QR/P256, session/workspace/ACK/restart,
  terminal/worktree, prompt/tool and actual running-worker cancellation.
- Python Stage 2 tools: exit 0, 145 tests.
- Lifecycle validator: exit 0, 25 fixtures, not API31/34/35 execution.
- Blueprint fixtures: exit 0, five tests; strict authority check: exit 0.
- cargo fmt --check and git diff --check: exit 0.
- Host debug build and verify-only after Cargo lanes: exit 0, current artifact.
- Android verify-only: exit 1, missing APK provenance. Last authoritative
  Gradle diagnostic remains denied daemon local TCP listener; no retry made.

```text
WorkerStatusCodec.kt afdc74a7da1306bd36e750aa75dea0a610050a9bbdadfd6b96382e0e88556047
TranscriptPageCodec.kt ef510d3c386d41615352b6722832cdfaa9f430fd6300c623676443987be7b924
WorkerReadPolicy.kt 56f492c9c50f5dd40c3cedd124c937b58c458f29a725716f3011473f23620f0f
WorkerReadPolicyTest.kt e142b0a73ba0733a5c30fa973a79ebf4e95ae5219395695286e40e058b2abdec
AndroidRustWorkerReadTest.kt 84cb22462492d1374bf7ec2c2ed2b080bae6bf88a22426a12fd9f93b8ab055cf
source-log e12a274d56f81b6e40ec48dbe743bceb215f233436ea3788002d576a1c5dc96d
python-log aaa460bfd601010e61f259722c2d762c0c85acaf7563168394e503263fac8499
worker-read-results 1b4ba6d126a8b9e08ac3c4f96fdf6dea63bdcf70dd4e8d111ae5297b4a7a6e85
main.jar a0e657482ee336deecafd90b6d2c10dc41e71c787ed3ceface865cea668082e2
unit.jar 96fc93d05918fb8f9c03729657b775e77269fa3278218cf948418ad92d4aefbb
instrumented.jar f95a635bb1b8d33d66febe3cf15afe0db9afd4ca9d63fb00a5f126e9d1e69630
host-receipt d524f7cad3e92f9cf8d06a515c0638e67c3480dbef31fe1e766bd769206c2943
host-inputs 8ccf956a10a2fcac07b143c4d28aa8d62d1f59af1b5623a338da499e0b09906b
host-binary 40f9f134ed62fd51a71dc6e8d24022f0122dced68faf12f6a52f8b5ea145607f
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35 lifecycle, complete parity and signed
reproducible release are still required. This repair is concrete product
progress; no completion or blocked status is asserted.
