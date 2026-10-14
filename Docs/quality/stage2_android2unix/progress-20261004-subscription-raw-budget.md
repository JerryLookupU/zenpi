# Subscription Received Byte Budget

Provisional S2-032/S2-033/S2-050/S2-053 progress on 2026-10-04. Previous turn
repaired cache byte-priority and verified source gates, so it was progress.
Current response reader, frame codec, Rust subscription budgets and APK build
restriction records were inspected. No nested agents, worker admission or
authoritative checkbox promotion. Operator environment/device availability was
requested to move the real APK/camera/LAN gates forward.

## Repair

ResponseReader previously accounted normalized JSON reserialization instead
of received body lengths. Large whitespace/escaped spellings could therefore
use substantially more actual input than the intended 196608-byte budget.
The reducer's parsed-data budget cannot independently detect those bytes.

FrameCodec now permits an internal length-consumer callback after prefix
validation and before allocation/body read. Existing callers retain identical
read behavior. ResponseReader uses this length for ordinary replay event/
snapshot page accounting and tracks every subscription frame body, including
terminal control pages across all batches. Aggregate subscription overflow
refuses at the prefix before body allocation/read, outcome validation or page
publication. Prefixes remain outside this JSON-body budget, bounded by existing
page/frame counts. The owner discards the partial connection; no retry occurs
on it and last complete persisted pages remain available.

Three regressions exercise two whitespace-padded subscription pages exceeding
the cap despite tiny normalized data, exact-cap/one-byte-over terminal responses,
and padded ordinary replay events. They check publication counts, preserved
reducer state and unread overflow bodies. Ordinary single control response
compatibility remains. First source run compiled tests after a reader refinement
but used its earlier main artifact; the expected replay refusal assertion
failed. The clean final whole-source run below verifies matching sources.

## Verification

- Final command: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-subscription-raw-budget-final-20261004; exit 0.
  Main/unit/instrumented sources compile; 407 JVM tests pass. Existing API
  deprecation and unnecessary-safe-call warnings remain.
- Stage2 Python discovery: 115 passed; lifecycle verifier tests: 21 passed;
  Blueprint verifier tests: 5 passed.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 rows remain [ ]. Rust production/binary unchanged; debug host
  provenance verify-only passes.
- APK verify-only exits 1: missing current-source build provenance. Earlier
  writable-temp Gradle attempt already failed at daemon ServerSocket EPERM;
  current policy still forbids escalation. No unchanged restricted Gradle/ADB/
  TCP retry, stale APK install or physical scan/LAN execution was attempted.

```text
FrameCodec.kt 7a0c03e2c521941c824403167c2ea1c56b8d6cdf7ad6c1eb1f3d87ea092182ff
ResponseReader.kt d5f9be376623addf3df5e3635b40b4e81ee7468c42dc6ee2d576e1fd97e88db2
ResponseReaderTest.kt 34d59708494a3604bc45e8515c3776647218b805d6b2e69d35d2a0861d1a3022
main.jar 8818a113d75d0f3ff555aa5390ee2c94b718e7625e3374e5fc475bb399e1742a
unit.jar e83f13c8247a1e4a5c76dd3497a7fdf26ad120c6ee2a8babefa4b3099acce0cb
instrumented.jar 1309a3d95c1cf8c58645507ebae423e6c66401f2730a546984a39dfb5916e27c
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current-source APK, real QR/direct physical LAN sync and
recovery, full parity/API lifecycle and signed release remain incomplete or
unverified. Framed JVM stream tests are not connected Android/TLS acceptance.
