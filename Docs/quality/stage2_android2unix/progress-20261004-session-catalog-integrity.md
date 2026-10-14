# Stage 2 Retained Session Catalog Integrity Before Resume

Recorded 2026-10-04. Partial S2-036/S2-054 evidence. Prior goal turn progressed
worker metadata; this turn changes session catalog/resume admission.
Canonical cwd /Users/mac/Github/zenpi, dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push or permission bypass. All 36 Blueprint items remain [ ].
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

Session catalog sameScope/resumable only checked project/session and one
noncurrent target entry. Retained malformed snapshot, page or entry metadata
could be considered current even though wire decoding would refuse it.
Catalog validation now checks snapshot digest, 16-entry page capacity, bounded
aligned offsets/continuation, distinct session IDs, current flag bound to the
catalog session, canonical u64 creation time, nonnegative turn/match counts,
optional valid name and bounded query/snippets. Decoder results also satisfy
the same predicate. Retained history must form a valid at-most-seven-page
consecutive chain with unique session IDs.

Navigation and fresh resume dispatch/Compose controls check the full retained
chain. Command construction rejects invalid individual catalog; pending exact
intent dispatch remains unchanged. Fresh search/refresh does not require an
old valid catalog. Healthy read-only browsing remains distinct from resume
Control. Fixture catalogs that claimed continuation with fewer than 16 entries
were corrected to actual distinct 16-entry pages, not weakening production
validation to preserve them.

Expected/actual: malformed snapshot/offset/continuation, duplicate or incomplete
entries, current-flag contradiction, invalid creation/count/name/query/snippet
metadata cannot authorize resume or continuation. Changed historical snapshot,
cross-page duplicate IDs or oversized history disable fresh resume/navigation.
Healthy forward/back pages, current Rust-owner session replacement/history
resume/restart and exact pending retry continue passing. Kotlin also refuses
invalid snapshot/current flags on actual Rust catalog. Existing Compose case
disables resume/next on malformed snapshot while search remains enabled; source
compiled, not device executed. No actual delayed ViewModel race is claimed.

## Verification

Source run 2026-10-04T05:45:11Z through 05:48:44Z, canonical cwd, exit 0.
No source/test edits after launch. OUT and per-lane ZENPI_HOME isolated under
/private/tmp/stage2-session-catalog-integrity-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-session-catalog-integrity-20261004:
  main/unit/instrumented compilation, 546 ordinary JVM tests, production QR
  corpus and all explicit current Kotlin/Rust lanes; six-test desktop target
  and actual Agent prompt/tool/worker cancellation lanes passed.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-session-catalog-integrity-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests, not connected API matrix.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo
  lanes; current debug artifact reattributed, no Rust source changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Unchanged Gradle/socket/ADB blockers not retried or bypassed.

Final log /private/tmp/stage2-session-catalog-integrity-20261004.log.
Session frames /private/tmp/stage2-session-catalog-integrity-20261004/android-session-results.json.

```text
SessionCatalogCodec.kt ce5cc8e43657247c6b5f4b0b571e7676b7748ccfd091d3e9627af82d9b111fb9
SessionCatalogReadPolicy.kt 37a5cd75c17aa5a3179afe495a4107b39da895dfb4e390c1ce35b13aca9ce0f2
WorkspaceDispatchPolicy.kt 05f201f4c010936cc31580ccf601fec6c118dd64d604f86c8e1668307eeae975
MainActivity.kt 8477b031edeec2b73cd2196001187a403e67e061989621d62f4bbda7fa23dc9b
SessionCatalogTest.kt ce300e1a0b344d86d29dbf813cab2dfbee339873b0a87240d18f14a72bcf11b0
AndroidRustSessionTest.kt 69bde0e81b669939e4dee4803d6ae257895e4b8df6ae875f0782f1a88bb482a3
source-log dea62ccf0790cce89941e8f114ed7e56d8e4bb2c4b816dabb2b8ed5c8b44af40
session-results de474776373798117819efbcf3edb306bd2155d2b23f6d432b1e01b5aebdc73a
main.jar c0554fb997c477c19f9b526b85c52a751b642787c77128c857743ee6ef1fcfce
unit.jar bfcfd5290726a0e5c129c3fb68648d13442c90942ae9e144c6225507f234b467
instrumented.jar 1b3bb24f36f209fdd1328909f9d123b142c04ae5ca0f1d6f9c1cffd3cfa4f84d
host-receipt 374fe0f582976eb728533fab95bbbe0cccf0c98e321bf083f2ff77c2d2c3cf41
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35, complete desktop parity and signed
reproducible release remain required product gates.
