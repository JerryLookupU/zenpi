# Generated Schema Build Admission

Provisional S2-030/S2-071 progress on 2026-10-04. Previous turn generated
Android protocol enums and passed source tests, so it was progress. Current
generator, guarded APK builder/provenance, Gradle preBuild wiring and fixtures
were inspected. No workers/nested agents were admitted; all Blueprint rows
remain unfinished. Existing dirty-worktree changes are preserved.

## Repair

The guarded builder previously relied on Gradle preBuild for schema checking.
It could start Gradle on stale generated source, and verify-only compared
hashes without independently validating that source matched its contract.

The builder now checks generated schema before starting Gradle and again after
successful completion. Current-build verification also checks it before an APK
can be credited or installed. A fabricated/matching inventory cannot promote
stale generated enums to authority. Preflight failure removes old builder
authority, preserves the edited source, does not auto-regenerate, does not start
Gradle and does not fabricate a process-attempt log. Existing Gradle preBuild
check remains an independent gate.

Three Python regressions prove stale source fails before subprocess execution,
a matching success record cannot bless stale schema, and malformed contract
fails without rewriting source. Fixture builds now include real reviewed
contract/generated content so success-path tests exercise the same check.
No production Kotlin/Rust behavior or protocol version changes this turn.

## Verification

- python3 tools/test_stage2_android_build.py: 16 pass.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': 125 pass.
- python3 tools/generate_stage2_android_schema.py --check: current.
- Lifecycle verifier tests: 21 pass; Blueprint verifier tests: 5 pass.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 rows remain [ ].
- Shared builder edit invalidated Rust attribution; a real offline locked
  python3 tools/stage2_host_build.py completed and refreshed debug attribution.
  The final production Rust binary hash is unchanged.
- Kotlin production/unit/instrumented sources are unchanged from the prior
  generated-schema source check with 420 passing JVM tests. That check is
  historical source evidence, not a fresh APK/device run this turn.
- APK verify-only refuses missing provenance. No unchanged blocked Gradle/ADB/
  TCP retry, stale APK installation or physical camera/LAN workflow occurred.
  This repair's preflight is verified with deterministic fixture tests; actual
  Gradle packaging and connected execution remain unverified.

```text
stage2_android_build.py 0d49538fcd5d2e9c2f5013bb59db46190878348a8f3a7010d74570146b194667
test_stage2_android_build.py ef800c1646dfc501ecc827b5f37921e4cdcb66cfa58259396035c0aa51d34432
debug-host-receipt 8a5290618441efe7e654303e98923bedb59ea6f20195f9e868fdef3f46c72bc2
debug-host-inputs f70bce4a61de3f9718a7be82d6ef3bca4f10e9800ef8e7827f34a13f141883ea
debug-host-binary 57b52b80beda551642c47cd1f404f2fd1400b7a16699f91401402cb77db227a8
```

Goal remains active. Source-bound APK, physical QR/direct LAN communication
and recovery, full shared schema/parity/API lifecycle and signed release remain
incomplete or unverified. No entire Blueprint item acceptance is claimed.
