# Camera Admission Live-Read Pause

Provisional S2-031/S2-038/S2-052 progress on 2026-10-04. The previous goal turn
added APK build diagnostics and refreshed host attribution, so it was progress.
Current scanner launch/result, ViewModel connection/detail-read ownership,
background and network transitions, existing admission tests and scanner
acceptance harness were inspected. No nested agents or workers were admitted;
all authoritative Blueprint rows remain unfinished.

## Repair

beginScan fenced new refreshes and writes but left an already-owned foreground
subscription/detail read live until Activity.onStop. Between launch admission
and onStop, those reads could still publish a new connected projection or keep
live capabilities displayed while camera work had begun.

Admission now advances the connection generation, cancels the connection,
closes all detail-read jobs and turns a saved host into an offline display-only
projection before returning permission to launch the camera. Existing generation
checks prevent a canceled read from publishing again. Transcript and event
cursor remain available; durable pairing/cache metadata are not modified.
An admitted cancel/error result can still start the existing fresh-authentication
read when foreground/network conditions permit. Invalidated late results retain
the previous explicit-stop behavior. An existing confirmation without a saved
host is retained while rescanning; no token, key, mutation or network request is
created by admission itself.

One JVM regression checks display/cursor preservation and removal of control
authority. One prepared ViewModel/device regression supplies owned connection
and transcript jobs, verifies immediate cancellation/generation fencing before
backgrounding, duplicate admission refusal, unchanged encrypted metadata,
cancel return and foreground camera-failure reconnect. This is controlled
ViewModel evidence, not a real camera/network acceptance run. The scanner
harness requires 23 exact state-suite identities plus
scanner_live_read_pause_verified; old evidence without this marker is rejected.

## Verification

- Final command: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-scan-live-pause-final-20261004; exit 0. Production/unit/
  instrumented sources compile and 413 JVM tests pass. Existing warnings
  remain; instrumented tests are compiled only, not device-executed.
- Stage2 Python discovery: 118 passed; scanner verifier tests: 11 passed;
  lifecycle verifier tests: 21 passed; Blueprint verifier tests: 5 passed.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 rows remain [ ].
- Rust source/binary unchanged; debug host attribution verify-only passes.
- APK verify-only refuses missing current-source provenance. No stale APK
  install or unchanged restricted Gradle/ADB/TCP retry occurred. Physical
  camera, direct LAN bidirectional sync/recovery and connected API acceptance
  are not claimed.

```text
PairingViewModel.kt 4e08ae3d186d0a3dfefc91a5af26f7c58c8948061cf7565d13ece8baa7e5b14b
ScannerAdmissionTest.kt bd00996ca19eb5cff55a6aad56385c4ba4b812d3e646203d728257cacdd328ff
ScannerResultTest.kt 487e889117ef4a970c9517ca4ba9b467002c33bc6b250fca9d46df137eebc67c
stage2_scanner_android_smoke.py b9b12d1cc7d71d2a5aa763406f1d96e3bcbc1b5aae1c2d3b09ee188c8176ae13
test_stage2_scanner_android_smoke.py f5a08a72f2b5f6061866ea8d322a9377b6e9819bf39a5f9674011849504cc11d
debug-host-receipt 06c70100d9496d38e8128dfe1ed06e29ac935cba3b2c2b1f570f9159f50dcfce
main.jar 0f13df9e31172f0418c61a4fa81951d8668489aa5f8e1169da3e7f49c1070fd3
unit.jar d0916dd16a673850824c8e83cc99107d2aac5c6ef9118674de1bea9278c22fb7
instrumented.jar a2d7ab9d56a37c3a8506707a1fe769e8173bfcab35e66d130e7baf13dcd6ec75
```

Goal remains active. Source-bound APK, physical QR/direct LAN bidirectional
communication and recovery, full parity/API lifecycle and signed release remain
incomplete or unverified. No entire Blueprint item acceptance is claimed.
