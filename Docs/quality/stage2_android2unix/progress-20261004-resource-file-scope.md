# Resource And File Read Scope

Provisional S2-031/S2-036/S2-038/S2-054 progress. Previous goal turn repaired
model read admission and was progress. Execution skill, Blueprint/specification,
dirty worktree, actual resources/files UI entries, policies, codecs and interop
lanes inspected. Existing changes preserved; no nested agents, reset, push or
checkbox promotion.

## Source Fix

Controlling clients with an accepted/running prompt had no resource metadata
browse entry even though ResourceReadPolicy admitted reads. Added independent
Browse host resources entry while the message is active; insertion remains
disabled, and no prompt/reload capability is synthesized.

Resources/files policy scope predicates matched project ID and session together,
allowing a mixed duplicate project ID to appear unique. Resource initial wire
requests checked only selected IDs. File initial requests did not verify listed
project membership or actual active-tab existence. Added codec-level readable
checks shared by policies and request/response paths: selected project ID
exactly once with matching session; files also require one real active subtab
in that project. Missing, foreign or mixed duplicate scopes cannot dispatch.
Resource continuation and reload preflight reuse navigation admission, including
scanner/reset/error guards. Failed reads can still fresh-refresh.

Two new JVM tests and existing lifecycle-policy tests cover unlisted/ambiguous
scope and missing/duplicate tabs. Actual Rust/Kotlin resource/file lanes extend
request/decode refusal checks on real views. Existing Compose resource paging
case adds running-client entry, no insertion, scanner/reset/duplicate-scope
disabled controls. That case was corrected during development to place its
paging assertions in the actual paged fixture, not the empty reload fixture;
instrumentation remains compiled only.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b.

- Complete /private/tmp/stage2-resource-file-scope-20261004 exit 0:
  519 ordinary JVM tests, 54 terminal QR cases, all three source compilations,
  every explicit Kotlin/Rust lane, six-test desktop target and real tool loop.
- Final /private/tmp/stage2-resource-file-final-20261004 complete checker exit 0
  after instrumentation/interop assertion corrections: 519 JVM tests, 54 QR
  cases, all source compilations and explicit lanes pass. Instrumentation
  remains compiled only, not device-executed.
- Python Stage 2 discovery 137 pass; lifecycle verifier tests 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass, 36 rows [ ].
- APK verify-only refuses missing provenance. No unchanged Gradle/ADB retry,
  permission bypass, stale APK installation or physical camera/LAN claim.
- Debug host rebuilt and verify-only passes after Cargo lanes.

```text
ResourceReadPolicy.kt d1c9816a1617649c5601ac4fb9fa7b7ceb1e75cd66e4ea5eea0a469f854d3f59
FileReadPolicy.kt b0b95f4f11e2abb5b0b8d14d43ec11db27e84e78ab76e73ad0bcb2b9aae5fd91
ResourceCatalogCodec.kt 6b968938aa58129c849903ed6c2f67b9f412202b542f733380e4c83713b5c84f
WorkspaceFilesCodec.kt 8692f1c9a16974311c8e7c3d8d8efe75d85c5698569819e802c43dc4f4a52218
MainActivity.kt 0f281b05a184d38176b55907e7e25ea5bcb3cabc58fb6dddc46b3f8044205341
PairingViewModel.kt c89a8c84c61f31d13b9d6ba83fecbc9d753de1d4b310dc66c56b959591e60050
final-source-log 959bbe94e9539f1ef1871e644d77497291ae3fa24f2267b79e07306e65242319
python-log 7f1cd53f058d1b9a839e03bba7bd486a8d7b51b1a510b44b13157a0dfca0a059
main.jar 1ba59004a1c58cba00386a096cb1057244e897e4da8dec5381bedd2b73117c3e
unit.jar d302d707131af1fcc8b56a05d01ea8660dd2a79b061a7ed839c58e30caca2553
instrumented.jar 26cdab142b99bde470053e23b1cc6324351100b8880326edd4d290faeda0892b
debug-host-receipt b9eadbcae1b6a828601fb3b8395328005f5c55bdf390a7b3b578877593fefb66
```

Goal active and incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35 lifecycle, complete desktop parity and
signed reproducible release remain required. Source/JVM tests do not replace
those gates; no whole-item acceptance is claimed.
