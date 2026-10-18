# Transcript Continuation Admission

Provisional S2-031/S2-034/S2-037/S2-051 progress on 2026-10-04. Previous turn
repaired oversized legacy display retention and passed its source gates, so it
was progress. This turn inspected current page request/decode, ViewModel page
publication/navigation and worker read selection sources. No nested agents,
worker admission, checkbox promotion or unrelated cleanup.

## Repair

TranscriptPageCodec.request checked project/session/target/has-more but did not
validate the entire prior page or bind a worker continuation's subtab to its
current succeeded status. ViewModel navigation covered part of this, but a
direct transport caller could construct a request from a malformed or foreign
prior page and only fail after network I/O. Decode also accepted a prior page
without reusing the request admission checks.

Continuation requests now use the existing bounded TranscriptNavigation page
validator for digest, offset/text/total/has-more consistency. They require the
correct role; worker continuations require the current succeeded task's subtab,
and ordinary continuations reject worker subtab metadata. Decode applies the
same request admission before accepting incoming page data. No request fields,
wire limit, mutation authority or journal/cache cursor behavior is changed.

Two regression methods cover malformed digest/byte offsets/text/role and
foreign/null worker subtab identity in request and decode paths. Existing valid
ordinary, worker, history-turn and tool-argument paging tests pass. Tests do not
instantiate Android sockets/Keystore or establish a connected resume result.

## Verification

- Source check /private/tmp/stage2-transcript-continuation-20261004: exit 0,
  main/unit/instrumented sources compile and 392 JVM tests pass. Existing
  deprecation warnings remain; device sources are compiled, not executed.
- Stage2 Python discovery: 110 passed; lifecycle verifier: 21 passed;
  Blueprint verifier tests: 5 passed.
- Strict Blueprint, git diff --check and rtk cargo fmt --all -- --check pass.
  All 36 authoritative rows remain [ ]. Rust code/binary are unchanged;
  debug host attribution verify-only passes.
- APK verify-only: exit 1, missing current-source provenance. No stale APK
  install, unchanged blocked Gradle/ADB/TCP retry, physical QR or LAN execution.

```text
TranscriptPageCodec.kt db505ca843371baaeb020f90b323a0bcaa0da70ac09ef707ff1f2c6a0c34d308
TranscriptPageTest.kt 26f90e6762fa97ddda7eb802b794d0f6931407b6d04743d1daf189c26fe0d380
WorkerTranscriptTest.kt 07cd7296d1469dd7d7468afe1f4d94e3392f33e42392175a1393e9988ae22b31
main.jar f5c168111bfa4e7a1fea88a72d845e1b6d37f313bb01de17e61ede218ddff491
unit.jar d926548a7f0758140b1338d98b4afd5246620af28fa2e67a7840463e9b9c1db4
instrumented.jar 566b5a93e89392f810324d7595271826d36ab4a2b16ce045c499583653ab4334
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current APK, actual physical QR/direct LAN bidirectional
sync, full parity, supported API lifecycle and signed release remain incomplete
or unverified. This repair is not whole-item acceptance.
