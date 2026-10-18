# Stage 2 Pairing Preconnect Admission

Recorded 2026-10-04T06:58:14Z. Partial S2-031/S2-032/S2-038/S2-053
evidence, not acceptance. Prior goal turn progressed LAN address-state evidence.
Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no
nested agents, reset, push or permission bypass. All 36 Blueprint rows remain
[ ]. Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

PairingClient captured a public identity before its connection but checked only
coroutine cancellation around LAN binding. Key deletion/replacement or invite
expiry during preparation could therefore still initiate TCP/TLS before the
existing protocol-handshake refusal. This did not authorize token transmission,
but missed the intended preconnect fence.

PairingIdentityProof.requireFresh now checks cancellation and invitation expiry,
validates the captured public identity, then repeats cancellation/expiry after
the potentially blocking Keystore read. PairingClient invokes it before LAN
binding, after binding before TCP connect, and after connect before TLS wrapping.
No replacement key, default-route fallback or token retry is introduced.
This does not eliminate races after the last observation or claim an atomic
kernel/Keystore transaction; existing handshake and receipt checks remain.

Expected/actual JVM: healthy identity remains admitted, replacement refuses,
expiry before a key read performs no read, wall/monotonic expiry during a key
read cannot admit the next stage (including rolled-back wall clock), and
cancellation during a key read preserves CancellationException. Three new
tests pass. Prepared LanSocketRoutingTest deletes its task-local Keystore key
inside bind, expects HostAuthenticationException, exactly one bind, no TCP
connect, closed socket and no replacement alias. This test compiled only.

Adding the named device test correctly caused Python suite-identity and
lifecycle verifier failures because the explicit base suite count was still
35. Updated BASE_INSTRUMENTATION_TESTS to 36; named completion discovery remains
mandatory. Both suites then passed, without weakening any gate or inventing a
connected receipt.

## Verification

Source run 2026-10-04T06:55:08Z through 06:58:07Z, exit 0. No production/test
Kotlin source edits after launch. Canonical cwd; OUT and per-lane ZENPI_HOME
isolated under /private/tmp/stage2-pairing-preconnect-20261004. Credentials
unchanged. Only docs and Python suite-count metadata changed during the run.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-pairing-preconnect-20261004:
  current main/unit/instrumented compilation, 560 ordinary JVM tests, current
  production QR corpus and explicit Kotlin/Rust interoperability lanes pass.
  Includes six-test desktop target and actual Agent prompt/tool/running-worker
  cancellation paths. Not Gradle/APK/device acceptance.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': first exit 1
  for exact suite count 35 != 36; after correction exit 0, 137 tests, OK.
  Logs /private/tmp/stage2-pairing-preconnect-python-20261004.log and
  /private/tmp/stage2-pairing-preconnect-python-final-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: first exit 1 for
  incomplete desktop instrumentation fixture; after correction exit 0,
  25 verifier tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  current debug host reattributed, no Rust source change or release claim.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/socket/ADB constraints not retried or
  bypassed; no historical APK installed.

Final source log /private/tmp/stage2-pairing-preconnect-20261004.log.

```text
PairingClient.kt 45f3f5ed379270734a8859be6abef8ae4e7254c766b51e7ed17e9c1bbe52e134
PairingIdentityProof.kt bb81e3caab4dae884e71c7ad5c44d7921b9cad90e897bc7df1ab3ddd611194b7
PairingIdentityProofTest.kt ae3c8c407f8d6fa0ef5de794f0fc2311749fd153526c19b2543b37ea7ec7d523
LanSocketRoutingTest.kt 1bc71c646dce455d92762400c2a069b030cb732fe3c76ee06f5ca420814ec7ad
stage2_desktop_android_smoke.py 7586dcd3b4627466673ae071e766fbd8e1188423fb74114099615cddae84487a
source-log e9c3891ab637832148e94bfda4dd42b95f0b0a5692be6d922107397495cb228b
python-final-log 1e5669532599ed95e7d29d2d9bd557c5d87eb74bb195a6dcf926dd1237a93737
main.jar 7effaa3a8e4bdd468eb42887cf55e836c01d3735d307b469441bb4e1b6eef763
unit.jar c154c14cfca0374106d1a8194e5a9e2fc5e0ac485e4320ae8d4119b1b31ac628
instrumented.jar ccac01c5282245852b5c1fbece1592fc23f0db85f6cc4a809c50c841a95c1049
host-receipt 92ebc1434b9ec2290cdf15d8acbd556a96f9fa7e422b2740ba120ffb7bfb8521
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required product gates.
