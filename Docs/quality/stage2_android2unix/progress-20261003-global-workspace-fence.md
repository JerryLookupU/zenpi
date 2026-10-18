# Global Workspace Publication Fence

Provisional S2-033/S2-034/S2-035/S2-053/S2-054 progress at
2026-10-03T21:03:44+0800. Previous goal turn produced verified catalog-selection
changes. Current inspection found workspace monotonicity was skipped whenever
the selected project/session changed, although workspace revision belongs to
the entire host owner. No nested agents, worker launch, checklist promotion,
device launch, commit/push or cleanup.

## Changes

ProjectionCommitPolicy now checks same-owner workspace revision before checking
project/session identity. A different selected project, replacement session or
empty selection cannot admit a lower workspace revision. A different owner
starts a new scope; session journal, event, subtab and model revisions keep their
own scope. Event cursor monotonicity is independent of optional journal metadata.
Normal project/session changes can reset their session counters without resetting
the same-owner workspace revision.

Foreground subscription callbacks and final reads compare against the current
live projection before persisting or displaying their result. A stale live
publication keeps the existing display rather than replacing it from the
display-only disk cache, which intentionally has no workspace authority or
workspace revision watermark. Existing encrypted-record CAS, cancellation,
pending-write identity and host revision checks remain unchanged. This is not
a new persistent watermark or a cross-process workspace-version guarantee.

Three new JVM tests cover project/session/empty-selection rollback, owner
replacement, independent event counters, malformed unsigned revisions and a
ResponseReader subscription whose old workspace cannot replace the newer visible
or simulated durable projection. The publication test is a local reader/policy
fixture, not actual Android storage or the full ViewModel recovery path. Existing
session-reset expectation was corrected to keep the global workspace revision.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-live-projection-fence-final-20261003: exit 0; production,
  JVM and instrumented Kotlin compile; 356 JVM tests pass. Existing warnings
  remain. Compose/instrumented tests compile only, not connected execution.
- env ZENPI_HOME=/private/tmp/zenpi-global-workspace-test-home cargo test
  --offline --locked --test remote_subscription -- --test-threads=1:
  exit 0, all 14 tests pass. This covers host-domain subscription/replay,
  metadata revision/owner, session switch and wakeup fixtures, not TCP/LAN.
- Python Stage 2 discovery: 93 pass; lifecycle verifier: 17 pass;
  instrumentation verifier: five pass; Blueprint verifier unit tests: five
  pass. Strict Blueprint validator, cargo fmt --all -- --check and
  git diff --check pass. All 36 checklist rows remain unfinished.
- python3 tools/stage2_host_build.py: exit 0; fresh locked offline debug
  source attribution. Subsequent Cargo tests changed the binary artifact and
  verify-only correctly refused it. The attributed build was rerun after all
  Rust tests and verify-only then passed. Release verify-only also passes.
- Android --verify-only still refuses missing current APK provenance. No stale
  APK, unchanged blocked Gradle/socket retry, permission bypass or connected
  Android/physical LAN evidence is claimed. All observed sessions are terminal.

```text
main.jar 717ea1a57d85edde7f997208f22003388725f75b9dbf284fa31c2beb4413b0c0
unit.jar 8511189d83f7d247e405b9d738b0a1be6e36aecfe2914d23db0d9f92aa71ab5e
instrumented.jar 73ef026d610c29e4b2aa68b5932158dcb4c004daf4b34f66c8fece579a41468f
ProjectionCommitPolicy.kt 75c85e904b65d07e8069db5799c4cdc63f7d0a707a5e06fddab74771c2ed8110
PairingViewModel.kt ed43c24aa11aef082ab20fb8cbbe64fa74c4e0a752b19f7c4305cdc9abefc92c
ProjectionCommitPolicyTest.kt 2e8f8858bdb72abc74511eeb903a7c855bc1903a2e547b0d2b5670f04fd3563d
SubscriptionTest.kt 9b135f98d6e783542ca59ebba84cd322b61d34d528f283ae3c85800a7e382764
debug build receipt a865713eb3cb449addb82179ab9de9ccfdd19a182929661269caf82c1307ce01
debug binary 0146a6732f7d5ffb1be4ab532e3724dc6dcf75b90caadcc1234384fe9a6fa788
host inputs b6cac3ef3196283eeb7c6abdaf14a4e3ad2e0fd8c23e69edf9558c844ac64c0c
```

Goal remains active. Current-source APK, production desktop/Android TLS sync,
physical camera QR/direct LAN, API 31/34/35 connected lifecycle, full parity and
signed install/upgrade remain unverified or incomplete. These source fixtures
do not satisfy those behavioral gates.
