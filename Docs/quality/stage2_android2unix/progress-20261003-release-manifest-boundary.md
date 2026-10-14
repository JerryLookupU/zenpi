# Release Manifest Admission Boundary

Provisional S2-071/S2-072 progress on 2026-10-03. Current merged-manifest
inspection had permitted unexpected permission declarations, shared UIDs and
instrumentation if the candidate was not explicitly testOnly. It also trusted
an exported component by allowlisted name without checking its component type.
These gaps are now refused before a signed APK can pass the inspection gate.
No signing credential, APK, runtime source or checklist state was changed.

## Changes

- Reject sharedUserId/sharedUserMaxSdkVersion and instrumentation declarations.
- Only allow the single AndroidX dynamic receiver permission declaration when
  a matching uses-permission exists. Require signature or equivalent 0x2 and
  exact attributes; deny duplicate declarations, permission groups/trees,
  normal/privileged/known-signer grants and unexpected attributes.
- Require named, unique release components. Exported system components must
  match the allowlisted name, component type and system permission together.
  Only the actual validated MainActivity element gets the launcher exemption.
- Added negative fixtures for each new refusal. Existing valid launcher,
  QR deep link and AndroidX signature permission fixtures remain accepted.

## Verification

- python3 tools/test_stage2_android_release_check.py: exit 0, 11 tests pass.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0,
  86 tests pass. Expected argparse error output belongs to invalid LAN input
  refusal fixtures, not failing tests.
- Backup validator: exit 0; backup unit tests: 5 pass; lifecycle unit tests:
  15 pass; Blueprint validator unit tests: 5 pass.
- Current Kotlin production/unit/instrumented compilation: exit 0,
  307 JVM tests pass (see offline-authority progress receipt).
- cargo fmt --all -- --check and strict Blueprint validator: exit 0.
  All 36 authoritative items remain unfinished.
- Release host source attribution: verify-only exit 0, receipt
  7afecd0cef72f3186acf28c1f8548acdc5d5275ddb58eed9d4dd0ecd0164016e,
  binary 9f1ff46c2fd82ccad171534cbde89b3f7c6b49babaef8fd0f7686ad3019caaa8.
- Current Android APK provenance: verify-only exit 1, missing provenance.
  No unchanged blocked Gradle retry, old APK install or device/LAN claim.

Read-only apkanalyzer inspection of the historical unsigned artifact confirms
it lacks current backup rule references/resources. Resource XML commands
refused the missing files. That artifact was already inadmissible because it
is unsigned and stale; these observations are not current release evidence.
No signed candidate is available to run the new rules end-to-end. Mocked
tool outputs prove checker behavior only, not APK signing or device behavior.

```text
stage2_android_release_check.py 039b4b8b6ab454ce7d10b1c360ab7f2fdbbb91653abb2c2e843c5998e9d585a2
test_stage2_android_release_check.py 904b70627086f838600600591b4905c21b5b0fbd4640f4fd68b90ebfe9edc532
```

Goal remains active. Current signed build, compiled backup contents, install/
upgrade, physical-camera QR/direct LAN and full product acceptance remain open.
