# Oversized Legacy Turn Display

Provisional S2-031/S2-034/S2-037/S2-051 progress on 2026-10-04. Previous turn
extended the physical bidirectional workflow and verified compilation, so it
was progress. Current Rust projection bounds, Android snapshot/replay/display
budget and full-message request sources were inspected. No nested agents,
worker admission, checklist promotion or unrelated cleanup.

## Repair

Android deliberately accepts legacy individual content up to 96 KiB, but its
entire encoded display budget is also 96 KiB. A newest turn at that content
limit, or smaller content expanded by JSON escaping, exceeded the display
budget. displayTail broke before retaining any turn while replay advanced the
cursor. The message disappeared together with its full-message paging action.

The shared snapshot/replay tail reducer now keeps that newest message's identity,
role and tool summaries, reducing its content to a scalar-safe prefix that fits
the actual JSON-encoded budget. A bounded binary search accounts for escaped
characters and the explicit truncation flag. Snapshot/replay truncation status
reflects the retained partial turn. Existing normal newest-tail eviction and
64-turn limits are unchanged; host journal and protocol frame limits are not
relaxed. Current 12 KiB host projections normally do not require this fallback.

The regression checks 96 KiB ASCII, heavily escaped content and supplementary
Unicode. Snapshot/replay retain identical partial messages, cache roundtrip
preserves ID/flag/cursor, and the resulting display can construct the existing
authenticated full-turn page request. No surrogate half is retained. This is
source/JVM evidence, not actual connected full-page UI execution.

## Verification

- Final source check /private/tmp/stage2-legacy-turn-display-final-20261004:
  exit 0, main/unit/instrumented sources compile and 390 JVM tests pass.
  Existing deprecation warnings remain. Initial check also passed, then the
  final check included snapshot/full-page-request assertions.
- Stage2 Python discovery: 110 passed; lifecycle verifier: 21 passed;
  Blueprint verifier tests: 5 passed.
- Strict Blueprint, git diff --check and rtk cargo fmt --all -- --check pass.
  All 36 rows remain [ ]. Rust production code is unchanged and debug host
  attribution verify-only passes.
- APK verify-only: exit 1, missing current-source provenance. No stale APK
  installation, unchanged blocked Gradle/ADB/TCP retry, camera scan or LAN run.

```text
HostClient.kt e84751ae25d85bfd187c8ebc74d06b7aafbd042d845b5fbda180d899a7394c77
ReplayTest.kt 479f3c87e2bab7ddd87828fd61cd669ae12dfc0fba18ff6e87e0052a68ad0cb1
main.jar 4156f35ca8e0ff8706affffab17f1397b883ca2d04667d1bce6fed6eda91d1b8
unit.jar cda2144e5d4df0f61eb2864f261079b7f093f04d6d50c36652538b7400470d79
instrumented.jar 566b5a93e89392f810324d7595271826d36ab4a2b16ce045c499583653ab4334
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current APK, real camera QR/direct physical LAN
bidirectional sync, full desktop parity, API lifecycle and signed release
remain incomplete or unverified. This repair does not accept an entire item.
