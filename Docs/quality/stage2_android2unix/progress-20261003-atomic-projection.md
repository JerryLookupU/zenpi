# Atomic Android Projection Publication

Provisional S2-032/S2-033/S2-036/S2-052/S2-054 progress at
2026-10-03T19:44:52+0800. Previous goal turn made release-manifest and offline
authority progress. This turn inspected current production transport and found
an early partial-publication path on foreground reads. No checklist promotion,
nested agents, worker launch, commit/push or unrelated cleanup.

## Repair

HostClient previously published workspace projects through onPromptStatuses
immediately after listing, before subtab/snapshot/replay validation. The
ViewModel copied those projects into its previous projection. A later failure
could leave new session/capability/status data attached to an old transcript,
cursor and owner/revision. The partial callback is removed; publication now
uses only the existing complete projection callbacks or final verified result.
Accepted writes still resolve through their separately validated receipt, not
through a listing or transcript observation.

HostProjectionCodec.workspace previously returned its input unchanged if a
fresh listing omitted both owner and revision. It now explicitly clears both
authority fields. A one-field omission continues to fail closed. Compatibility
with read-only metadata remains without retaining control from an earlier host
response.

ProjectionCommitPolicy now refuses regressing workspace revisions within the
same owner/project/session independently of journal/subtab progression. Fresh
owner/session scopes can start their own revision. Initial inspect/replay and
post-confirmation prompt/control projections check this policy before return
or subscription publication. SubscriptionReducer already calls the shared
policy atomically before updating any of its internal projection/counters.
No authority is added to the durable offline cache.

## Tests

New fixtures cover missing/partial/new workspace authority, regressing
workspace revisions with newer transcript or tab revisions, malformed/max
unsigned counters, independent owner/session scopes and absent journal fields.
A fragmented actual-frame response test presents a valid subscription page,
then an event plus stale workspace page: the old complete projection/cursor
remains, the event is not published, and the next wire page is not consumed.
This is framed-memory parser/client evidence, not a TCP or device execution.

Python Stage 2 tool discovery: exit 0, 86 tests pass. Backup validator tests:
5 pass; lifecycle validator tests: 15 pass; Blueprint validator tests: 5 pass.
Strict Blueprint validator and cargo fmt --all -- --check pass; all 36 items
remain unfinished. Current release-host source attribution verifies unchanged
binary 9f1ff46c2fd82ccad171534cbde89b3f7c6b49babaef8fd0f7686ad3019caaa8.
APK attribution verify-only still refuses missing current build provenance;
no old APK was installed or treated as current source evidence.

The first source compilation completed with 310 JVM tests, but subsequent
initial/confirmed-view policy checks and fragmented-frame coverage were added
while it ran. That run is not final-current-source evidence; the fresh final
compilation result is recorded below after its handle is terminal.

Final current-source command:
`zsh tools/stage2_android_source_check.sh /private/tmp/stage2-atomic-projection-final-20261003`
completed with exit 0, recompiling production/unit/instrumented Kotlin and
passing all 311 JVM tests. Existing compiler warnings remain. All required
command handles from this turn are terminal.

```text
main.jar 5eb4a22653b61df77f1782cfcebc28c7d6f1be4e09d7a788c1231c6bb71dd9a0
unit.jar 65ec269dd36ed3545af2b02e4a18f233df04c3fa92a22fa903de9760f6476724
instrumented.jar e34f62d3d831891da253c80806fb820032eb462145437ea85df3ba0ac020da89
```

```text
HostClient.kt 60506c2c808daa58c09fd9a9878d0655cf4c7530e33b004b28375d15a769d12a
PairingViewModel.kt 02f1b420f2b9ed1a070cd687dc1192788715231695402352aa77ea5ffe588484
ProjectionCommitPolicy.kt dfe311c33228502d398d15a146421a0fee2ea7270bf1d331bf6dfc1203f5f40f
HostProjectionTest.kt beab04ab28e78926b48c62f524c7b3793fac47ca5e8d669a1999fd58bedbd653
ProjectionCommitPolicyTest.kt 969a92118079fe45870bf26ac058f7296e17e5c407c5c12c2d9695a49fc1af0c
SubscriptionTest.kt a0f5bec3b7658dd56a4f6bed5b0f331ebab10c7945c6283c419f6035ee9cdb8d
ResponseReaderTest.kt ebee407312846a2471961eaef3f9a4f9c9d4fcd058beeac68ba800d1638c5da0
```

Goal remains active. Current APK, connected session/lifecycle parity,
physical-camera QR/direct LAN and signed/install/upgrade release gates are
still open; local source tests do not satisfy them.
