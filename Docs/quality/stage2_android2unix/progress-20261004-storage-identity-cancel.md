# Stage 2 Storage Identity Read Cancellation

Recorded 2026-10-04T07:25:59Z. Partial S2-033/S2-037/S2-038/S2-053
evidence, not acceptance. Prior goal turn progressed ACK admission. Canonical
cwd /Users/mac/Github/zenpi; dirty HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b.
Existing changes preserved; no nested agents, reset, push or permission bypass.
All 36 Blueprint rows remain [ ]. Blueprint SHA-256
d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

AuthenticationProof.current had been hardened for network operations, but
foreground storageAdmission and ResumeWorker commit callbacks still checked
cancellation/generation only before a potentially blocking identity read.
Cancellation or foreground replacement during that read could therefore return
a matching device and admit the next non-suspending storage/key-removal stage.

Foreground storageAdmission now uses AuthenticationProof.current with coroutine
and generation checks on both sides of the read. ResumeWorker uses the same
post-read coroutine check before commitResume and inside its storage callback.
No cache schema, CAS, request identity, retry or key-creation behavior changes.
This does not make the filesystem or external key transitions atomic; already
performed host effects remain possible and unknown requests remain preserved.

Added JVM generation-replacement regression for cache/request/key-removal
admission. Extended existing ResumeStoreTest with simulated cancellation inside
identity read at both first storage admission and final pre-startWrite admission:
original cancellation expected, saved bytes/display unchanged, then healthy
commit still succeeds. The prepared test retains its exact named phase and suite
count, and is compiled only, not executed on Android in this turn.

## Verification

Source run 2026-10-04T07:22:01Z through 07:25:44Z exited 0. Renamed a local
ownership helper at 07:22:15Z before main compilation output at 07:22:36Z;
no Kotlin edits thereafter. Current source hashes below. Canonical cwd; OUT
and per-lane ZENPI_HOME isolated under
/private/tmp/stage2-storage-identity-cancel-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-storage-identity-cancel-20261004:
  current main/unit/instrumented compilation, 569 ordinary JVM tests, production
  QR corpus and explicit current Rust/Kotlin lanes pass. Includes ACK/restart,
  actual Busy-owner frame consumption, six-test desktop target and actual Agent
  prompt/tool/running-worker cancellation paths. Instrumented assertions compiled only.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-storage-identity-cancel-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 verifier
  fixture tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  current debug host reattributed, no production Rust source change.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/socket/ADB constraints not retried or bypassed;
  no historical APK installed.

Final log /private/tmp/stage2-storage-identity-cancel-20261004.log.

```text
PairingViewModel.kt 9be65d09dcc74a1158f6a4f5c978a334f630e1700cb21001934173db4379fedf
ResumeWorker.kt ce9914212c23f8eac6196774c76dcbf5fdba09a14cb1b07a7e686655e7930862
AuthenticationProofTest.kt f71f67cba1b449a58ef9b0f6ad005ae033582dc2be7d44b6440549a3b78e82e4
ResumeStoreTest.kt e57be11892e5aadb757308bc5b55d0f141214ee93988c78e88cd62d73118fc8b
source-log cb2d739bd654942b0adad37e17f0fdccc12042b7a787ea42050bcc7125a60942
python-log 7cc78c272e254b88e8461e90a5a0a7582458b29ab128980e1aa31524848c4c8a
main.jar e41ca10dea643b0037fcb88e9b17460e0a4d1f9a1c18d397c3931319109370fc
unit.jar 1b04fa5787c6cf62a294c31828dacfab117e249d3de900506a9742a356b033e9
instrumented.jar 7d5296e147572595b9301eff81d6a857db7eb1ee2ee0e4e1faf063576505c80a
host-receipt 749cc79bf0c3232c34b74d56203b44993972556db490214cec9157a26c0f6a9e
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required product gates.
