# Background Session Race Recovery

Provisional S2-033/S2-038/S2-053/S2-054 progress at 2026-10-03.
Previous goal turn was verified snapshot consistency progress. This turn's
current-source lifecycle audit found that ResumeWorker did not opt into the
authorized session-race probe used by foreground reads. No nested agents,
worker launch, checklist promotion, commit/push or cleanup.

## Implementation

HostClient now has an opt-in recoverSessionChanges read setting. Only pure
automatic reads can use it: workspace/revoke and explicit bulk/resource/model
reads remain excluded. ACK/resume/inspect/subtab-list owner mismatch uses the
existing fresh authorized workspace listing probe. A verified same-project new
session produces SessionChangedException, never a mutation retry.

ResumeWorker uses ResumeReadPolicy: one matching race discards the old
projection/cursor and permits one fresh authenticated snapshot read. A second
race, wrong scope, authority refusal or cancellation propagates; no immediate
loop or stale display fallback. Both reads fit inside the existing 20-second
outer deadline. Final cache commit still compares the original encrypted-record
stamp; foreground updates, pending writes/revoke, re-pair or wipe invalidate it.
No subscription, signing-key creation or unresolved-command send is introduced.

Five JVM tests exercise success/first attach, one verified race, foreign scope,
authority/transport refusal, cancellation/second race, and the actual framed
refusal -> authorized listing probe -> fresh snapshot decoding chain. Existing
ResumeStore instrumentation now checks session replacement commits and a
foreground write invalidating the background stamp. No additional instrumented
test method was added, so the explicit smoke class count is unchanged.

## Verification

Commands ran from /Users/mac/Github/zenpi.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-background-session-race-final-20261003:
  exit 0; main/unit/instrumented Kotlin compilation, 345 JVM tests pass.
  Existing compiler warnings remain. Instrumented source was not run on a
  device; this is not APK/WorkManager scheduling/Keystore/network acceptance.
- ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home cargo test --offline
  --locked --test remote_subscription -- --test-threads=1: exit 0, 14 pass.
  Host-domain subscription/scope races only, not production TLS/LAN.
- Python Stage 2 tooling discovery: exit 0, 86 pass; lifecycle validator tests:
  exit 0, 15 pass; Blueprint validator tests: exit 0, 5 pass.
- Strict Blueprint validator, cargo fmt --all -- --check and git diff --check
  pass. All 36 authoritative rows remain unfinished.
- Host release --verify-only passes for unchanged Rust source. Android build
  --verify-only refuses missing APK build provenance. No stale APK installed
  or unchanged blocked Gradle/socket attempt repeated.

```text
main.jar 776188095649e87c1353bd75fc2e43b3cdd37716cb34e51e1108ea9d61b87f71
unit.jar c9778f559b92148dde3139392817da07ef0dde43415d8ec981096ea6b02c6167
instrumented.jar 9fba60521867aea3b024e6109b30aeadaaace3f2cb6adaa0f1bd8ced4d8cdbe5
ResumeReadPolicy.kt eeed007859493d6cac4b0fe6c9b2861840255c4ff4cb4089fc799dbc9b0632f4
ResumeWorker.kt f7ba660965381e38d930d3d725ae7ebcfa664aac9719eb709af5eb96584da66d
HostClient.kt cfdf796fa5e55725e45f762fd9e9731ede531091182d93180f91d7b3b846def2
ResumeReadPolicyTest.kt 7577802f87f6a55f210a92fa6fecb891df4d62998e5bc3be7ccc22353feb5122
ResumeStoreTest.kt f6266d0a3560ddfc2caf10c644e181f7cf184f48c37665a7bb8f55b8502fe1c4
```

Goal remains active. Current-source APK, production desktop-to-Android TLS sync,
physical QR/direct LAN, API 31/34/35 connected lifecycle and signed install/
upgrade remain required behavioral acceptance surfaces.
