# Stage 2 History Continuation And Tool Summary Integrity

Recorded 2026-10-04T06:27:06Z. Partial S2-036/S2-054 evidence, not acceptance.
Prior goal turn progressed subtab metadata. Canonical cwd /Users/mac/Github/zenpi;
dirty HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved;
no nested agents, reset, push or permission bypass. All 36 Blueprint rows remain
[ ]. Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

Worker history request construction checked scope/has-more but not retained
page shape, while direct decoding did not call request admission. Navigation
had a stronger page check, but neither worker nor conversation retained history
revalidated tool-call summaries. These inconsistent predicates could admit
malformed continuation state or historical tool-argument selection.

WorkerHistoryCodec.validPage now supplies request/decode/navigation validation:
digest, bounded offsets/counts, nonempty turns, unique IDs, roles/content and
tool summaries. Decode invokes request admission, rejects adjacent repeated
turn IDs and validates the result before publication. WorkerHistoryNavigation
reuses this predicate instead of maintaining another copy. SessionHistoryCodec
also validates retained tool summaries and decoded results. Tool summary checks
cover assistant-only ownership, at-most-eight unique bounded IDs/names/previews,
NUL refusal and truncated-count agreement. No tool execution is granted.

Expected/actual: mutated retained digest/offset/count/content or tool summaries
cannot authorize worker request/decode/navigation. Conversation history rejects
the same malformed summaries before historical message projection. Valid
eight-call truncated summaries and ordinary tool-loop pages remain supported.
Current real Rust tool-loop history is mutated in Kotlin to confirm refusal;
Compose tool-argument controls disable invalid retained summaries. The existing
next-page unit fixture now has a distinct turn ID rather than repeating input.
Compose is source-compiled only, not device execution.

## Verification

Source run 2026-10-04T06:24:11Z through 06:26:56Z, canonical cwd, exit 0.
No production/test edits after launch. OUT and per-lane ZENPI_HOME isolated
under /private/tmp/stage2-history-tool-metadata-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-history-tool-metadata-20261004:
  main/unit/instrumented compilation, 553 ordinary JVM tests, production QR
  corpus and all explicit current Kotlin/Rust lanes pass. Includes real
  multi-page worker tool history/arguments, six-test desktop target and actual
  Agent prompt/tool/running-worker cancellation paths.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-history-tool-metadata-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, 5 pass.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  debug host reattributed, no Rust source changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Unchanged Gradle/local-socket/ADB blockers not retried or bypassed;
  historical APKs not installed.

Final log /private/tmp/stage2-history-tool-metadata-20261004.log.
Tool exchanges /private/tmp/stage2-history-tool-metadata-20261004/android-worker-tool-results.json.

```text
ToolCallProjectionCodec.kt 83c72f1c45bff1a3bbd036683f901f4064ac7e436ba0d9559f90be1be647c37c
WorkerHistoryCodec.kt 46089f48850a6a1be0ff1171bee10e026959cfb37f823753652908b40787a216
WorkerHistoryNavigation.kt 68a257d0c0a3a6466292e5aa6ff6dbadab1b45adce713dd33277fb9c0995511b
SessionHistoryCodec.kt 4615c5d20c7df295cdd9c6fcad48c3e1dcd18ddd95e2c3ee93073ddaf01c9184
WorkerHistoryTest.kt cdf42ed83b0f976722cce40e2904b93a9f07d185ab37db48adabc01a17ef9c29
SessionHistoryTest.kt aba0ebf69a5974202810a3c6befd2a0663ec0893e9834b163cc9c03226417712
AndroidRustWorkerToolTest.kt 2495b87e8dde864dcdc20b1cd3a20122266b9ece2c7d4a344417790d84f32ecf
WorkerScreenTest.kt cc1a06c67b9a567527ff5ef018ae9e7fbb7b232dff9597f112b9c610e027aed3
source-log 0f3b5eac0705505d4c9edcf75b87d71a7ad1d80a19007ff06da93f764ac7016b
tool-results 0388bbe059f86ad6fcb749da6a2c3503c677bfa9f7bde1b1aec20349fbad5ec1
python-log 5e5210a54b140f70203f0863c8af9afea945071b65a6db8e3180f6a7ebc30ba2
main.jar 1d266e647a138bb688f890973112aa437cf749035511dee942f6d45a844fa4c7
unit.jar 4688e77802dae4401888ef9eef45f822c1e963a6ccd72f72293f01d1ab4a022f
instrumented.jar 67cc4c5c2ff4500a7f3a73f2b9c7ac014246037806fcbffd8b9c16fd7e076317
host-receipt 02a67489aa2fc715cbbd0dbf47219f925643d42ca62f4fc5e20ffb49e8eb7c59
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35 lifecycle, complete desktop parity and
signed reproducible release remain required product gates.
