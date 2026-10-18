# Busy Snapshot Recovery

Provisional S2-032/S2-033/S2-036/S2-038 progress on 2026-10-04. Previous turn
repaired received-byte budgets and verified source gates, so it was progress.
Current foreground retry path, snapshot/omission admission, Busy response and
transient subscription semantics were inspected. No nested agents, worker
admission or authoritative checkbox promotion.

## Repair

A cache marked with omitted assistant tool summaries intentionally needs an
authenticated inspect snapshot rather than tail replay. During Desktop
execution, that inspect can return Busy while metadata/subscription views are
available. Previously Android failed before subscribing, so those restored
phones could not display available live status until execution finished.

TranscriptRestorePolicy now admits a Busy metadata view only from an existing
same-project/session event cursor, for a foreground read with current advertised
subscription support and no mutation. It retains exact messages/cursors and
omission state, applies newly authenticated workspace/subtab metadata and checks
freshness. Both initial resume and required inspect use the same admission
policy and typed HostRefusedException Busy code, not message matching.
No first-attach cursor is invented. Missing/foreign cursors, writes, disabled
subscriptions, unauthorized/owner/refusal/timeout and matching text remain
failures. Background reads keep their prior bounded retry behavior.

The known pending-tail hint is cleared for this subscription opportunity, not
the saved cursor or catch-up state. Subscription resumes from that exact cursor;
transient views cannot advance it. Existing omissions survive any newly replayed
turns, forcing a later successful idle snapshot to restore old invocation
summaries. There is no automatic write retry or host-identity replacement.

Three JVM tests cover metadata fallback and cache omission preservation,
negative admission/freshness and an actual framed Busy-transient/new-turn
subscription followed by a full snapshot restoring summaries at the same final
cursor. They exercise production policies/reader/reducer, not real Android TLS,
Keystore or camera. Initial 409-test compilation predates the third regression;
the final full-source count is recorded below.

## Verification

- Final command: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-busy-cache-snapshot-final-20261004; exit 0.
  Main/unit/instrumented sources compile and 410 JVM tests pass. Existing API
  deprecation and unnecessary-safe-call warnings remain.
- Stage2 Python discovery: 115 passed; lifecycle verifier tests: 21 passed;
  Blueprint verifier tests: 5 passed.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 authoritative rows remain [ ]. Rust production/binary unchanged;
  debug host provenance verify-only passes.
- APK verify-only exits 1: missing current-source APK build provenance. No
  stale APK install, unchanged blocked Gradle/ADB/TCP retry or real camera/
  physical LAN execution was attempted. Requested operator environment/device
  information remains outstanding; no device result is assumed.

```text
HostClient.kt c0e0221eff67e8694348553d65bcadf45e8ecc2160e507355ca137e99d3fc3be
TranscriptBusyRestoreTest.kt 42a050a0bcea5dcd4740d4de8d2d6efec8a137ac54b589911538e5819b03f446
main.jar 68378f48dc78379a0d185718a6080e138cc4f60d117becbc7adea31e800f0202
unit.jar a901a132c98aba8abb4cb8e7cbf1211e50de08fcad9bf5b885e36d5bd5e2245f
instrumented.jar 1309a3d95c1cf8c58645507ebae423e6c66401f2730a546984a39dfb5916e27c
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current-source APK, actual QR/direct physical LAN sync and
recovery, full parity/API lifecycle and signed release remain incomplete or
unverified. This repair does not accept an entire Blueprint item or workflow.
