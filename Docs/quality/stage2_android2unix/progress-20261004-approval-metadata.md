# Stage 2 Retained Approval Metadata And Preview Integrity

Recorded 2026-10-04T06:44:38Z. Partial S2-036/S2-054 evidence, not acceptance.
Prior goal turn progressed model status integrity. Canonical cwd /Users/mac/Github/zenpi;
dirty HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved;
no nested agents, reset, push or permission bypass. All 36 Blueprint rows remain
[ ]. Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

Approval current admission checked scope/capability and target equality, but
not retained metadata/preview shape. A malformed retained target could therefore
enable confirmation or command creation despite failing equivalent wire decode.
An unrelated malformed/oversized pending list also left a valid target admitted.

ApprovalProjectionCodec now shares retained validation between decode and
current: bounded control-free IDs/tool, known effect/origin, canonical digest,
paired worker/subtab identity, bounded control-free preview path/patch and exact
withheld/redacted consistency. Current additionally validates all at-most-16
pending entries, project/session scope and unique worker/request identities.
Existing confirmation policy, construction and fresh dispatch reuse current.
No additional authority, automatic decision or tool execution introduced.

Expected/actual: mutated metadata, partial worker scope and invalid preview
refuse fresh Allow/Deny construction, confirmation and dispatch. Malformed
other targets and oversized lists refuse current admission. Exact saved pending
envelopes remain retryable for receipt resolution; valid withheld previews and
distinct workers with the same request ID pass. Actual Rust approval metadata
is mutated in Kotlin and refused. Existing Compose confirmation case disables
invalid targets; compiled only, not connected device execution.

## Verification

Source run 2026-10-04T06:41:12Z through 06:44:27Z, canonical cwd, exit 0.
No production/test edits after launch. OUT and per-lane ZENPI_HOME isolated
under /private/tmp/stage2-approval-metadata-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-approval-metadata-20261004:
  main/unit/instrumented compilation, 556 ordinary JVM tests, production QR
  corpus and all explicit Kotlin/Rust lanes pass. Includes actual approval
  coordinator receipts, six-test desktop target and real Agent approval/tool
  effects, prompt/steering and running-worker cancellation paths.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-approval-metadata-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, 5 pass.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  current debug host reattributed, no Rust source changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Unchanged Gradle/local-socket/ADB blockers not retried or bypassed;
  historical APKs not installed.

Final log /private/tmp/stage2-approval-metadata-20261004.log.
The explicit Rust/Kotlin approval lane validates fixture-local exchanges; the
final log records test outcomes, not raw exchanges. No separate durable
approval-results artifact is claimed.

```text
ApprovalProjection.kt 4cd9b6133e0414ebc870750bed8ba8da82ffea0f7e04bb4d442431f4a491f303
ApprovalDecisionPolicyTest.kt 4ddecc9d6fdba533a7cb2a236bb7f9389e00a1aa34c46120980edca021f09257
AndroidRustApprovalTest.kt 3dee9185f4cfad95547b6fcd51a27c735bc5c3b8a175b4599e189fc9a2c01590
ApprovalScreenTest.kt 7468aad82585015adfe87316dca748032ddebd4212ec841bd1ef9400d5cba56c
source-log a1d76d342dcc2a2aeb8cc64a5c752edcea4373cd4037d25ccc03132f464c04f1
python-log ee6a4b0f8f249d798b0334d7cd2fd7f1b452ac3c3d41c72159032aec5131b3a1
main.jar 2da16d123c9f4b173194e35c98aaaae2c29e736b0e84333bf188b94ea7e4163b
unit.jar 31f5704ff6bfcf66bd6f52f2c65f6e774afa0a62968426aa8548b9c986be5a5d
instrumented.jar 2d23ab49f0d6f3119aaca7789b1e03e01d197c8ca913b96d97d070521f90cbf7
host-receipt 313a03ce2575a4f0b4f80e8b119041a8c32cbe7bca922d8f75af8eb75efc7238
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35 lifecycle, complete desktop parity and
signed reproducible release remain required product gates.
