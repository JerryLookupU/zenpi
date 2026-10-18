# Android Synchronization Recovery Status

Provisional S2-033/S2-038/S2-053 progress on 2026-10-04. Previous goal turn
changed the actual subscription wire budget and verified host/TLS/source
regressions, so it was progress. Current execution skill, dirty worktree,
Blueprint, Android replay/snapshot reducer, Compose status and cache inspected.
No workers/nested agents admitted; no authoritative checkbox promotion.

## Implementation

Validated replay-gap or branch replacement previously changed the transcript
silently while the phone showed only Connected. HostProjection now has a
transient snapshotReplaced observation, set only after the existing scoped
snapshot and terminal phase/journal/cursor checks succeed. Ordinary replay
clears it. No wire field or host behavior changed.

The connected screen shows Catching up when a verified replay tail remains,
otherwise Snapshot replaced for the replacement observation. Offline/error,
scanner/reset and unresolved write/revoke states hide this live status. The
cache codec does not serialize the observation; offline restore cannot claim
recovery. It grants no authority, acknowledges no cursor and resolves no write.
The current subscription heartbeat may clear the observation promptly; this
is a current-read status, not a permanent completion banner or recovery audit.

JVM tests cover validated replacement, ordinary replay clearing, cache omission,
tail priority and all non-live fences. Prepared Compose assertions check both
labels and offline removal in the existing PairingScreen test. Scanner harness
requires its unique sync-status marker; verifier negatives reject missing,
duplicated or old markers. Compose/device tests compiled only, not executed or
visually inspected on a running Android surface.

## Verification

Commands ran from /Users/mac/Github/zenpi against current dirty integration.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-sync-status-20261004:
  exit 0; production/unit/instrumented sources compile; 450 JVM tests pass.
  Existing warnings remain. No connected test run.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': exit 0; 131 pass.
- Scanner verifier tests 14 pass; lifecycle verifier tests 21 pass;
  Blueprint verifier tests 5 pass.
- Strict Blueprint, Rust format and git diff --check pass; all 36 rows [ ].
- Host verify-only attribution passes; Rust source unchanged this turn.
- APK verify-only: exit 1, missing provenance. No blocked Gradle/socket retry,
  stale APK installation, ADB/camera/direct LAN or screenshot acceptance.

```text
HostClient.kt c4beab39a5d4bdabc3c22412e192e0974784444857de844fa999bc64b59f64b2
SynchronizationStatus.kt aa75c19429e134ba23edde5e194ea6174bb1986f068bf1efacb3489f165213e6
MainActivity.kt 40cbf52d34d06d3d121256fac253e5f270979a5f400e01519cdcc82bc0910f0a
ReplayTest.kt 677a6990d532edd51f2c86c86d14a3e83cec2da25fd3cb8bd1f36a6f1bf172ee
SynchronizationStatusTest.kt 741441a4364a327635c59f4f78537385cda22aeb2da4ebae4dd19cd609536b60
PairingScreenTest.kt 6857e600c72ebcd88c8c98885e52339e71e89155c4a9a42d318002a156f511ee
stage2_scanner_android_smoke.py c124bb57467d29c5337ac5481c725d4dfd0931817e4f7ef80b707a731ca51fae
main.jar b038f51080bc22a45f41892aeb52151bd21c77535aa589893c2aecd756f139c7
unit.jar c35a42e3feb28f9046e6ed8c25cf1c924f8b30b74cf67a4fe427e2e6e3f64fc9
instrumented.jar 9520e4b24c1da2eccc57cc7969e76844851c918e740dc5e66a569b77c5eb9573
debug-host-receipt 4b3615a4ec77a551bb958ed7f0742dd4ffb65953e8739833ed41490cd1376993
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active. Current-source APK, physical QR/direct LAN bidirectional
sync/recovery, connected API lifecycle matrix, full parity and signed release
remain incomplete or unverified. Source/policy tests do not accept those gates
or an entire Blueprint item.
