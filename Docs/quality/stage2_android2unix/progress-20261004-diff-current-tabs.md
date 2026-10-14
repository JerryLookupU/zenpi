# Stage 2 Git Diff Reads Require Complete Current Tabs

Recorded 2026-10-04. Partial S2-036/S2-054 evidence. Previous goal turn
progressed file scope; this turn aligns Git diff admission with current subtab
validation. Canonical cwd /Users/mac/Github/zenpi, dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push or permission bypass. All 36 Blueprint items remain [ ].
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

Diff readability previously required one matching active tab but did not check
the rest of the list for duplicate identities or capacity overflow. Current
diff readability and request construction now reuse SubtabProjectionCodec.current.
Legacy and paged decoding call request admission; current document selection,
read policy, UI navigation and existing asynchronous publication call
readability, so each uses the same complete current scope.

Expected/actual: a healthy main plus inactive tab keeps read-only diff and
continuation available without required_owner. Duplicate inactive IDs and 33
tabs refuse initial legacy/paged requests, continuation, response decode,
current patch matching and read/navigation policy. Unrelated event-cursor
updates retain healthy selection. Current Rust Git/Kotlin page lane validates
actual patch pagination and rejects those pages against the altered list.
Existing Compose patch case disables path input, refresh and previous-page
action under duplicate inactive IDs; source compiled, not device-executed.
No actual delayed ViewModel race or physical transport acceptance is claimed.

## Verification

Source run 2026-10-04T05:29:20Z through 05:31:56Z, canonical cwd, exit 0.
No source/test edits after launch. OUT and per-lane ZENPI_HOME isolated under
/private/tmp/stage2-diff-current-tabs-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-diff-current-tabs-20261004:
  main/unit/instrumented source compilation, 542 ordinary JVM tests,
  production terminal QR corpus and all explicit current Kotlin/Rust lanes;
  six-test desktop target and actual Agent prompt/tool/worker cancellation
  lanes passed. Instrumented compilation is not connected Android execution.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0,
  137 pass; /private/tmp/stage2-diff-current-tabs-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0,
  25 fixture verifier tests, not connected API matrix.
- cargo fmt --check, git diff --check and strict Blueprint validator: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo
  lanes; current debug artifact reattributed, no Rust source changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Unchanged Gradle/socket/ADB blockers not retried or bypassed.

Final log /private/tmp/stage2-diff-current-tabs-20261004.log.
Git frames /private/tmp/stage2-diff-current-tabs-20261004/android-diff-results.json.

```text
DiffProjection.kt 72b37992862c6affd33ddde88052e11f80d6f9a745ff22bc3ba0e580f48ad781
DiffProjectionTest.kt 994e520c125d6ead7298f8eff993a0aa9ba3b69990a63ade80453dc38785d094
AndroidRustDiffTest.kt 42838f8725df5779532fcfda036a7aae9347d43f85ef87a9a940d552330d9de9
DiffScreenTest.kt 99fe89b8ac36037327554fdea2160c1f907070dac1784c30ddcca5c99bf2dd4e
source-log 03cba021871e11bed71f8f902e0e4f509003a50284fbd44b48ec7eaae57b17b2
diff-results 69de0f5b506f1052cfe23a32698c74c4eba0207ea86f2576af41c3a2ff967ad5
main.jar a259acc1a86252c204d52771816f6fe5756417007d246dc410b692280fca6189
unit.jar 98a7c0e2cf3298b7fc84ca31379c17a8078683ace20586e43b2016ad4a577e4a
instrumented.jar 947c115ca6f930a87b98fbdfe7f5096eb398762be6ad2ee6d9ba0d532b05090a
host-receipt fa81c3085a9da540039689101fa7795a3b994401f7461b066ff986e02d4eac5c
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35, complete desktop parity and signed
reproducible release remain required product gates.
