# Stage 2 Worker Task Page And Retained Chain Integrity

Recorded 2026-10-04. Partial S2-036/S2-054 evidence. Prior goal turn progressed
diff scope; this turn changes task-list navigation and page-only selection.
Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push or permission bypass. All 36 Blueprint items remain [ ].
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

Task-list history compared selected project/session and adjacent offsets but
did not revalidate retained metadata/counts or every preceding page before
navigation and page-only cancellation/read selection. WorkerStatusPageCodec
now validates digest, bounded offsets, total/has-more/entry-count consistency,
at-most-16 scoped unique metadata-only tasks, and the complete at-most-seven
preceding-page chain. Cross-page duplicate request IDs are refused. First-page
retention must begin at zero; later retained chains may begin above zero after
the normal seven-page bound has discarded earlier history.

Read navigation, previous-page restore, and page-only task read/cancel use
these checks. Invalid current pages cannot authorize wire continuation; a
failed list may still start a fresh read. Fresh task-summary targeting remains
independent of submission tabs and page history. Exact persisted cancellation
retry is unchanged. Valid page data still comes from WorkerStatusCodec wire
decoding; retained integrity is not a substitute for signature/authentication.

Expected/actual: malformed digest/count/offset/total/has-more, foreign task
scope, preview payload on metadata pages, oversized/broken historical chain
and cross-page duplicates cannot authorize navigation or page-only cancel.
Healthy paging/back/cancel and exact pending retry pass. The former previous
page test used empty tasks for 16-entry offsets; corrected to actual distinct
entries. Current Kotlin/Rust worker-read helper now trims preceding history
when deliberately selecting an earlier page, as real previous-page navigation
does. Its edit preceded unit.jar compilation (file mtime 1791092095 versus
jar 1791092120); all final source was compiled. Existing Compose task-detail
case checks malformed count disables details while fresh refresh remains
enabled; source compilation only, not device execution.

## Verification

Canonical cwd. Source run 2026-10-04T05:34:26Z through 05:37:00Z, exit 0;
OUT and per-lane ZENPI_HOME isolated under
/private/tmp/stage2-worker-page-integrity-20261004. User credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-worker-page-integrity-20261004:
  main/unit/instrumented compilation, 544 ordinary JVM tests, production QR
  corpus and all explicit current Kotlin/Rust lanes; six-test desktop target
  and actual Agent prompt/tool/worker cancellation lanes passed.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-worker-page-integrity-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests, not connected API matrix.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo
  lanes; current debug artifact reattributed, no Rust source changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Unchanged Gradle/socket/ADB blockers not retried or bypassed.

Final log /private/tmp/stage2-worker-page-integrity-20261004.log.
Worker frames /private/tmp/stage2-worker-page-integrity-20261004/android-worker-read-results.json.

```text
WorkerStatusPageCodec.kt b2107004aad18404d6113cd02851e9786ac4925664b4c3790a98bfa83ba37cfb
WorkerReadPolicy.kt 8b9e1403b9863e82534866533f2e3920037217fbbb864fc8be9aae0c74c2f086
WorkerTargetPolicy.kt 60e1fbed51944947a2f741a568722fe4c6c9a3e730594e9a7831b4fc59d3cbea
WorkerStatusReadPolicy.kt 3efda0d99e7537d5584b07b32149be86bb681e0dc4e6086fb1acd01a3b59cee3
AndroidRustWorkerReadTest.kt 47b482632de73a6421c0fe79197bc2b78df6839fc3b48e476b2a7a66476506b7
source-log f89c3fbf11ecc035b0cb1455daee1ccf093c2d050b190a6252360cb3c58da059
worker-results 09913550185c4fecd4684fa194c583b50e6e7d740d7f471c982db0434282635d
main.jar d3a7a0fd402c35a5e503ca0339b036bb000deedf5a4030c96f59fbae02c01ebf
unit.jar 756c80ac3a107ef155bc8271a3216adeeec64fc6b71a7d06833524d867d39d4d
instrumented.jar 087de0d44bc083c4f73604192544d5f2a503474c27ccb59ca4e62771920faec6
host-receipt d2e4bb9a254017bc74433e55e1a6eb6853f945bed383055012148576492830ff
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35, complete desktop parity and signed
reproducible release remain required product gates.
