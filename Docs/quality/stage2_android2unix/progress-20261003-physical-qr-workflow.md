# Operator-Assisted Physical QR Acceptance Workflow

Provisional S2-021/S2-031/S2-032/S2-051/S2-054 progress on 2026-10-03. Previous
goal turn changed invitation-bound confirmation source/tests, so it is progress.
Current LAN routing/scanner/harness evidence was inspected before this addition.
No nested agents, worker admission, checkbox promotion, commit/push or cleanup.

## Missing Acceptance Path

Existing host/device harnesses pass invite URIs into Android. Production Rust
QR pixels decode in JVM fixtures, but neither path proves a physical camera
scans the visible production terminal. A separate operator-assisted path now
connects production MainActivity/ScanContract, manual confirmation, authenticated
Read grant and actual desktop transcript display. It is not a product shortcut.

stage2_physical_qr_android_smoke.py requires attributed current APKs/host,
explicit physical device and API, private LAN literal, empty adb reverse table,
a visible terminal at least 140 columns by 100 rows and explicit camera-run flag.
It creates an isolated production provider/session/desktop fixture and displays
the real terminal QR. Independent desktop decode checks the interface and cert
pin. Only public expected endpoint/pin/project are passed to Android, never
invite URI/token/SAS. The operator permits the camera, points it at the screen,
compares the SAS, types it and taps confirmation. No automation enters the code.

PhysicalQrHostTest refuses existing paired metadata or signing keys. It checks
confirmation before any registration, then authenticates, proves no project
grant, and reads the actual provider-seeded transcript after a host-local Read
grant. The production screen must display the same text. Only fixture-created
phone metadata/key are wiped; existing data is neither cleared nor overwritten.
Named camera/confirmation/registration/transcript phases and a paired JUnit
start/success identity are mandatory. The host verifies unchanged read-only
journal, build/harness digests and no reverse forwarding; invitation text/token
are redacted from saved output. Cleanup failures cannot publish a passing receipt.

This bounded path proves only operator-assisted physical scan/direct LAN Read
when actually run. It is deliberately separate from emulator suites and does
not accept full writes/parity/lifecycle/release. No real camera execution has
occurred here, and no successful physical receipt was created.

## Verification

- New Python tests reject emulator serial/qemu, wrong API, foreign/missing/
  duplicate/skipped JUnit identities and missing/reordered phases. Two pass.
- Stage2 Python discovery: 103 passed; lifecycle verifier: 18 passed;
  instrumentation/Blueprint tests: five each passed. CLI --help exits zero
  without demanding a device lock/serial. Early help-selector failure was
  fixed, not treated as camera evidence.
- Final Android source check /private/tmp/stage2-physical-qr-workflow-final-20261003:
  main/unit/instrumented compile and 378 JVM tests pass. New physical test is
  compiled, not executed; automatic desktop suite remains 71/72 methods.
- Strict Blueprint, Rust formatting and git diff --check pass. All 36 rows
  remain unfinished. Rust debug/release verify-only pass; Rust is unchanged.
- APK verify-only still refuses missing provenance. No APK installation,
  unchanged blocked Gradle/ADB retry or physical QR/LAN execution occurred.

```text
stage2_physical_qr_android_smoke.py 7e2d174aba408bf7e2e4cba57f843b750d6a0e76660d3632eff5cc66ce7c186f
test_stage2_physical_qr_android_smoke.py 16ae1ec88c351abf23f87f0c26242d58c27e4f6f2be574654d2722a8ee8dbf72
PhysicalQrHostTest.kt a947591218bf8858f5be7eba70b177c7a138f29d90d9e9acd6a47b1aab9a01eb
main.jar 77481c863ef0f2ba79568da8eb60ae0bb74b4846ee1c65ccb3dfcbd5a16bae7e
unit.jar a2a24570e2dffdd018d635b161598dad8b0ab38eb3d269fe7355ed89784f395f
instrumented.jar 28b97cc95283b8902bb3f54907e0dfa76ee6a3cd899fa522a03c0d9dbc5e0c62
```

Goal remains active. Current APK, this actual physical scan/direct LAN run,
complete desktop sync/parity, supported API lifecycle matrix and signed release
still require behavioral acceptance.
