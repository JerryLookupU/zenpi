# Generated Android Protocol Enums

Provisional S2-003/S2-030/S2-050 progress on 2026-10-04. The previous turn
added the shared enum/classification matrix and passed cross-language tests,
so it was progress. Current Blueprint, dirty worktree, contract, FrameCodec,
Gradle/source-check entry points and APK source inventory were inspected.
No workers/nested agents were admitted and no authoritative row was promoted.

## Change

The reviewed schema-contract.json now generates RemoteSchema.kt. FrameCodec
consumes its version, operation allowlist/read-write classification, outcome
states and error codes instead of maintaining separate handwritten enum sets.
The Rust enums remain independent and the shared matrix tests verify their
agreement with this contract. Existing acceptance behavior and wire version
are unchanged. Kotlin tests also compare all generated sets with the fixture.

The stdlib-only Python generator validates exact fields/version, bounded lists,
safe names, duplicate object/list members and disjoint read/write operations.
Its check mode refuses missing/stale/edited source without changing it.
Regeneration is deterministic and includes the contract SHA-256. Source/output
symlinks are refused. Four Python regressions cover these gates and stale checks.

Gradle preBuild and the cached source check invoke verification, so Python 3 is
now a documented Android build requirement. The APK inventory includes the
generator as well as contract/generated source; a generator edit invalidates
previous APK authority, with a provenance regression. This generates enum
admission only, not typed operation arguments/results or the complete shared
schema module. No packaging/Gradle task execution is claimed from static wiring.

## Verification

- python3 tools/test_stage2_android_schema.py: 4 pass; --check succeeds.
- Final python3 -m unittest discover -s tools -p 'test_stage2*.py': 122 pass.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-generated-schema-20261004: exit 0; production/unit/
  instrumented sources compile and 420 JVM tests pass. Existing warnings
  remain. Instrumented tests compile only, not device-executed.
- rtk cargo test --offline --locked --test remote_protocol: 15 pass.
- Lifecycle verifier tests: 21 pass; Blueprint verifier tests: 5 pass.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 rows remain [ ].
- Shared builder edit invalidated host attribution. After the Rust test target
  finished, python3 tools/stage2_host_build.py refreshed debug attribution using
  the real offline locked Cargo build. Final binary hash is unchanged.
- APK verify-only refuses missing provenance. No unchanged blocked Gradle/ADB/
  TCP retry, historical APK installation or physical QR/LAN execution occurred.

```text
generate_stage2_android_schema.py a0a304f1dbab4d44e128d4e860a20a0418db2083fd618d44745fc5b5ca86b548
test_stage2_android_schema.py 55214743bead2723ba630cc63e22ee21ec4d1228a72d0e927adf0c137b861cb3
RemoteSchema.kt 4bfef13e07d3d48a626228e5dd59acd5dc0443144f9151a45a200e419fde0bfa
FrameCodec.kt 4ee560b03df2b529fcbc676ce4e1f693c0046305904810dbab3018180ca73fcc
app/build.gradle.kts eda54f86810017700dce25c653fa9433ed10e91dc827c25cf4ec2f561e5808da
stage2_android_source_check.sh 6a128a544abb19f40fabbef39eb0a46c8fc7bf1030d82bfb468108dc6514ca1b
stage2_android_build.py a8cbfd1deaaeb2724f698bd3be76a86002778644466b22a320e7b19d19403d6f
main.jar 44c9e3474f61043b955ebcf592a8e79d9a265e3a2c1d9935a5aa5bff8bfbed2f
unit.jar ad593b2c443ce631d9b0501fddae2c457bb12d29ae2a8ccf278ca55c9c52b2bf
instrumented.jar a2d7ab9d56a37c3a8506707a1fe769e8173bfcab35e66d130e7baf13dcd6ec75
debug-host-receipt 7bb3f51457cd203e9a47c7b0edb1a0203cbaa452eb71056e7ce99d102c20e7dc
```

Goal remains active. Source-bound APK, physical QR/direct LAN bidirectional
sync/recovery, complete shared schema, full parity/API lifecycle and signed
release remain incomplete or unverified. No full Blueprint item is accepted.
