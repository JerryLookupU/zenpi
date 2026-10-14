# Prepared Production Android/Desktop Session Workflow

Provisional S2-036/S2-051/S2-054 progress at 2026-10-03T18:39:51+0800.
Previous goal turn is progress: current-session name control and subscription
behavior changed authoritative source and tests. This turn inspected current
terminal/lifecycle/pairing implementation and expanded the connected-device
acceptance surface, without narrowing the full QR/LAN synchronization goal.
No nested agents, worker admission, checkbox promotion, commit/push or cleanup.

## Added Workflow

SessionHostTest uses production PairingClient, LAN socket routing, encrypted
store, PairingViewModel and Compose buttons. It pairs to the supplied real
desktop invite, requires local Control grant, confirms rename without changing
transcript/event cursor/workspace revision, creates a distinct empty session,
reads the host historical catalog and confirms restoring the provider-seeded
conversation. Each replacement advances workspace revision exactly once. It
checks resolved encrypted pending state, current-session resume refusal and
cached display without Control authority. A restored-conversation screenshot
is prepared for capture. Fixture-local keys/store are wiped in finally.

stage2_session_android_smoke.py requires current APK/source provenance before
adb, installs the two current artifacts, seeds a conversation through the real
Rust provider/headless entry and launches the production desktop PTY host. It
independently reconstructs/decodes the terminal QR, verifies the advertised
interface and certificate digest, grants only the fixture device/project,
and observes ordered Android phases plus current desktop name reconciliation.
Loopback uses a fixture-created reverse mapping; --lan-ip requires an empty
reverse table and uses the direct host interface. Cleanup stops the fixture
app and exact terminal, removing only its own reverse mapping.

The evidence validator requires an actual original provider turn, exactly one
rename effect with reserved/success identity, a distinct empty new journal and
remote new-session transition, two ordered successful mapping receipts with
contiguous revisions and matching owner/device/project/request/fingerprint.
Final checkpoint maps back to the original journal. Metadata-only/duplicated/
missing/wrong-scope effects fail. Successful instrumented count, ordered phases,
desktop callbacks, APK/source provenance and host/harness hashes are rechecked.
Receipts never persist the invite; camera/physical-phone LAN/whole-item claims
remain false. This harness prepares evidence, it does not assert that it ran.

## Verification

Commands ran from /Users/mac/Github/zenpi. No credentials/signing environment or
permission bypass. No Rust or production Kotlin source changed this turn.

- python3 tools/test_stage2_session_android_smoke.py: final 5 passed, exit 0.
  Complete chain, duplicate/missing effects, foreign identity, revision/name/
  fingerprint failures, pre-adb stale APK fence and exact instrumented count/
  ordering/desktop callback requirements verified with deterministic fixtures.
- python3 tools/test_stage2_android_build.py: 10 passed, exit 0.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-session-workflow-verified-20261003: exit 0; production,
  unit and instrumented Kotlin sources compile; 293 JVM tests pass. Device
  SessionHostTest is compiled only, not executed. Existing compiler warnings
  and nullable BigInteger-constructor warnings remain visible.
- Initial source check failed because cleanup called a nonexistent static
  DeviceIdentity.remove and accessed its private alias. Corrected to the real
  identity.wipe API. The failed run is not claimed as a pass. Source checks
  after correction succeeded; final run covers the local-grant refresh fence.
- python3 tools/stage2_session_android_smoke.py --serial emulator-fixture
  --expected-api 35 --receipt /private/tmp/stage2-session-current-device.json:
  exit 1, correctly refused missing current APK provenance before adb/device/
  host invocation. This is refusal evidence, not connected-device acceptance.
- python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0,
  all 36 items unfinished. git diff --check: exit 0.

All observed command handles are terminal. Earlier blocked Gradle/ADB/TCP
conditions were not bypassed or retried unchanged. Current APK, real device
interaction, physical camera/direct LAN, full parity, connected lifecycle and
signed reproducible release remain incomplete or unverified. Goal is active.

## Digests

```text
SessionHostTest.kt 3b457d6fc670303951798c08f06e0b87669c718080e8ec50095c02b2b46f1181
stage2_session_android_smoke.py 8a282082bc2a299695c193c38df968adde992b44dbba5c8c41e126853d4ca44e
test_stage2_session_android_smoke.py 96d56a1ce574bd85ab3536678a8c3774071a4625bf39083e84b2207fe5974360
main.jar 2b8a40eff37869a2558ae93d0965d18e35b47737f26be7bf6b92e30904dead27
unit.jar c384d7f3b11514660c53b16bf20e38deb7154b640d934b70260e4e8f9a8e2351
instrumented.jar 0a4f53fb4c65757403ddedb186987c72cdfbd5212a14c38adc041de4e7b959c0
```
