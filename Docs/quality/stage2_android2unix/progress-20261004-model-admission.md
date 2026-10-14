# Current Model Selection Admission

Provisional S2-036/S2-054 progress. Previous goal turn added model interop and
unified desktop dispatch, so it was progress. Current execution skill, Blueprint,
specification, dirty worktree, model read/write publication and retry paths,
policy tests and Compose tests inspected. Existing changes preserved; no nested
agents, commit, push or authoritative checkbox promotion.

## Defect and Fix

Android new-command dispatch checked model/reasoning control capability but did
not recheck the selection revision against the current displayed/live model
status. Workspace revision does not change when the host's model selection
changes. A prebuilt command could therefore be saved as pending despite a
newer observed selection, absent/failed/loading model read or removed reasoning
choice. The host CAS would still refuse a stale write, but Android should not
persist it as a newly admitted command.

WorkspaceDispatchPolicy now requires scoped registry-bound displayed status,
matching selection revision and successful nonloading read state for new model
commands. If the live projection carries status, its scope/binding/revision
must agree. Reasoning selections must be offered by both observations; default
null and explicit none remain distinct. Manual model entry remains host-
validated rather than restricted to a single catalog page.

ModelStatusCodec selectionCurrent/reasoningCurrent predicates are shared with
Compose controls, so contradictory live status disables the model field/apply
actions and removed reasoning options. Exact saved pending-envelope retry is
checked before these new-command predicates and remains permitted only under
the existing foreground/network/reset/scanner/revoke fences. No automatic write
retry, fresh identity, retargeting or pending-request reconstruction was added.

Four JVM tests cover missing/failed/loading/scoped/bound/revision status, live
owner changes without workspace revision change, default/none choice removal,
manual models, refreshed selection and exact pending retry. The current
Rust/Kotlin model lane now rejects each earlier command against actual updated
desktop statuses while accepting only its exact saved retry. Existing Compose
tests cover live revision/registry change and choice removal without revision
change. They are compiled, not device-executed evidence.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-03T22:52:36Z.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-model-admission-current-20261004: exit 0; 485 ordinary JVM
  tests, current QR fixtures, production/unit/instrumented compilation and all
  explicit current cross-language lanes pass. Earlier runs were superseded
  after sharing UI predicates and expanding Compose assertions.
- Python Stage 2 discovery: 135 pass; lifecycle verifier tests: 25 pass.
- Rust fmt, strict Blueprint and git diff --check pass. All 36 rows remain [ ].
- Cargo lanes changed binary attribution. Debug host rebuilt afterward; source
  inventory and binary hash unchanged by this Android-only fix.
- APK verify-only refuses missing provenance. No unchanged blocked Gradle/ADB
  retry, permission bypass, old APK install, physical QR/LAN or release claim.

```text
WorkspaceDispatchPolicy.kt 002feb646c00fa6c3a78de77f21ce72e88d9fab2f95498d2765a649139e1063c
ModelStatusCodec.kt 08defa0c791bd99ed24da0fa5b9762d983a495fd0aed2eb1e2e9b605d23b8dcb
MainActivity.kt de9b2f3b455e887835739dcc2218abd17a85afe634e132ae2fa59bb6d36d1d38
WorkspaceDispatchPolicyTest.kt 7a3cebb21134e254dac07b4a310bba9c87b0bf5091e248f331a68d27c7598668
AndroidRustModelTest.kt 31d0659c093c53ff42d62e25917b70816d993d3fc11682621e4a7464a9785406
ModelStatusScreenTest.kt 659dbb37c19780a3994e1e079239af0755371c632b92b93b6cdfa9e7b868740d
source-log 75130b9edcc2aaf3f47a89ccb646aa637a3b4675b8c85b765e4a5dcc660e04e3
model-results 62f4e170578a0d01e09f3eaa712624816daed25f90f413dba54398296a91d40b
main.jar 1b8b38f30575c152b5b9b530e520b77a7153bd87458388c70dc6f37f8511d713
unit.jar 581019f12db77b87418a1aa2b58c9593856bef20348fc3763426f61f1f0c789f
instrumented.jar 41cc1bd5bc324f7772711c0fe2b5d87c601afab40b618d85d27f3d756c842205
debug-host-receipt 521dbb4d7d002f0393df11613c1880b8c067d131885066de6eff11626f0e3a86
debug-host-binary c6c9e11b699e83b537f0c38e6f63bdc32e74f0f4a82338abd595e9715ac46fe6
```

Goal remains active and incomplete. Current APK/device UI, physical camera
QR/direct LAN bidirectional sync and recovery, supported API lifecycle,
complete desktop parity and signed reproducible release remain required.
