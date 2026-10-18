# Attributed Release Host And Signed APK Inspection

Provisional S2-070/S2-071/S2-072 progress at 2026-10-03T19:31:41+0800.
Previous goal turn was progress: safe host help entry changed Rust source and
tests and refreshed debug attribution. This turn inspected Gradle signing,
manifest and release docs and ran a current-source release build. No nested
agents, worker launch, checklist promotion, commit/push or cleanup.

## Work

Current Rust release binary built through stage2_host_build.py --profile release,
with unchanged source inventory and structured Cargo artifact attribution.
Final verify-only and actual release host pair --help succeed. Existing release
doc wording implied reproducibility from a single locked build; corrected to
require independent repeat-build/toolchain/input evidence.

New stage2_android_release_check.py is read-only candidate inspection, not a
signing/build/install helper. It requires an operator-trusted certificate
SHA-256, exactly one signer, real apksigner verification for API 31..35,
zipalign 16 KiB native/4-byte alignment and structured apkanalyzer merged XML.
Manifest checks package/version/API, non-debuggable/non-test-only application,
backup/cleartext policy and backup rule references, bounded permissions,
absence of FGS and exact launcher/QR route. Unexpected exported components
are refused; known WorkManager/profile receiver components must retain their
system signature permissions. APK mutation and existing receipt overwrite
fail closed. No passwords/private key/signing operation or adb invocation.

Receipts explicitly keep source attribution, install, upgrade, reproducibility,
compiled backup-rule content and whole-item acceptance false. A passing static
candidate inspection cannot authorize release. Docs clarify signing values:
none produces unsigned output, partial values fail, all four enable signing.

## Verification

Commands ran from /Users/mac/Github/zenpi.

- python3 tools/stage2_host_build.py --profile release: exit 0; final
  --profile release --verify-only: exit 0. Source/artifact hashes below.
- target/release/zenpi host pair --help: exit 0, actual current CLI options.
  No production listener/LAN pass claimed.
- python3 tools/test_stage2_android_release_check.py: nine passed, exit 0.
  Positive structured fixtures, wrong/duplicate/missing signer, unsigned tool
  failure, alignment failure, malformed policy/QR/exports, mutation and
  overwrite negatives covered. Mock signed fixture is not a real signed APK.
- Real release_check against the historical app-release-unsigned.apk, with
  synthetic expected pin a*64: exit 1. apksigner rejects it before alignment
  or manifest validation. Failed receipt saved at
  /private/tmp/stage2-unsigned-release-inspection-final-20261003.json.
  Artifact unchanged; this is unsigned refusal, not certificate comparison
  success or current-source evidence.
- Read-only apkanalyzer manifest print against the historical unsigned APK:
  exit 0, shows old merged manifest missing current backup-rule references.
  This reinforces it cannot represent current source, not an acceptance pass.
- SDK apksigner verify --help: exit 0. zipalign -h: exit 2, unsupported help
  flag printed its check syntax; not claimed as an alignment check.
- Python host builder eight and APK builder ten tests pass, exit 0.
- git diff --check and strict Blueprint validator: exit 0, all 36 unfinished.
- APK verify-only: exit 1, missing current provenance. No unchanged Gradle/
  ADB/TCP retry, signed APK creation, stale install or sandbox bypass.

No production Rust/Kotlin source changed this turn. All required handles are
terminal; no fresh JVM/Compose/device runtime tests claimed.

```text
stage2_android_release_check.py 59431fb345a2a703e9b9ea1bb42ed59871dbd55bb84db601e8dc5e94186702c3
test_stage2_android_release_check.py 588b691043e99087033bfae08b90c72e042b57057026ec354af2cba5b3732290
failed inspection receipt 43b350599fdcbfe6ba3232e2cca80aa42ea47be83b835779bdc2d6775c9c7e09
host source inventory cebe982976891747331085b9f288874fedc1bf6ed8fccfe2169747863522cf1e
target/stage2-host-build-release.json 7afecd0cef72f3186acf28c1f8548acdc5d5275ddb58eed9d4dd0ecd0164016e
target/release/zenpi 9f1ff46c2fd82ccad171534cbde89b3f7c6b49babaef8fd0f7686ad3019caaa8
historical unsigned APK 53f990e21f467a4d8832fd21737086fb278ae64c22783e36a8dae766f6a2eb72
```

Goal remains active. Fresh signed/current-source APK, physical camera/direct
LAN, connected UI/lifecycle/upgrade, complete parity and independently
reproducible release remain open; inspection tools do not replace those gates.
