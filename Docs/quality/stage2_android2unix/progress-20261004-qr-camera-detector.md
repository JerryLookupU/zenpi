# Pairing Camera Detector Repair

Provisional S2-021/S2-031/S2-036/S2-038 progress. Previous substantive goal
turn added transcript admission fences and retained a reproducible QR failure;
it was progress. The intervening RTK request verified the installed local tool,
not Android acceptance. This turn inspected the execution skill, current
Blueprint/specification, dirty worktree, production QR renderer, scanner and
cached JourneyApps/ZXing APIs. No nested agents, reset, push or checkbox
promotion. Existing edits are preserved.

## Root Cause And Fix

The exact retained 260x264 production terminal image still fails ZXing core
3.4.1's single QRCodeReader. Reflection-based diagnostic of the installed
finder reports confirmed centers (30,30), (230,30), and a false interior
(58,116), each module size 4/count 2. The finder stops before the real
bottom-left center. Extra white margins and TRY_HARDER did not repair it;
PURE_BARCODE could decode it but is not appropriate for camera scenes.
ZXing's existing QRCodeMultiReader geometric detector decodes these same
pixels without pure-image extraction. No QR encoder or wire change is needed.

PairingQrReader now uses that existing detector and admits exactly one result
with real geometric result points. Multiple decodable symbols, blank frames,
and merged structured-append results are refused rather than selecting a
host. Only result-point callback hints are forwarded; PURE_BARCODE cannot
enter this path. Existing invitation parser, expiry, pinned identity, human
safety-code confirmation and TLS admission are unchanged.

MainActivity launches private in-process PairingCaptureActivity, which replaces
CaptureManager's default decoder factory after intent initialization and before
camera resume. JourneyApps still owns camera permission, lifetime, timeout,
cancellation and result delivery. JVM tests invoke the same JourneyApps Decoder
factory, not just an unrelated standalone decoder. Entry verification refuses
missing/duplicate/exported/out-of-process scanner configuration.

The original failed image and expected metadata are copied unchanged into
android/app/src/test/resources/qr-regression. Digest-pinned tests reproduce
the old detector failure, decode the new path, check matching safety code and
expiry, and cover all four rotations with off-center/perspective synthetic
scenes plus multiple-symbol/blank-frame refusal and callback preservation.
The current Rust fixture manifest expands from 6 endpoint cases to 54 with
48 deterministic varied fingerprint/token payloads. Public fixture values
are not live credentials. Tests do not prove every possible payload or pose.

ScannerActivityTest additionally monitors the actual production capture
Activity and checks its decoder factory. The device harness requires the
unique production_pairing_qr_factory_verified marker; legacy output cannot
prove this repair. Device execution remains pending, not silently passed.

## Verification

Canonical cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. No whole-item acceptance.

- First development checker had a manifest mismatch because fixture generation
  preceded the in-flight test edit; failure retained, not counted as success.
- Complete checker /private/tmp/stage2-qr-camera-reader-20261004 exit 0:
  512 ordinary JVM tests, 54 current QR images, main/unit/instrumented source
  compilation, and all explicit Kotlin/Rust owner interop lanes pass.
- Final complete checker /private/tmp/stage2-qr-camera-final-20261004 exit 0
  repeats 512 ordinary JVM tests, 54 QR images, all three source compilations
  and every explicit interop lane after the device-marker change. Final
  model/resource/file/diff/history Rust target: 5 passed. Instrumented tests
  compile only; no device execution is inferred.
- Python Stage 2 discovery: 137 pass. Lifecycle verifier tests: 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass, 36 rows [ ].
- Debug host build/verify-only passes. APK verify-only refuses missing
  provenance. No unchanged Gradle/ADB retry, permission bypass, stale APK
  install or physical camera/LAN claim.

```text
PairingQrReader.kt a914441f576ce8d17b55c40974d2772d38bf52abd5784c99adcad41406ae5036
PairingQrDecoderFactory.kt 8a53ce28035cc273b166d7223ca7416ca96d946fdbb818255ed48e32480a1027
PairingCaptureActivity.kt f6010e1ee4def0138004e9584be6d13ff67c5cb0efbaab9ab49b3335b8114f8d
PairingQrReaderTest.kt 42a702c0b7616f86fe00c89049dfe87da8a0b7963faf33c4a881356653b7b95e
ScannerActivityTest.kt 6aade851b18f5d0e913caddf09b51356e4ec9e5c2e2b735d09314b6b820f9b32
remote_qr_fixture.rs 90da3ee7ba1a1e40761d0f0cab8f28c4ddc8d3482608c9e383dfb9da53173064
original-pgm b877ab32979ca2e0e4ae10d9b0d396560551941c8f97bd0a2251af090166a69b
complete-source-log 62e774c125521c47b582d267600643b9ca8f3be782b6c90d105f42ad1c75e971
python-log f1a5b780bc405890c4dcbdbc5922f08dcc97418f6cdf0ca6047e7e6c9ca129a1
final-source-log 96956b6e16533f60b080f7eb6a71b89b584e44ea7a61f1d407a3dd2bdbb3e931
main.jar 5cf1ed68fd404813407262efccc2361e99ee402c6a90701949c165d786d9ab01
unit.jar 96ce4af7ef04bdae288ba2626edbc9aca5593fa892f638fe58b55b8f91be84d2
instrumented.jar dacd81423fc1a732ec6316e15f98c2f1155855e3c06256e43232499320b35356
debug-host-receipt 7c2c5e6b643d2023528a92424f38673e77e162420291207991a9c3292aeab256
```

Goal remains active and incomplete. Current APK, physical QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, full desktop
parity and signed reproducible release remain required. Source/pixel repair
does not substitute for any of those gates.
