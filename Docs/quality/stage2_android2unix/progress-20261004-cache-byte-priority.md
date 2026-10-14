# Cache Byte Priority

Provisional S2-033/S2-038/S2-056 progress on 2026-10-04. Previous turn repaired
combined cache complexity and strict encrypted writes, so it was progress.
Current subscription/replay, cached prompt status, encoded display and store
boundaries were inspected. No nested agents, worker admission or authoritative
checkbox promotion.

## Repair

A 256-status node budget alone does not bound JSON bytes. Long request IDs
combined with a nearly full 96 KiB transcript could exceed the 128 KiB cache
cap even though live listing and transcript responses were individually valid.
This prevented authenticated sync results from being durably committed.

Cache encoding now enforces actual UTF-8 JSON bytes after the existing count
budget. It removes statuses from the lowest-priority project's tail first,
retaining current-project priority and per-project order. Message display,
workspace identities/order, session, journal/event cursors and exact pending
operation identities are untouched. It accounts removed JSON values and their
array comma bytes directly, avoiding repeated full-record serialization for
each removal. A record that remains oversized without statuses still fails
closed rather than losing messages while saving their advanced cursor.

One new JVM regression combines 16 projects with 16 long-ID statuses each and
eight nearly 12 KiB transcript turns. The result fits cache bytes, retains all
messages/cursors and the selected project's status history, drops lower-priority
history, preserves live input, and remains stable on decode/re-encode. The same
status history without the large transcript fits normally. Existing count/node
capacity regression remains. ResumeStoreTest extends its prepared encrypted
capacity case to prove the resulting record can be reloaded by a new store;
this device case is compiled, not executed.

## Verification

- Final command: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-cache-byte-priority-final-20261004; exit 0.
  Main/unit/instrumented sources compile; 404 JVM tests pass. Existing API
  deprecation and unnecessary-safe-call warnings remain. Earlier pass predates
  direct byte accounting refinement and is not the final main artifact.
- Stage2 Python discovery: 115 passed; lifecycle verifier tests: 21 passed;
  Blueprint verifier tests: 5 passed.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 rows remain [ ]. Rust production/binary unchanged; debug host build
  provenance verify-only passes.
- APK verify-only exits 1: missing current-source APK build provenance. No
  stale APK install, unchanged blocked Gradle/ADB/TCP retry, real camera scan
  or direct physical LAN execution was attempted.

```text
ProjectionCacheCodec.kt 60aa8fc555b2a0450265fd6e782946a44a02f87736ff1c4e94c2921925c400b2
ProjectionCacheTest.kt 48eb5d0bf63a9d1211c66502e12e223549bd0d8ea732f181e94fb64910fc5443
ResumeStoreTest.kt 977b2bf1534dd0811c2b7bf42a03c874d8d33321f6b8831db256137096255f4e
main.jar 6b5b568bee4d9e4082939bacc799f802d16640e9dbab024bcd5694e776097a9a
unit.jar 0bab27f9a4fd203c75d5e0821dffe7e60d8ba8be375a1c44bdc3419eb3982d86
instrumented.jar 1309a3d95c1cf8c58645507ebae423e6c66401f2730a546984a39dfb5916e27c
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current-source APK, actual camera QR/direct physical LAN
bidirectional sync/recovery, full parity/API lifecycle and signed release remain
incomplete or unverified. No full Blueprint item or workflow acceptance is
claimed from this narrow capacity repair.
