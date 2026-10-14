# Stage 2 Resource Metadata Is Revalidated Before Draft Insertion

Recorded 2026-10-04. Partial S2-036/S2-054 evidence. Prior turn progressed
session catalog integrity; this turn fixes resource-command draft admission.
Canonical cwd /Users/mac/Github/zenpi, dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push or permission bypass. All 36 Blueprint items remain [ ].
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

Resource insertion checked invocable and draft prefix/size but trusted the
resource command string. Wire decode checked metadata, but a retained/copied
resource could bypass that boundary and prepend shell or mismatched command
text. ResourceCatalogCodec now shares validResource across decode, catalog
scope/selection and insert: valid bounded name, skill/template kind, exact
name-derived command and bounded control-safe description. Catalog selection
also refuses duplicate identities and more than 32 entries. Draft NUL is
refused; ordinary multiline/literal arguments remain accepted.

Expected/actual: altered resource name/kind/command/description cannot insert
or authorize catalog navigation/insertion. Fresh browsing remains possible.
Duplicate/oversized retained entries refuse selection. Current Rust resource
entry inserts its exact command plus literal $1, while altered shell command
refuses. Existing generation/paging/reload/receipt and exact pending retry
remain passing. Existing Compose case disables a retained altered shell entry;
source compiled, not device executed. Resource paging/generation shape is not
newly validated here; metadata validity is not authentication or an actual
skill/template execution claim.

## Verification

Canonical cwd, source run 2026-10-04T05:50:32Z through 05:54:01Z, exit 0.
No source/test edits after launch. OUT and per-lane ZENPI_HOME isolated under
/private/tmp/stage2-resource-command-integrity-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-resource-command-integrity-20261004:
  main/unit/instrumented compilation, 547 ordinary JVM tests, production QR
  corpus and all explicit current Kotlin/Rust lanes; six-test desktop target
  and actual Agent prompt/tool/worker cancellation lanes passed.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-resource-command-integrity-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests, not connected API validation.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo
  lanes; debug artifact reattributed, no Rust source changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Unchanged Gradle/socket/ADB blockers not retried or bypassed.

Final log /private/tmp/stage2-resource-command-integrity-20261004.log.
Resource frames /private/tmp/stage2-resource-command-integrity-20261004/android-resource-results.json.

```text
ResourceCatalogCodec.kt af53589670a0915aaa30e3d0cbfb1e0f16abd13b994a0a4062879ac64382e8f1
ResourceCatalogTest.kt f2c1646dfba1482847406504569d3bb02f7949ba5f32cdd29b021dbb8a7c20b2
AndroidRustResourceTest.kt 761fb866126412bac85d546c4cd7d4a582c1e04136b7d822906ab8f431ee0bc3
ResourceCatalogScreenTest.kt 6181d26fb3b3a845e0848d3711d73cb2b1702beeaa5b977bc2a004caaf81f096
source-log d50a8a7fcc720540df11ab61b199c2d17f67d78effed6c5518c34b449eaac8d0
resource-results 1e01bed8d2d4386357b270abf8ad4cb32729d5bcf29db0aa3e77a9ba87b2b5aa
main.jar d1ecb70d2bc7fbeafc6be01a95eda31aae023e7d4b5e2a5258ff763981b712d9
unit.jar 2f10c17cf847a0bb5470a5968d5ece5980faa46e78b869932454f3394df518a6
instrumented.jar 9d673adcb9b0550dd883e59c0c34711191cf22f9d00cf1d7ff008280ec7eaf37
host-receipt be7d6cf819d67bed9948e448f244420bcdc62921c801b63366fbaa6d40b74d8a
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35, complete desktop parity and signed
reproducible release remain required product gates.
