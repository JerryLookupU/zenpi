# Background Resume Storage Cancellation

Provisional S2-033/S2-037/S2-038/S2-055/S2-056 progress. Previous goal turn
closed connection-handshake scope and was progress. Current execution skill,
Blueprint, dirty integration, encrypted store/CAS/reset, ResumeWorker/deadline,
existing connected storage/worker tests and lifecycle receipt gates inspected.
No nested agents, reset, commit, push or authoritative checkbox promotion.

## Defect and Fix

ResumeWorker checked coroutine cancellation and signing identity before calling
PairedHostStore.commitResume. That non-suspending method could then wait for
the shared storage monitor while a foreground commit held ownership. If the
worker was cancelled during that wait, it could still enter later and publish
its projection, as the only cancellation check preceded the blocking wait.
Record CAS protects concurrent persisted changes but does not by itself detect
worker cancellation when the ciphertext is unchanged.

commitResume now accepts a checkActive callback and invokes it after monitor
acquisition, before reading/decode/CAS. It forwards the callback to the common
write method for a final check after cache encoding/encryption and immediately
before AtomicFile.startWrite. ResumeWorker passes its captured coroutine context
and captured saved-device identity check. Cancellation propagates rather than
requesting retry. Existing callback-free foreground/store callers retain their
behavior. Atomic publication already in progress is not claimed to roll back.

Added ResumeStoreTest regression using real isolated encrypted storage and
Android Keystore metadata. A separate thread holds the actual storage monitor;
the coroutine IO thread is observed in BLOCKED state, then cancelled before
ownership is released. Its first in-lock callback must throw and encrypted
bytes, cursor and display remain unchanged. A second scenario throws at the
last pre-write check, and a later live commit succeeds with both checks. Cleanup
joins owned tasks/threads and wipes only the unique fixture namespace.

This new instrumented method compiles, not connected-executes. Desktop suite
counts are 33 base and 85 with workspace/Doze. The production harness and API
lifecycle verifier require resume_storage_wait_cancellation_fenced_verified
as a named phase and successful gate, in addition to exact named test completion.
Negative receipt tests refuse missing/duplicate/substring phases or missing gate.
Historical suite counts cannot be acceptance evidence for this fix.

## Verification

Cwd /Users/mac/Github/zenpi, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty integration preserved.
Final observed UTC 2026-10-03T22:02:52Z (Oct 4 local time).

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-resume-storage-cancel-20261004-final: exit 0, pipefail/rg
  output filter; current QR fixtures, Kotlin main/unit/instrumented compilation,
  475 ordinary JVM tests and all existing current interop lanes pass.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': 135 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: 24 pass.
- Backup and Blueprint validator unit tests: 5 each. Strict Blueprint passes,
  all 36 rows remain [ ]; Rust fmt and git diff --check pass.
- Debug host rebuilt after source-check Cargo lanes and separate verify-only
  passes. Host binary hash remains unchanged: this fix is Android-only.
- APK verify-only refuses missing provenance. No unchanged blocked Gradle/ADB
  retries, permissions bypass, stale APK install, physical camera/LAN or actual
  WorkManager/cancellation acceptance is claimed.

```text
PairedHostStore.kt 38402af2a868c3ab4a6fe5315867f8ccf2a748331267c3e443a9a1d47fd5f259
ResumeWorker.kt f98e68a07a8ee6dbb2c19cf1d5317bc81690b17a1b3dae872a7ddcb224a943d5
ResumeStoreTest.kt 575b319df429a876cda4671f3e4377e8b91b6a4fcadc2432f5ab0de7e3e9ef3d
stage2_desktop_android_smoke.py 8946d0dffb6b435c306bf8b4baef372fc5d5e8b5dfe0ca10c1046c294296c057
validate_stage2_android_lifecycle.py 29c34c96525c200420233b139db2d8a66b1ebdde3e690607e4466be4c294868e
test_validate_stage2_android_lifecycle.py 3e92d8a30b1a15a5b6441018fe00547c2e543e0ae5dc54bdcea4ba366b476eb8
main.jar bca8369fad7c12940d683977b64512a689a710f6a270a7b67696445f65248390
unit.jar 1251c4a3ae844f3aa9ace8a4cbfe7fbdd75f7b18aabec0a4ef5db57c42bf2544
instrumented.jar 49475d299643c1c896973be032d24d1fcccd05ec51276465bcd41c27a67a6516
debug-host-receipt 9c37e7cced3ce230365242c14ffa189a40fa4a3848af6d3334aa64311b31a53b
debug-host-binary 8a826eef42d30492ace13ea37eb454d6c7819bf91251beb6942dcd683cd039e9
```

Goal active and incomplete. Fresh source-attributed APK, physical camera QR/
direct-LAN two-way synchronization and recovery, connected API31/34/35 lifecycle,
complete parity, migration/reset/uninstall and signed reproducible release
remain unaccepted gates.
