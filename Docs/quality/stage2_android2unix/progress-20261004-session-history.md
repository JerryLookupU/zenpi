# Current Conversation History Beyond Snapshot Tail

Provisional S2-036/S2-054 progress on 2026-10-04. Previous body-search turn
changed source and passed deterministic gates, so it was progress. Current
Blueprint, execution skill, dirty worktree, APK builder, LAN/scanner lifecycle,
snapshot tail and transcript/worker-history paths were inspected. No workers or
nested agents admitted; no authoritative checkbox promotion.

## Build Boundary

A fresh pinned-wrapper APK attempt ran against the body-search source inventory.
It exited 1 before compilation: the global Gradle wrapper zip lock under the
user cache is not writable in this sandbox (Operation not permitted). Diagnostic
attempt and private stdout/stderr files were preserved. No APK provenance was
published, and no historical APK was installed. Subsequent history source edits
make this failed attempt historical diagnostics, not current source attribution.

```text
stage2-device-build-attempt.json 2308fe5e1f8545d2760a64f28c90e74e59c94a05065c16e6b79fafa8ccdeca79
attempt inputs d735b90c4f1969f3efb602c1926b73c455445bb2eefaf81282cc9c0b2260fbc0
stdout e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
stderr 3ba5f553d2a4c0e4bc4ac4787e15e03ff3c7c80b3e773beff3d089099a2b7995
```

## Implementation

The ordinary host snapshot intentionally keeps only the latest 64 messages.
Android previously had no way to browse older current-session messages. Added
Read-authorized transcript_read view=session_history, exact journal-revision
and selected-branch binding, sixteen-turn chronological preview pages and digest
continuation. It reuses existing redaction/tool summaries/control-frame budget.
Count, aggregate content/metadata and deadline bounds fail closed; stale journal,
branch or digest cannot continue. No mutation occurs during these reads.

Android adds a Conversation browser, bounded seven-page back history, forward/
refresh controls, typed scoped validation and full-message/tool-argument reads
through the existing transcript path. A historical turn is transient read scope
only, not a foreground projection replacement or cache/write authority. Journal,
session or authority changes discard history and a selected historical message.
Offline, scan, pending, loading and reset states fence navigation.

Rust regression reaches all 70 durable messages, including those excluded from
the latest-64 snapshot, checks redaction/read-only behavior, malformed args,
stale digest/journal, foreign session and revocation. Existing tree test now
checks history contains only root plus selected branch and refuses the previous
journal revision after branch change. JVM cases cover malformed pages, exact
revision, retained-page bounds, old-message read scope and authority loss.
Prepared narrow Compose/device cases browse history and verify the production
host read without changing projection or pending writes. Session harness now
requires nine exact named tests and unique history UI/host markers. No connected
run is claimed; the prepared provider fixture is small and does not prove a
70-message device browse.

## Verification

- Focused Rust current-conversation regression: 1 pass.
- Final owned-home serialized full remote_workspace: 71 pass:
  env ZENPI_HOME=/private/tmp/stage2-session-history-home-20261004 rtk cargo test
  --offline --locked --test remote_workspace -- --test-threads=1.
- Android source checks compile production/unit/instrumented source, with
  428 JVM tests passing after final test additions. Final current UI-source
  verification: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-session-history-current-20261004; exit 0, 428 JVM pass.
  Artifacts are recorded below. Instrumented code compiles only; existing
  deprecation warnings remain.
- Python Stage 2 discovery: 125 pass; session harness: 8 pass;
  lifecycle validator tests: 21 pass; Blueprint validator tests: 5 pass.
- Strict Blueprint, Rust formatting and diff-check gates pass; all 36 rows [ ].
- First host build correctly refused attribution because source changed during
  execution. A second real offline locked build completed; verify-only confirms
  current Rust source/binary attribution.
- APK verify-only refuses missing provenance. No current APK/device, physical
  QR/LAN, lifecycle, production connected acceptance or release claim.

```text
transcript.rs f50fd8d5b45079d22e46cb6e223d58ca93b41de541ecdcc3b5ab44d8b89f3f8d
SessionHistoryCodec.kt d3a6b912c441325d374a72cda8346c5a597a4452ff1d6933a5d866397c0358e0
SessionHistoryReadPolicy.kt 1213a98e74533659d77e6f7246acffbd6366e9f166142a6481bf5557b03b3d0c
PairingViewModel.kt 6f672093d19c770de67725580a723f87798b10e4898b770482d3d41d028e5c1b
MainActivity.kt 8dab0c78f9db5c83b4d602a7a716ac0e821bc30cd5c9f572faede9e39ec76ae3
remote_workspace.rs 3975bfa2c677668666bd2ffaac9893f45480e4570dd8a020172b9b95b21a1202
SessionHistoryTest.kt 777a756441605da3b102f63f7dceeb11a5e25d3173969c736d61842a5545155d
SessionCatalogScreenTest.kt 979fbc85b25ebe268a291980e8c2df5e98cfb3a4be599bfa401c753b7c67c5e5
SessionHostTest.kt 7a7c10d23674defc6753f7142a5e222d231dcae74ada259434b043682c4d70d6
stage2_session_android_smoke.py 9b7a4d94e9d56ad029a9eb35f6574ae74e151708c6fb895a265fd5fd863238a6
main.jar 436f1e993966e420d0697335ba4a990e161cc21e9083b1e849e3da92c6bb4f9b
unit.jar c80f35893b4c329101eaa8790d5bab2de078bf5b04a8ce08530cba06720521ae
instrumented.jar 441c712f390de8dd6ae08cc77b8e7b9bebc96c2bbaad6e9293dd5963d0cdf15c
debug-host-receipt 591f08044109e4a702516d076950ed2f6bf560a0289e2a170eec55ee98fa09ac
debug-host-inputs 4431f3a783b11d69321a24d084a3b2ae30f84630369b7eaf0ce782d1c4580932
debug-host-binary 3e6773d125de79c0cfb7e5c592f77fef2f6d3ca55f5a4c379ef2b45f5558910b
```

Goal remains active. Current-source APK provenance, physical QR/direct LAN
bidirectional synchronization and recovery, API lifecycle matrix, complete
desktop parity and signed release remain incomplete or unverified. Source
checks alone do not establish these gates or entire-item acceptance.
