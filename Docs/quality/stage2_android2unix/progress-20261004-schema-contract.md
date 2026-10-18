# Shared Protocol Enum Contract

Provisional S2-003/S2-030/S2-050 progress on 2026-10-04. The previous goal turn
repaired authentication refusal authority handling, so it was progress. Current
Blueprint, dirty worktree, Rust protocol enums/codec, Android codec, shared raw
wire corpus and Gradle resource wiring were inspected. No nested agents, worker
admission or authoritative checkbox promotion.

## Change

The raw-wire corpus covered typical envelopes and malformed traffic but did
not enumerate every operation and its read/write ownership rule. A shared
schema-contract.json now lists the 33 current operations (10 read, 23 write),
six execution states and twelve error codes. Rust and Android read the same
fixture and construct a production-codec matrix, refusing duplicate fixture
names and unexpected enum values.

For each operation the tests check missing owner/revision, owner-only and
revision-only combinations, acceptance with maximum u64 revision, and refusal
on event/bulk channels. Rust also checks Operation.is_write against the shared
classification. All outcome states are checked; failed requires a code, each
known error code is accepted structurally and unknown operation/state/code is
refused. Existing raw-wire and 2048 seeded mutation tests remain separate.

This contract is a test fixture, not a generated production shared-schema
module. It does not validate operation arguments, ACL grants, CAS behavior or
network authentication, and it does not replace physical QR/LAN acceptance.
No wire version or production codec behavior changes.

## Verification

- Final rtk cargo test --offline --locked --test remote_protocol: 15 pass.
- Final zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-schema-contract-final-20261004: exit 0; production/unit/
  instrumented sources compile and 420 JVM tests pass. Existing warnings
  remain; instrumented tests compile only, not device-executed.
- Stage2 Python discovery: 118 pass; lifecycle verifier tests: 21 pass;
  Blueprint verifier tests: 5 pass.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 rows remain [ ].
- Rust test execution transiently invalidated the binary hash check during
  Cargo activity. A completed python3 tools/stage2_host_build.py refreshed
  current debug attribution; Rust source inventory and final binary hash are
  unchanged. No network/device acceptance is implied by this build.
- APK verify-only refuses missing current-source provenance. No unchanged
  restricted Gradle/ADB/TCP retry, historical APK install or physical QR/LAN
  execution occurred.

```text
schema-contract.json 9d86bfcaa63edf12d437a159b4672acd631d6398a174d0f228f0a065a50f2bba
remote_protocol.rs 758f7252e7439282fc172fb8e9697bfef5436e109552c69fbfd704619e9a49e9
RemoteProtocolCorpusTest.kt 032259f0e2928ef31da9b2448b5c72558f5a35280c2c7d8c8571cd1e98e2311d
debug-host-receipt 6a9c9d6b32d44fc77a4cdd9362e1abb42909e2d888dccdeb8b86c07b654886f1
debug-host-inputs c55bacffb031a8e324b93971274ebb56615d4f2978c135ab74ec45451d2dd49f
debug-host-binary 57b52b80beda551642c47cd1f404f2fd1400b7a16699f91401402cb77db227a8
main.jar 144db61df9a2677c04658e5d6c16f8637a8a151673131ad90e1a8a2e3df66a6d
unit.jar f3901bfe647729e893bc9088f4bd50182608d0fd7e6d697ac48f04f6a429ef09
instrumented.jar a2d7ab9d56a37c3a8506707a1fe769e8173bfcab35e66d130e7baf13dcd6ec75
```

Goal remains active. Source-bound APK, physical QR/direct LAN communication and
recovery, full parity/API lifecycle and signed release remain incomplete or
unverified. No entire Blueprint item acceptance is claimed.
