# Stage 2 Revocation Identity Removal Cancellation

Recorded 2026-10-04T08:57:03Z; partial S2-037/S2-038/S2-055/S2-056 evidence,
not acceptance. Canonical cwd /Users/mac/Github/zenpi, dirty baseline HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. User/existing changes preserved;
no nested agents, reset, push, socket permission bypass or stale APK install.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.
All 36 authoritative rows remain [ ]; zero [_]/[x]. Previous goal turn was
progress: production host smoke source attribution and endpoint validation.

## Finding And Change

PairedHostStore.finishRevoke checked cancellation/generation before invoking
key removal, but DeviceIdentity.wipe(expectedDeviceId) then acquired its
identity monitor, read the current Keystore identity and opened another
Keystore without rechecking ownership. A delayed identity-lock/Keystore read
could outlive foreground/network generation ownership and still delete the
matching signing key. This is a source-level cancellation window, not an
observed connected-device failure.

Expected-device wipe now accepts checkActive, uses AuthenticationProof.current
inside the same identity lock to bracket public identity reads, and checks
again after opening the deletion store immediately before deleteEntry. Both
direct self-revoke completion and saved-receipt reconciliation pass the same
storageAdmission closure to the store and identity operation. Local explicit
Forget/wipe remains unconditional. Missing/replaced keys still refuse, and
external Keystore mutation is not an app-local atomic transaction. Cancellation
after an admitted deleteEntry cannot promise rollback.

DeviceRevokeTest adds three admission failures with original cancellation,
unchanged encrypted bytes, retained signing ID and exact pending revoke UUID.
It also holds DeviceIdentity.Companion's actual monitor, cancels the coroutine
while its thread is BLOCKED, releases/joins it and checks unchanged key/intent,
then proves matching uncancelled cleanup removes both. Test artifacts are
compiled only: none of those real Android Keystore assertions were device-run.

The desktop control+Doze suite now requires 92 named tests (38 base, 53 control,
one Doze), the new revocation_identity_removal_cancellation_fenced_verified
phase and boolean. Source test enumeration reports 92/92. Lifecycle verifier
negative fixtures refuse missing boolean and missing/duplicate/unstructured
phase. Current instrumented jar contains the named phase, verified with javap;
this is compiled-artifact inspection, not execution. Historical receipts cannot
satisfy this expanded connected gate.

## Verification

zsh tools/stage2_android_source_check.sh /private/tmp/stage2-key-removal-cancel-20261004:
exit 0, current main/unit/instrumented compilation, 579 ordinary JVM tests and
all explicit Rust/Kotlin lanes passed. Includes terminal QR, software P256,
workspace/session/ACK/restart, model/resource/file/diff/history reads, real
prompt/tool and running-worker cancellation lanes. Isolated per-lane homes;
required process 55535 polled through exit 0, never restarted after timeout.
The final instrumented jar includes the added test/phase. Unit count is
unchanged because this new real-Keystore regression is instrumented-only.

- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 142 tests;
  /private/tmp/stage2-key-removal-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixtures,
  not connected API31/34/35 behavior.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0; 36 [ ],
  zero [_]/[x]. No product acceptance inferred from authority validation.
- python3 tools/stage2_host_build.py --verify-only during Cargo lanes: exit 1,
  binary changed since attribution. After the lanes terminated, build and
  --verify-only both exited 0. Final provenance binds the restored debug binary.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. No repeated Gradle/socket/ADB attempt or permission bypass.

```text
DeviceIdentity.kt b5af43f685c226435659ee0dd612f8bf382f9377010c0032e3c47ba91b77a41c
PairingViewModel.kt 9ecd3443ac23958e1fb04a211fd01eeff070450e349f99a44a578e6c0e3ee04d
DeviceRevokeTest.kt cdb27bb18e8514b25805c02108bdd356bee60a639d2248127be2ace028ae4962
stage2_desktop_android_smoke.py 7de3d08f85a9b2ef4876ff3ffcc8c72846160fc39a65bc6cee73e14d387e6a17
validate_stage2_android_lifecycle.py 3c347757ba81b9f223d0a7e7737299e8ad5ed08f1031e6d18fa135d7dad15f8d
test_validate_stage2_android_lifecycle.py a849b8c4bfefc8a684f2700d14692e1938ed458ca90c68f7d800f470efc73c3f
python-log 020c10d6d7b4aba806115b4e0d0cbfd1a0a261bbc27414a422a966417b435a93
source-log 1aba9490d5a43e57b9825dea20966b9809ab3c7ebd90e6694dd37f3c48eb7546
main.jar 708a2203eb46de4be09de1a432530bdaad92097aefb74199205f7cb45bda3418
unit.jar 3c3ed270a9f7b30654e593cbb39f977524a2d3d52e535d9ff6796bc4ac72c917
instrumented.jar f95a635bb1b8d33d66febe3cf15afe0db9afd4ca9d63fb00a5f126e9d1e69630
host-receipt f6551e2f6f1681173c591462c327c5a56f3c66ffa8e79c1cea79bbf3e023690b
host-inputs 08312e22d68633b57228db66f7d64511bc87dc758579124ef9de588e3a466219
host-binary 40f9f134ed62fd51a71dc6e8d24022f0122dced68faf12f6a52f8b5ea145607f
```

Goal active/incomplete. Fresh APK provenance, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, full parity and
signed reproducible release remain required. This source repair and expanded
acceptance gate are progress; neither completion nor blocked audit passes.
