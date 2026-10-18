# Stage 2 Retained Subtab Metadata Integrity

Recorded 2026-10-04T06:21:46Z. Partial S2-035/S2-036/S2-054 evidence, not
acceptance. Prior goal turn progressed terminal metadata. Canonical cwd
/Users/mac/Github/zenpi; dirty HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b.
Existing changes preserved; no nested agents, reset, push or permission bypass.
All 36 Blueprint rows remain [ ]. Blueprint SHA-256
d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

SubtabProjectionCodec.current checked project/session, capacity, unique IDs and
active presence but not retained revision or each tab's metadata. An inactive
tab with malformed name/kind/ID/quota could therefore leave fresh scoped
commands and reads admitted even though its equivalent wire view was invalid.

Current resolution and decoded display results now share a shape predicate:
canonical u64 revision and 64-hex IDs, 1..32 unique tabs with active presence,
supported kinds with main exactly first, nonblank control-free names up to
128 UTF-8 bytes and concurrency in 1..64. Existing construction, dispatch,
worker-send, file/diff/terminal reads and Compose controls reuse current.
Diff JVM/Compose fixtures now use real canonical IDs, not "tab"/"inactive".
No transport change, new capability or Android execution introduced.

Expected/actual: malformed active or inactive metadata refuses tab selection,
directory/worktree creation, quota changes, worker submission, file/diff/PTY
reads and fresh dispatch. Saved pending commands remain exact-retry eligible;
healthy read-only tabs and boundary values pass. Current real Rust/Git snapshot
lane repeats revision, quota and name refusals before emitting valid commands.
Existing Compose case hides indexed controls and disables secondary selection
for malformed retained metadata. Compose is compiled, not device-executed.

## Verification

Final source run 2026-10-04T06:18:11Z through 06:21:38Z, canonical cwd, exit 0.
No production/test edits after launch. OUT and per-lane ZENPI_HOME isolated
under /private/tmp/stage2-subtab-metadata-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-subtab-metadata-20261004:
  main/unit/instrumented compilation, 551 ordinary JVM tests, production QR
  corpus and all explicit Kotlin/Rust lanes pass. Includes actual Git worktree
  CRUD/quota, PTY effects/history, six-test desktop target, actual Agent prompt/
  tool and running-worker cancellation paths.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-subtab-metadata-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, 5 pass.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo
  lanes; debug binary reattributed, no Rust source changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/local-socket/ADB blockers not retried or
  bypassed; historical APKs not installed.

Final log /private/tmp/stage2-subtab-metadata-20261004.log.
Git exchanges /private/tmp/stage2-subtab-metadata-20261004/android-worktree-results.json.

```text
SubtabProjection.kt 9fdfb9eb3afbc99abbcdb672e18372db6dba1b8fee565c0200ac9cf8c6a1d50c
SubtabProjectionTest.kt 9b2e237d7cdb1a9a299ea921eb74a1f42dbf3b5f8a72a8fc7ceb72163449a263
AndroidRustWorktreeTest.kt ced113e5c5daa5d1c17caeab4216c564c4a15dc4f0fee15ae3e51500501e323a
SubtabScreenTest.kt fe4186c384612433f50e28a1e75b5474a73b1d366458c7c18ead35d53d8fff6f
DiffProjectionTest.kt 70965602627124860c5b7a2fb20b75cad5b227fbb0df2434cba54a72738b3530
DiffReadPolicyTest.kt 4ca77bc2aea88be8967c4cf95b97ede3671de7ea1d6d706060efbd0d0b5e209c
DiffScreenTest.kt 6f1261cc5364c05bd724714993dd65cd048694295f3579f94f469ba8b694e629
source-log b4c4c5847260e75fea4af62e9baf7c48e981c35a384b57dd35322e775c5dbde6
worktree-results d3afd33f6e7a8a988eba0fd6706ca7b2dc7cf6836b8fbcba9a6026ede0b90da2
python-log 3b5104525fc99e557d172ab905b6d63b2cd3ee152cb9632d84b19c6cc8110c08
main.jar 6ebb91a6c81dad1dbaff443c7a4cafbbb1846f02052012836d33c53e7b630a38
unit.jar 2ac2085b5785f28cade730b185e53075db9ee2aaab871b5a2dbbd8aa859ae7a4
instrumented.jar 08da9edfe761f1b880a2c1153754714551b58dd33a2a13064a231861f56f4a3d
host-receipt bc331c58ffdd9089e494096629913b76e472ca99596ad7cdeb909b2a1884cd74
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35 lifecycle, complete desktop parity and
signed reproducible release remain required product gates.
