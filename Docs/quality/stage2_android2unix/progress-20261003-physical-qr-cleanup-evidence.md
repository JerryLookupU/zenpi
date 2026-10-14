# Physical QR Cleanup And Saved Evidence Verification

Provisional S2-031/S2-037/S2-051/S2-054 progress on 2026-10-03. Previous goal
turn added the physical camera workflow/source checks, so it is progress.
Current test and harness were inspected before this repair. No nested agents,
worker admission, checkbox promotion, commit/push or unrelated cleanup.

## Repairs

PhysicalQrHostTest previously deleted storage directly while the production
ViewModel could still be connecting or writing its cache. It now obtains that
same Activity-owned ViewModel, calls the actual forget/reset path and waits for
cancel-and-join plus encrypted metadata/key removal. It verifies no remaining
host/reset-required state and cancels resume scheduling. Cleanup completion is
a separate required instrumentation phase, not implied by test-body success.

The harness checks both Android packages are absent before installation. A
dedicated physical device with an existing main/test package is refused, not
upgraded or data-cleared. The Android check for existing pairing/key remains
an additional fence. Only fixture-created phone state can enter cleanup.

Physical receipt v2 adds exact public-only invocation, endpoint/project,
instrumentation exit code, named completed test identity, cleanup flags and
sanitized registry state before/after the host-local Read grant. No signing
key/public key or invite token is saved. The saved-output hash, all five phases,
provider seed, unchanged journal and current app/host/harness attribution are
rechecked via --verify-receipt without adb. Extra invite arguments, old schema,
changed artifacts, false cleanup or a skipped test cannot satisfy acceptance.
The final publication path revalidates and downgrades the receipt if it fails.
Missing saved evidence exits nonzero with a generic diagnostic, not a traceback
or acceptance claim. This is local evidence consistency, not cryptographic
attestation or a substitute for actual physical execution.

The decoded camera-target QR is checked with the existing host-smoke
invite-envelope validator before starting instrumentation: exact fields and
version, valid one-use token, fresh <=300-second expiry and usable endpoint,
followed by the chosen private LAN address and certificate digest. Harness
attribution now includes the terminal rendering/drain helpers, invite validator
and build validators, not just direct imports. The output-path fence checks the
actual requested receipt (including non-json suffixes) and dangling links;
neither an existing receipt nor its instrumentation output is overwritten.

## Verification

- Six physical verifier tests pass, including regular-file/hash checks,
  missing/changed cleanup/test/output/argv/source/build evidence and existing
  package refusal, malformed/expired QR, mismatched LAN/pin and existing output
  paths/links. Missing --verify-receipt input exits 1, no receipt published.
- Stage2 Python discovery: 107 passed; lifecycle verifier: 18 passed;
  instrumentation/Blueprint tests: five each passed.
- Kotlin source check /private/tmp/stage2-physical-qr-cleanup-final-20261003 exits 0:
  main/unit/instrumented compile and 378 JVM tests pass. Physical test compiled
  only; automatic desktop suite is still 71/72 methods.
- Rust debug/release verify-only, formatting, git diff --check and strict
  Blueprint pass. Rust is unchanged; all 36 rows remain unfinished.
- APK verify-only refuses missing provenance. No APK installation, blocked
  Gradle/ADB retry, physical camera run or successful physical receipt exists.

```text
stage2_physical_qr_android_smoke.py e739c42114d9f2420e49d56bdf74061a3f5cd7378b1e07bdd3a9eaa9bacb4060
test_stage2_physical_qr_android_smoke.py 97b6eea9fd8f84f098943fc080855e6c9841a96a5b1f921343b76e9f1ee9a4b1
PhysicalQrHostTest.kt 9fbe556a975f7df863aa97eaf00e0a554f2de0681c2360769d8e3527e53b1bf4
main.jar 77481c863ef0f2ba79568da8eb60ae0bb74b4846ee1c65ccb3dfcbd5a16bae7e
unit.jar a2a24570e2dffdd018d635b161598dad8b0ab38eb3d269fe7355ed89784f395f
instrumented.jar 8db3630fa4b3cc8927dedfb8de2af2d3f41c3e1143c9a84df8225be53e7df610
```

Goal remains active. Current APK, actual camera/direct LAN desktop sync, complete
parity, supported API lifecycle matrix and signed release remain unaccepted.
