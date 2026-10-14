# Stage 2 File Reads And Publication Require Current Tabs

Recorded 2026-10-04. Partial S2-036/S2-054 evidence. Previous goal turn
progressed PTY scope; this turn changes file metadata admission and late-result
publication. Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved. No nested
agents, reset, push or permission bypass. All 36 Blueprint items remain [ ].
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

File readability only checked a unique active tab; duplicate inactive IDs or
oversized lists could authorize metadata requests, decode and attachment
insertion. Asynchronous file publication compared selected IDs/revision but
not the validity of the current full projection. WorkspaceFilesCodec.readable
now uses SubtabProjectionCodec.current, and PairingViewModel publishes files
only when the returned value matches the current complete selection. Model
and resource publication use their existing project-scoped checks unchanged;
the shared sameSelection helper was deliberately not made file-dependent.

Expected/actual: healthy main/inactive-tab state permits reads and attachment;
duplicate inactive IDs and >32 tabs refuse request/decode/navigation/insertion
and returned-page current matching. Current Rust owner file pages still pass
the Kotlin read-only/pagination/back/nested/snapshot/refusal lane, while the
altered current list rejects their decode and matching. Existing Compose
attachment case disables directory/file selection with duplicate inactive tabs.
The current-matching predicate is JVM tested; no instrumented asynchronous
ViewModel race, device file insertion or provider attachment effect is claimed.

## Verification

Canonical cwd, source run 2026-10-04T05:23:37Z through 05:27:12Z, exit 0.
OUT and per-lane ZENPI_HOME isolated under
/private/tmp/stage2-files-current-tabs-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-files-current-tabs-20261004:
  main/unit/instrumented compilation, 541 ordinary JVM tests, production QR
  corpus, all current Kotlin/Rust interop, six-test desktop target and actual
  Agent prompt/tool/worker cancellation lanes passed.
- During that run, corrected a Compose assertion: Refresh host files exists
  only on read error, so its absent state is asserted rather than disabled.
  No production/unit source changed after checker launch. Recompiled all final
  instrumented sources separately with the checker's lines 21..44 dependency/
  compiler setup and the exact line 48 compile command, output
  instrumented-final.jar; exit 0. Log
  /private/tmp/stage2-files-current-tabs-instrumented-final-20261004.log.
  This is source compilation, not connected execution.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-files-current-tabs-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests, not API device validation.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo
  lanes; current debug artifact reattributed, no Rust source changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Unchanged Gradle/socket/ADB blockers not retried or bypassed.

Final log /private/tmp/stage2-files-current-tabs-20261004.log.
File frames /private/tmp/stage2-files-current-tabs-20261004/android-file-results.json.

```text
WorkspaceFilesCodec.kt 4ba0b11608b5f1c84b2bddd34cd50f6d2aecb0ff7efb7df702e9563c348bc9c1
PairingViewModel.kt b5aff60912e6073f879ec8e97a919582234c70e861e8be95f6284f17efa3f9f5
WorkspaceFilesTest.kt d418a4add8b19dad3314b0f04de8cad1ea787b6acf083566ae22e57e7f3ddb7a
AndroidRustFileTest.kt cd783023f9a5ec5dcbc69bf1b7c47111ec9e1310842c41c0c92d5f183ed6a155
WorkspaceFilesScreenTest.kt 1beb28fd968e9356cce86927e4dbfc17d7c0d30c7a988b7bfaede479e604a2a4
source-log 1d3e71ad8f62f19383a44a124aec9307d6201e002daa1168270a0c1465d72f69
file-results d6898711e8dd562b1c8e554f4507749ebbcbbff9cd8abb7bd8151ed77b044e16
main.jar 89dbb4965619c4e67b835e5f1d8a305760d8db8889cbabd0149455febb591482
unit.jar c5fc8275f6ccc1d50a35cd9de021efd1b6faadcf7b4a6d0ecd1e26b3fd39fbc6
instrumented-final.jar e01bc50217846c1ec1770dc79fd9e51767955aef0a6c56223fb9f4e68675b626
host-receipt 8a2844ab7cc4aea6a5a22f4f49396048ad2d950f6bf8f13ebfef8688bd53c343
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35, complete desktop parity and signed
reproducible release remain required product gates.
