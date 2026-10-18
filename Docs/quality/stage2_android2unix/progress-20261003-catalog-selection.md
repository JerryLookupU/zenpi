# Catalog Selection Scope and Continuity

Provisional S2-033/S2-036/S2-052/S2-054 progress at
2026-10-03T20:58:23+0800. The preceding goal work produced verified
instrumentation-identity changes; the intervening RTK check confirmed the
installed local tool and did not change Stage 2 product state. Existing source
check session 96060 completed with 351 JVM tests. Rust session 48658 was no
longer available, so its result was not assumed; the resource test was rerun.
No nested agents, worker launch, checklist promotion, commit/push or cleanup.

## Product Changes

Resource selection and reload now require a catalog scoped to the selected
project/session and project membership, with no error/reset/loading/pending
state. Continuation requires adjacent offsets and unchanged scope, generation
and snapshot. Duplicate entries against the current or retained pages are
rejected. Previous-page history is consecutive and bounded to seven pages.
A stale successful read closes the resource panel instead of leaving it loading.
Host resource generation/CAS and prompt admission remain authoritative.

Model status decoding now validates the requested adjacent offset before
accepting the response. UTF-8 ordering is checked both within and across pages,
including backwards reads. Retained history shares one scope, selection
revision and snapshot and refuses duplicate IDs. Model/reasoning controls require
the displayed project/session to match the current projection and membership,
with no model error or local reset. A fresh status read can recover an error,
but an old Apply or continuation cannot. Late stale replies close the panel.
Newer pushed status or catalog changes clear previous-page history; unchanged
snapshot pages retain it. Host write ownership/revision checks are unchanged.

Two new model JVM tests cover history bounds, scope/revision/snapshot changes
and membership; existing pagination tests cover skipped and overlapping pages
in both directions. Existing Compose tests add foreign-scope/error/reset fences
without changing their method count. Resource tests were expanded in the prior
source run. The explicit desktop suite now includes ModelStatusScreenTest as
well as ResourceCatalogScreenTest: 66 tests, or 67 with Doze (20 base, 46 control,
one Doze). Exact named-test verification remains required. This is preparation,
not a new connected execution or proof of the complete host catalog digest.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-model-selection-final-20261003: exit 0; main, JVM and
  instrumented Kotlin compile; 353 JVM tests pass. Existing warnings remain.
  Compose tests compile only, not device execution.
- env ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home cargo test
  --offline --locked --test remote_workspace
  resources_catalog_is_scoped_metadata_only_read_and_invokes_existing_agent_resources
  -- --exact --test-threads=1: exit 0, one pass.
- env ZENPI_HOME=/private/tmp/zenpi-model-selection-test-home cargo test
  --offline --locked --test remote_workspace
  model_catalog_pages_bind_selection_and_metadata_without_side_effects_or_private_fields
  -- --exact --test-threads=1: exit 0, one pass. Both Rust tests are host-domain
  fixtures, not Android/LAN or production listener acceptance.
- Python Stage 2 discovery: 93 pass; lifecycle verifier: 17 pass;
  instrumentation verifier: five pass; Blueprint verifier unit tests: five
  pass. Strict Blueprint validator, cargo fmt --all -- --check and
  git diff --check pass. All 36 authoritative checklist items remain unfinished.
- Host release --verify-only passes with unchanged attributed Rust source.
  Debug --verify-only refuses stale inventory. Android --verify-only refuses
  missing APK provenance. No stale APK installed or unchanged blocked Gradle
  attempt repeated; no device/emulator launched.

```text
main.jar 902557ea2f523e0a076b8007c552755beb2e33b1c02dcb9eedbc6d2f3f1568df
unit.jar 544ad5e8261d42e1c5dbf8fc0b49929dc2a262fdeff66eb9e0f8c72911e862e2
instrumented.jar 73ef026d610c29e4b2aa68b5932158dcb4c004daf4b34f66c8fece579a41468f
ResourceCatalogCodec.kt 9ebdfbbd83fcc84dd1c5bb0493ce8dbb973c82ab0662f9b469df26786f1f85c3
ModelStatusCodec.kt 1df5d387c7b3b265f73439761bf95fbc1ae6acefaa008ce632b8484e2ac155d9
PairingViewModel.kt 623b49eb650a6ace542eddcd62b6bdb803327e3e43d5a6610b165be142a232dc
MainActivity.kt e9164f4ff8f4582c5c62d3eff71c691263f07f7b5f9720481f7309ff61186a20
ModelStatusTest.kt f7077dd4d9f74acc61e45ec5c5530e900dd9bacafe347a87c80bf504dd788acf
ModelStatusScreenTest.kt 00eb73b109beb204207dc25956fe63de5ab362cc8c85d946f933211425c0c571
stage2_desktop_android_smoke.py 721a6f5dd43887974bf91ab641e52d039346f7a23656f9ff1b33e80687791cab
release binary 7e1b2b68b9940c8b0312598c4edc42f277f4acd5b962c185fad7600fae481a45
```

The full goal remains active. Current-source APK, production desktop/Android
TLS synchronization, physical QR/direct LAN, API 31/34/35 connected lifecycle,
complete parity and signed install/upgrade require their own current behavioral
evidence. Source/JVM results cannot substitute for those gates.
