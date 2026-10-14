# Stage 2 Retained Worker Metadata Matches Decoder Admission

Recorded 2026-10-04. Partial S2-036/S2-054 evidence. Prior goal turn added
page/chain shape checks; this turn closes task metadata admission gaps.
Canonical cwd /Users/mac/Github/zenpi, dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push or permission bypass. All 36 Blueprint items remain [ ].
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Change And Criteria

Retained task page checks now call WorkerStatusCodec.validMetadata: bounded
nonempty request ID without controls, canonical subtab ID, known execution
state, valid failed-only failure code, allowed runtime-active state, and no
output/stream preview or truncation flags. Failure code session_changed remains
excluded as in existing PromptStatusCodec, not expanded into worker failures.
Empty metadata pages are valid only for the zero-task first page. Decoded
pages must pass this same page predicate before publication; wire decoding
still performs its existing typed state checks independently.

Expected/actual: malformed IDs/state/failure/runtime/preview flags cannot
become retained pages or page-only task reads/cancellations. Valid succeeded,
failed busy, running runtime-active and unknown-outcome runtime-active metadata
remain accepted. Empty first list passes, nonzero empty retained page fails.
Current Rust-worker/Kotlin list lane alters actual succeeded task metadata to
runtime_active=true; Kotlin refuses wire decode and retained scope. Existing
current-page pagination, history, output and targeted cancellation still pass.
The decoder-boundary empty continuation test also fails its existing terminal
continuity check; it is not claimed as independent proof of the new predicate.

## Verification

Canonical cwd, source run 2026-10-04T05:39:10Z through 05:42:38Z, exit 0.
Final ordinary test addition preceded unit compilation (mtime 1791092362 vs
1791092404). No source/test edits after compilation. OUT and per-lane ZENPI_HOME
isolated under /private/tmp/stage2-worker-metadata-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-worker-metadata-20261004:
  main/unit/instrumented compilation, 545 ordinary JVM tests, production QR
  corpus and all explicit current Kotlin/Rust lanes; six-test desktop target
  and actual Agent prompt/tool/worker cancellation lanes passed.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-worker-metadata-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests, not device API validation.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo
  lanes; debug artifact reattributed, no Rust source changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Unchanged Gradle/socket/ADB blockers not retried or bypassed.

Final log /private/tmp/stage2-worker-metadata-20261004.log.
Worker frames /private/tmp/stage2-worker-metadata-20261004/android-worker-read-results.json.

```text
WorkerStatusCodec.kt a14ee70d35e98e6b1d91a451f6a342dcc55ff57d62d32a1e59c382c9fa57e5bb
WorkerStatusPageCodec.kt 43ecc503f1fe07c4ca332fc1728e8a83b9105859b942b1b58dd7843e092d47a2
WorkerStatusReadPolicyTest.kt 4e7ebcc71ce30793fb943d959c8f0117311bd8594f7ef15781e60fe916af7825
WorkerTargetPolicyTest.kt a5f18b15bf0dc0a16e45735b4efa88264d5c2b96207efbe9ee385a5cabb53afe
AndroidRustWorkerReadTest.kt af49125ca8f43e02c1d9079d293e11b3b98cc2d2800052eb9c01f79bfc248c99
source-log 03ba619b0f628553740f1a3a7c47e1a5c6d5fd15c9d99f18445e6623fbe0c48a
worker-results 074276eac70d11700bc62b51d93d3d0dabab9594c559acb4814e66acbbb54434
main.jar cae11be2a1f8236d0a2256d62b5e52af534d1abfea2c9effedfbf614a7ee2453
unit.jar bd7058eb5bb42751ede0afb1e49166c3c09fba31d9fdbbe1b732750e55108166
instrumented.jar 087de0d44bc083c4f73604192544d5f2a503474c27ccb59ca4e62771920faec6
host-receipt 9826b9679d021f138434a92ec2c401d14b19ba06fb48a7a02291bc11834d7363
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35, complete desktop parity and signed
reproducible release remain required product gates. No device/transport success
or actual backend liveness is inferred from metadata fixture validation.
