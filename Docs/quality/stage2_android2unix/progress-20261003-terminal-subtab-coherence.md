# Terminal Projection Matches Selected Subtab

Provisional S2-032/S2-033/S2-036/S2-054 progress on 2026-10-03. Previous turn
ran broader Rust parity/encrypted subscription targets and corrected evidence
scope, so it was progress. Current codecs, transport call sites and tests were
inspected before editing. No nested agents, checkbox promotion, commit/push or
unrelated cleanup.

## Repair

The terminal decoder checked project/session, but selected-subtab decoding did
not cross-check terminal active ID/revision. UI and command builders already
hide/refuse many mismatches; however, contradictory listing/subtab state could
still reach the projection reducer/cache publication boundary.

SubtabProjectionCodec.decode now refuses a terminal with foreign project or
session, a different active tab, or a different subtab revision. This shared
entry covers initial HostClient listing and subscription views. A legitimate
desktop selection race between separate reads now fails closed and refreshes,
rather than composing views from different revisions. No old/new output is
silently retagged and no capability is created.

Two new JVM tests cover all scope fields, earlier/later revisions, valid and
absent terminal cases, plus atomic subscription rejection preserving previous
view/cursor/completion. The existing workspace/subtab rollback tests now keep
terminal and tab revision consistent so they still prove monotonicity refusal.

## Verification

- Stage2 Python discovery: 107 passed; lifecycle verifier: 21 passed.
- Physical verifier six, instrumentation/Blueprint tests five each pass.
- Rust debug/release current-source provenance, formatting, whitespace and
  strict Blueprint pass; Rust unchanged and all 36 rows remain unfinished.
- APK attribution still rejects missing provenance. No stale APK install,
  unchanged blocked Gradle/ADB retry, camera or physical LAN pass is claimed.
- Initial source run compiled but failed one old stale-tab fixture because it
  changed only the tab revision. The terminal revision in that fixture and its
  newer control were corrected; production coherence validation was retained.
- Final source gate /private/tmp/stage2-terminal-subtab-coherence-final-20261003
  exits zero: main/unit/instrumented sources compile and 385 JVM tests pass.
  Device source compilation is not current APK/device acceptance.

```text
SubtabProjection.kt 9f2660bd3c04c1770f0c120efcfac6d416f5d75214dd23ac19260ccfb5018d48
SubtabProjectionTest.kt dbc98719b3e596c3433988fad403c1c50335372f634996a51bfbd34b8af8f596
SubscriptionTest.kt d184d22a5dde414274422b9185128907b27b3c11f2dc68754af8c3f4df557681
main.jar b13cd1f0fbd5ea97bf87bfcc47020fa4eec3628e625cf5b1f7fac224a80fe9d3
unit.jar d6e97fcb83b5e200852163830fcb7fe9a9ca8cfe88947267c5098edb718ea188
instrumented.jar de9d431d793e95eca7d393d0a145acb521a0769ac72b814ca69ce11fb0ca981a
```

Goal stays active. Current APK/device, actual camera/direct LAN desktop sync,
complete parity, supported API lifecycle and signed release remain unaccepted.
