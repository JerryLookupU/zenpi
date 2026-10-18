# Read Response Outcome Consistency

Provisional S2-013/S2-031/S2-034/S2-051 progress on 2026-10-03. Previous
turn repaired subtab receipt target semantics and completed its source/host
tests, so it was progress. This turn inspected projection commit, encrypted
resume CAS, subscription reducer, prompt status, frame/prelude and production
response-reader sources. No worker admission, nested agents, checkbox promotion
or unrelated cleanup.

## Repair

FrameCodec recognizes typed error codes and requires one for failed responses,
but does not prohibit an error code on a non-failed response. Pairing and write
receipts already enforce that semantic consistency. ResponseReader previously
passed a succeeded read/subscription response with a non-null error code to
HostClient's success handling, potentially allowing its page to commit/publish.

The shared reader now refuses that contradiction before outcome callbacks or
subscription consumers. This covers ordinary reads and bounded replay using
the same actual length-prefixed reader. Absent/explicit null code remains
compatible, and failed typed responses still reach existing refusal/session
change handling. Rust wire schema and production source are unchanged; no
unverified schema compatibility claim is made.

Two JVM regressions exercise three-byte-fragmented succeeded responses carrying
busy, unauthorized, revoked_device, session_changed and internal_error codes
for resume and subscribe. Neither validation nor publication runs for the bad
page. An explicit null code remains readable. A bad later subscription page
retains the already committed prior turn/cursor and never marks the subscription
done. These are parser/consumer tests, not SSLSocket/Keystore/physical LAN tests.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-response-outcome-consistency-20261003: exit 0. Main/unit/
  device sources compile; 389 JVM tests pass. Existing deprecation warnings remain.
- Stage2 Python discovery: 110 passed; lifecycle verifier: 21 passed;
  Blueprint verifier tests: 5 passed.
- Strict Blueprint, git diff --check and rtk cargo fmt --all -- --check pass.
  All 36 authoritative rows remain [ ].
- Debug Rust build verify-only passes. No Rust test/build rerun was needed for
  this Android-only repair; current attribution remains unchanged.
- APK verify-only: exit 1, missing current-source provenance. No stale APK
  install, unchanged blocked Gradle/ADB/TCP retry or physical QR/LAN run.

```text
ResponseReader.kt e190b17a71d7dba234c4feaa0a78dd5190e9d23bd4d85f29b5b0590d6c97f176
ResponseReaderTest.kt 87b4e55c1c67e5033e17aa47efcb80c443c6cd4225144508ed754d7d1311a812
main.jar 43c3b51e71a7ec0b4f2cda155802b874bfe69fba0cbe58abc2284cf400ccaa82
unit.jar 4e9bb778b316e91f6a88101fc7edb64f563029b4651d4a827054327a06500feb
instrumented.jar 9c01be541d722e853a403fed9a45163d9a8b57855425f856a510334d2142f2c5
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current-source APK, real camera QR/direct physical LAN
roundtrip, full parity, connected API lifecycle and signed release are still
incomplete or unverified. This repair does not establish whole-item acceptance.
