# Worker Read Admission

Provisional S2-031/S2-036/S2-038/S2-054 progress. Previous goal turn repaired
the reproducible camera QR detector and was progress. This turn inspected the
execution skill, Blueprint/specification, current dirty worktree and task
list/output/history paths. Existing changes preserved; no nested agents,
reset, push or checklist promotion.

## Source Changes

Task list entry/refresh/paging had connected-only UI guards, while ViewModel
dispatch omitted scanner state and allowed failed-list offsets. Completed
summary task history omitted scanner state in its read-scope policy. Added
WorkerStatusReadPolicy shared between Compose and ViewModel, separating fresh
reads/refresh from successful open current-scope continuation. Error recovery
can restart but not reuse old pages. Read-only clients and running master
sessions remain eligible; no task-control capability is synthesized.

WorkerReadPolicy now refuses scanning and missing/foreign/duplicate selected
project scope, including retained transient page-only targets. Status and
history codecs reject unlisted/foreign/mixed-duplicate selection before wire
construction or decoding. History requests are preflighted before clearing
dialogs or publishing loading state. Summary and listed task output/history
controls use these same fences. Existing digest/subtab/owner/revision,
completed-task identity and navigation-chain semantics remain.

Three new JVM tests cover read-only/running reads, scanner/reset/pending-write
refusal, retained page-only scope loss, recovery versus stale continuation and
codec ambiguity refusal. Existing worker read/history tests extend scanner
checks. Existing Compose task-list case adds reset/scanner/duplicate scope
disabled controls and error-refresh recovery; it is compiled, not device-run.

## Verification

Canonical cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. No whole-item acceptance.

- Initial development checker refused a test fixture's nonexistent Kotlin
  copy parameter; corrected to actual HostProjection/RemoteSubtabs fields.
  That failure is not counted as passing.
- /private/tmp/stage2-worker-read-final-20261004 complete checker exit 0:
  515 ordinary JVM tests, 54 production Rust QR pixel cases, main/unit/device
  source compilation and all explicit Kotlin/Rust interop lanes pass.
- /private/tmp/stage2-worker-preflight-final-20261004 final complete checker
  exit 0 after history preflight addition: 515 JVM tests, 54 QR cases, all
  source compilations and every explicit interop lane pass. Final five-test
  model/resource/file/diff/history target passes. Instrumentation compile only.
- Stage 2 Python discovery 137 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass, 36 rows [ ].
- APK verify-only refuses missing provenance. Unchanged Gradle/ADB restriction
  is not retried or bypassed; no stale APK install or physical scan/LAN claim.
- Current debug host build and verify-only pass after Cargo lanes.

```text
WorkerStatusReadPolicy.kt f02eec9bd4fe846e97e9a349df7476e13b9e51ffe287de37bd6b2d5d38978b3d
WorkerReadPolicy.kt 451956d870e8d6d600a2a0ff7aea9a836378935d7c8a1c9b6e8ce00c4cc7c6be
WorkerStatusPageCodec.kt aa0e6402f27adee8c6dbbf052907bcba1bc0fb40c2af768d70fe2569f8d1c814
WorkerHistoryCodec.kt 9579efa048b261ee758ee6d8b872b5e0e12f0e40e2503dffcd26aed7d60ec748
WorkerStatusReadPolicyTest.kt 52f1d176d74e75d1fcaa227b410f97203074a0c419f03ce72d7adfc4cbee1207
PairingViewModel.kt 5740341edb53dac3998b2fcb6f0f83ee08944a9b7fa9528def654799999083b4
MainActivity.kt 828f86d6a3bd25ea3d5512f792d67ca15a30abf1b6c84486f9c971d558c30e54
WorkerStatusPageScreenTest.kt bc19a5626e188a9eaacf3729e25eea2d3cf1b002cd8d7810b10409f192c565e4
final-source-log 4279fc98081768908c30c5215e69921b1963a2d7196a6ee4223fffc1cf3cab20
python-log 21eb98564aaae9b08204298a6e70d506d98184ac7c372dc84bd36f89840e7029
main.jar d0ba8f4168d9217d9df7775d2a368ac453d3e30c5614562b4c7af4732934110a
unit.jar dd1b654072e6eb42c7c8c55430dce7fb26912471d5db7cef772bbcd7c1ebaf4d
instrumented.jar a5deb9ca7f74c5438968a4ec710572adee8a563d7ce2f37450a52b50af7abab7
debug-host-receipt ef692523232f6259526269105cad10835ff95d473ff1f82bf8a7e48e097fe64a
```

Goal remains active and incomplete. Current APK, physical QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, full desktop
parity and signed reproducible release remain required. JVM/source evidence
does not substitute for those gates. Real worker journal/task-list Android
interop is a remaining coverage gap; existing lanes do not prove that workflow.
