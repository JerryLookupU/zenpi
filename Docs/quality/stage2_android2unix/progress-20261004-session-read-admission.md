# Android Session Catalog Read Admission

Provisional S2-036/S2-038/S2-054 progress. Previous goal turn strengthened real
Read-only PTY interop and was progress. Current execution skill, Blueprint/
specification, worktree, session catalog codec/UI/ViewModel and existing tests
inspected. Existing changes preserved; no nested agents, reset, push or checkbox
promotion.

## Defect and Fix

ViewModel catalog reads omitted the scanner fence. Catalog entry and paging/
refresh buttons omitted reset/scope fences, although search and resume had
some independent guards. Paging did not require an open successful current
catalog matching the retained query. Initial codec requests required selected
IDs but not a unique currently listed project/session. Invalid fresh reads
could close the diff dialog before request validation.

Added SessionCatalogReadPolicy shared by entry/search/refresh/page controls and
ViewModel admission. It requires connected unique current listed scope and no
scan/reset/loading/pending write/revoke. Navigation requires open, error-free,
current catalog and exact retained query. Read-only and running-task projects
remain browseable; session-resume authority stays separate. Codec checks
unique listed scope for initial requests too. Request validation now precedes
closing another dialog or device-key access.

Three new JVM tests cover read-only/running/error refresh, lifecycle/unique
scope/pending-write rejection, closed/foreign/query-mismatched page fences and
initial request refusal. Existing named SessionCatalogScreenTest extended for
scanner/reset/scope page/search/refresh disablement, read-only paging versus
resume, and error refresh. Updated fixture includes its actual retained query.
Instrumented cases compile only; existing interop lanes rerun, not credited as
new actual Read-only session catalog network coverage.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-04T00:04:30Z.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-session-read-admission-20261004: exit 0; 503 ordinary JVM
  tests, main/unit/instrumented Kotlin compilation, current QR decoding and all
  explicit current Kotlin/Rust lanes pass; model/resources/files/diff target 4
  pass. Device UI cases are not executed by this source checker.
- Python Stage 2 discovery 135 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass; 36 rows [ ].
- Debug host rebuilt after Cargo lanes; verify-only passes.
- APK verify-only refuses missing provenance. No stale APK installation,
  permission bypass, physical QR/LAN or connected UI execution claim.

```text
SessionCatalogReadPolicy.kt bc7b4e72f85656c30b5d6ac4e6d1d089e43d66c6798998427d2678fafe5cb7b4
PairingViewModel.kt ad369030fb9321c612b6183c8a8533d6bce9a080857f3991eddb00da337945db
MainActivity.kt ce24a6e97b852d1c6075cf5e7340630113a87de9a0706696fc66a67c1391a5d2
SessionCatalogCodec.kt 54cbb5ee4ae025c07aa62de9145e2e406c334f807f250b7cab3dab912b6e9182
SessionCatalogReadPolicyTest.kt a3f0de39b79f88052a81af330e9b946fb7ece54aa30770beb576d93577ec82c0
SessionCatalogScreenTest.kt 7cae49b0d911349f4f375a3e73ebf0954f74d417fc035f6ed6cc831dc1295e1c
source-log 5fe46f5881df1a85a661cfbbd8b7d09dd843b41fc7c45493129f22b9140814db
main.jar 1c8bd3a9e43f94628bd2912b35a638b90f9b4cf0834b96d0bd2e9e05cda4ef91
unit.jar 47d1306bdab4e8a7dfe5e36454f6c457715ca1bd1503b64d8a5ce519c89265b5
instrumented.jar ba395433cf7913b7761598e5fb76da027f2f41eb087dd39af1db6a744d652659
debug-host-receipt 1f805f75b533b6d781ef0000bc2150bb03525f0e1030856f0d19a384f39aaa1e
debug-host-binary 51cf433e9ebfd59aeebdbf46f844c0603872e798f8e4b6443308aed8627eca67
```

Goal active and incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API lifecycle, complete desktop parity and signed
reproducible release remain required unaccepted gates.
