# Stage 2 Catalog Write Construction Matches Current Scope

Recorded 2026-10-04. Partial S2-019/S2-036/S2-054 evidence. Previous goal turn
added actual Kotlin worker cancellation evidence; this turn repairs catalog
write construction consistency. Canonical cwd /Users/mac/Github/zenpi, dirty
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved;
no nested agents, reset, push or bypass. All 36 Blueprint rows remain [ ].
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Change

Resource reload previously filtered project ID/session/capability together,
which could ignore a same-ID foreign-session record when constructing a new
command. Existing fresh dispatch refused this ambiguity. Construction now
requires ResourceCatalogCodec.sameScope before capability selection, reusing
unique selected identity and catalog scope checks.

Model/reasoning construction now requires ModelStatusCodec.selectionCurrent;
reasoning also checks reasoningCurrent against live choices. Existing dispatch
and UI already used these checks. This prevents constructing a new command
from a page invalidated by a newer live selection revision or a duplicate
project with filtered-out capabilities; no host authorization bypass is claimed.
Valid manual model names remain for host registry validation; a separately
read status remains usable with no live modelStatus projection. Resource
reload from a later catalog page and exact persisted retry remain supported.

Two new ordinary JVM regressions cover mixed duplicate identities, live status
revision/binding/scope loss, changed reasoning choices, manual model, later
resource page and exact pending retry. Current Kotlin/Rust model/resource lanes
repeat negative construction and admission on real desktop snapshots while
their existing CAS/reload/durable-receipt workflows continue to pass. These
lanes are controlled in-process owner fixtures, not current APK/TCP/TLS/LAN.

## Verification

Commands from canonical cwd. Source checker controls isolated OUT and fixture
ZENPI_HOME directories; cached compiler/SDK compilation is not Gradle/APK.

- Intermediate zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-catalog-write-scope-20261004: exit 0; a negative assertion
  was made explicit during execution, so final attribution is the repeat below.
- Final zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-catalog-write-scope-final-20261004: exit 0, 536 ordinary
  JVM tests, main/unit/instrumented compilation, production QR corpus, all
  explicit Kotlin/Rust interop lanes including model/resources and worker
  cancellation, six-test desktop target, real worker-tool and prompt lanes.
  No source/test edits after final run; screen compilation is not execution.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-catalog-write-scope-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass;
  validator fixtures, not connected API31/34/35 execution.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes.
- python3 tools/stage2_android_build.py --verify-only: exit 1 missing current
  APK provenance. Unchanged Gradle/ADB permission blockers not retried/bypassed.

Final log: /private/tmp/stage2-catalog-write-scope-final-20261004.log.
Current model/resource exchanges: android-model-results.json and
android-resource-results.json under that same output directory.

```text
HostClient.kt 29a55a5c6a1d5a7488edf4b8b7be7e79c0dfe1b9f4b9f0d86fc479c14ed30496
ResourceReloadTest.kt 5728a9ce2b3456a1b9d432755be6cfb4ddfb8e36572c7a84f1dc8dd973302d16
ModelStatusTest.kt 69ea93d2503958fad5dac0eaf5060ea9b3fd2819c0c63dbbb0dd7c76895b7897
AndroidRustResourceTest.kt 238c9766bfc43e5b5bff51e8a057b4446d1e0aa79623b2bbc117c74f052cfeab
AndroidRustModelTest.kt c4923ea7f90d750cf0f9bc00be0181038d4b89d0036fbf32726b376045e98209
final-log 8432d839ed67f0ef5195b2f09ad47a1a33b971b03a9db6ac89b3b67f24ab57b7
model-results 1ccbe4c453e34c8150381c7124b8e881793c688f5ea4b486bbd0ab14be0ef0ea
resource-results 879102e7833e6e266b06754a18ffbc00cddd581b1d6e99cbd6e768f4679b78bc
python-log 6844c234b33ada1940c6fea747c8bd3d909909b049b35f8c8a49c3dbb63996c1
main.jar 926ad203ccf14deb856acc10a9956a775b146cd0ba324a22872e87dba8d6e8a3
unit.jar 11f45e2e677aa3a0a9d9e68a832ad4abe2ddb122f61248b168e97f7a9d0db291
debug-host-receipt 69eb425656599dd21616b0b0614141446f8d36c2c770b97bd8018889bdacec5c
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35 lifecycle, complete desktop parity and
signed reproducible release still require authoritative evidence. These
construction consistency fixes do not replace the product acceptance gates.
