# In-Flight Pairing Deep-Link Fence

Provisional S2-031/S2-037/S2-051/S2-054 progress on 2026-10-03. The previous
goal turn changed the physical message-roundtrip workflow and passed source
gates, so it was progress. This turn inspected current scanner, invitation,
network inventory, socket ownership, credential save and lifecycle sources.
No nested agents, worker admission, checklist promotion or unrelated cleanup.

## Repair

MainActivity.onNewIntent calls PairingViewModel.acceptUri directly. Unlike
scanResult/beginScan, this path previously allowed a new valid invitation while
first pairing was CONNECTING. It cancelled the old connection and advanced its
generation even if the old host had accepted the device or a synchronous save
was already owned. A subsequent storage reconciliation was not guaranteed by
the replacement path. This could leave the displayed confirmation inconsistent
with a late persisted old host.

acceptUri now refuses replacement in that first-pair connecting window, before
parsing or cancelling anything. The existing explicit cancel and background/
network interruption paths remain responsible for reconciliation. Saved-host
read refreshes are not classified as first pairing, and replacing an unsubmitted
confirmation remains allowed. Queued cold-start/reconciliation invitation
handling and pending-write/revoke fences are unchanged.

PairingCancelTest now models CONNECTING for explicit cancellation as well as
background/network cases. While storage is held, each case sends a distinct
valid invitation through acceptUri and requires identical connecting state,
closed scanner admission and an active old save-owner Job. It then exercises
the existing interruption/reconciliation and saved/absent-record outcomes.
This regression compiles but requires connected-device execution; the fixture
does not fabricate a successful transport registration.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-pairing-deep-link-fence-20261003: exit 0. Main/unit/device
  sources compile; 385 JVM tests pass. Existing deprecation warnings remain.
- Stage2 Python discovery: 110 passed; lifecycle verifier: 21 passed;
  Blueprint tests: 5 passed. Device suite identities/counts are unchanged.
- Strict Blueprint, git diff --check, rtk cargo fmt --all -- --check and
  debug Rust source attribution pass. All 36 Blueprint rows remain [ ].
- APK verify-only refuses missing current-source build provenance. No existing
  APK was installed and no unchanged blocked Gradle/ADB/TCP attempt was repeated.

```text
PairingViewModel.kt 3188ebbbe623cdf3f89edc63b3396e9937394d37e94a59e9fa6aeb973ec9d21f
PairingCancelTest.kt 6c67356acefd28b003131bdd05fe7123399201375bb4b47483a41372d2898bd6
main.jar c8876b4ff20805c6c1ef062eb0d330576b413fa3a2a9e465661154838bb2c307
unit.jar d6e97fcb83b5e200852163830fcb7fe9a9ca8cfe88947267c5098edb718ea188
instrumented.jar 9c01be541d722e853a403fed9a45163d9a8b57855425f856a510334d2142f2c5
debug-host-receipt 7020b95d66d5de27c6e83e403914087f6731954002d9fe0b2f56c19f5b81367a
```

Goal remains active. Current APK, actual camera QR/direct physical LAN desktop
roundtrip, full parity/API lifecycle and signed release are still incomplete or
unverified. Compiled lifecycle tests are not real-network acceptance evidence.
