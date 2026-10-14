# Combined Encrypted Cache Budget

Provisional S2-033/S2-038/S2-056 progress on 2026-10-04. Previous turn extended
physical foreground replay preparation and verified source gates, so it was
progress. Current replay/snapshot reducers, wire complexity caps, prompt status
listing and encrypted store write/read paths were inspected. No nested agents,
worker admission or authoritative checkbox promotion.

## Repair

Workspace listing and transcript are fetched in different envelopes. Each can
fit FrameCodec's 4096 payload-node limit, yet their combined encrypted cache
record can exceed it. The previous store writer checked bytes but not the
restore envelope: it could replace good ciphertext with a record that failed
the next cold read. Plain toByteArray could also silently replace an unpaired
Unicode surrogate, changing the saved display rather than refusing it.

Offline v4 prompt status history now has a 256-record combined cap, prioritizing
the selected project's statuses before other projects in stable order. Live
listing/controls, all workspace identities/order, transcript display and its
cursors are unchanged. Exact pending write/revoke identities are separate and
are not trimmed. Encoding the restored bounded record is stable.

FrameCodec exposes its existing strict UTF-8 encode/validate operation; network
write delegates to it without changing framing. PairedHostStore reuses it for
the complete plaintext before key creation, encryption or AtomicFile writes,
then checks its local byte limit. Invalid scalars/complexity leave prior
ciphertext and pending identity untouched rather than creating unreadable
metadata. This is not a promise that every independently maximum-size view fits
the local byte cap; excess bytes still fail closed.

New JVM coverage constructs 64 projects with nine typed failed statuses each
and 64 transcript turns. The listing alone passes actual FrameCodec, but the
old combined record fails its node cap. The bounded combined record passes,
retains the selected project's full status list, project order, all turns and
cursor, and preserves the live input. Initial eight-status fixture did not
cross the node boundary; its failed assertion led to the corrected nine-status
counterexample. Existing malformed Unicode write test also checks shared encode.
ResumeStoreTest extends its existing prepared device case with capacity restore
and invalid-scalar rejection preserving exact old ciphertext. It is compiled,
not device-executed evidence.

## Verification

- Final command: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-cache-record-budget-verified-20261004; exit 0.
  Main/unit/instrumented sources compile; 403 JVM tests pass. Existing API
  deprecation and unnecessary-safe-call warnings remain. Earlier 403-test pass
  predates shared strict encoding extraction and is not the final main artifact.
- Stage2 Python discovery: 115 passed; lifecycle verifier tests: 21 passed;
  Blueprint verifier tests: 5 passed.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 rows remain [ ]. Rust production source/binary are unchanged;
  debug host provenance verify-only passes.
- APK verify-only exits 1: missing current-source APK build provenance.
  No stale APK install, unchanged blocked Gradle/ADB/TCP retry, real camera
  scan or direct physical LAN test was attempted.

```text
ProjectionCacheCodec.kt 4dfeac3abf182fa71e8665c615aded8d145622603d59e39ab6fd20f7c7bdd58d
PairedHostStore.kt 89a9d82fb69698e5a982c6a0830fdf6ac93e8204305fa15f439deff009432c95
FrameCodec.kt 14a386b22ec8617768b1e61f39e18db7ca610d38fac07fedff90ad0b214e1252
ProjectionCacheTest.kt ba5154a29587d554ebb2d46ea51dba10ccdc9d5e16a820ff650b62879a8734f2
FrameCodecTest.kt db23c6d6f4a5c0a35b5e5e39ca8b35fbad2b7f08337a7815b7dc63764da69afe
ResumeStoreTest.kt 35f745df2c75c19a9d74ea5e83fb8a4968f75b493dfe62329872143669ce166d
main.jar a0017e468eda21caac68e2b2ddda3e2e67ed10eee05ef00256bb7bf0153c752d
unit.jar 8a2e909eede406c59fcc930359d84113785e575dffd92728e5774ce6f7ac6ee1
instrumented.jar 97d7a3bd83f1df737cff7db9d1e147d7f72aa475bfc7fd30a5e74bd8148f6077
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current-source APK, actual QR/direct physical LAN sync and
recovery, full parity/API lifecycle and signed release remain incomplete or
unverified. No whole Blueprint item or product workflow acceptance is claimed.
