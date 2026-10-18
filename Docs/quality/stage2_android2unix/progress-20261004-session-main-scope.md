# Stage 2 Session Writes Require Current Main-Subtab Scope

Recorded 2026-10-04. Partial S2-036/S2-054 evidence. The preceding RTK
installation check did not change product state; this goal turn fixes session
write admission. Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved. No
nested agents, push, reset or permission bypass. All 36 Blueprint items remain
[ ]. Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Change And Criteria

The former session guard checked only first-tab ID equality with active ID.
Duplicate tab IDs or malformed main classification could satisfy it. New,
resume and rename now share SessionCommandCodec.mainSelected in construction,
fresh dispatch and Compose controls/confirmations. It first uses current
SubtabProjectionCodec scope validation, then requires first kind main, active
first ID and no second main. Current unique project/session, 1..32 unique
tabs and a present active ID are necessary. Existing capability, owner,
revision, catalog and journal checks are retained.

Expected/actual: JVM rejects null/foreign/empty/duplicate/oversized tabs,
non-main first tab, additional main and selected non-main for all three
operations. Healthy main plus one in-place tab still authorizes them. Captured
commands cannot be freshly dispatched after invalidation; only the exact
saved pending envelope can retry, not a changed request ID. Current Rust-owner
session fixture also rejects duplicated actual tab state before new/rename/
resume construction; a valid synthetic catalog target confirms resume refusal
is due to subtab state rather than absent history. Existing actual owner
replacement, history resume, conflict and restart receipts continue passing.

Existing new/rename/resume Compose cases now cover duplicated/foreign tabs.
Resume browsing/search remains read-only and available with invalid tab state;
only session writes are disabled. These cases compiled, not device-executed.
An initial source run exited 1 on Kotlin cross-module nullable-property smart
casts in the new screen assertion. Fixed using stable local tabs; final run
below includes all final source edits (instrumented.jar newer than those files).

## Verification

Commands use canonical cwd. Source run 2026-10-04T05:08:18Z through
05:11:21Z; exit 0. Controlled OUT and per-lane ZENPI_HOME are isolated under
/private/tmp/stage2-session-main-scope-final-20261004; no provider configuration
or credentials changed.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-session-main-scope-final-20261004:
  main/unit/instrumented source compilation, 537 ordinary JVM tests, production
  terminal QR corpus, explicit Kotlin/Rust interop, actual session owner lane,
  six-test desktop target, real prompt/tool/worker cancellation lanes passed.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-session-main-scope-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests; not connected Android lifecycle validation.
- cargo fmt --check, git diff --check and
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after source
  Cargo lanes; reattributed current debug binary, no Rust production changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Unchanged Gradle/socket/ADB blockers not bypassed or retried.

Final log /private/tmp/stage2-session-main-scope-final-20261004.log.
Session frames /private/tmp/stage2-session-main-scope-final-20261004/android-session-results.json.

```text
SessionCommandCodec.kt 9ef5de4a553af44038c0e5abcc0701c61ec0d431c425fbd8d159f9740b5f0c3a
WorkspaceDispatchPolicy.kt 728c8deb257a39c65a0ba7df97eea70b969a41cdb1f3e723e471489e7b3cdea2
MainActivity.kt 313f3086ae0f880d777dc03f4f957bbaaa502a531d01ffd90df66eca27c09528
SessionCommandTest.kt e3635797d48aa849b1be79cd304dc033160399b3546ffc036eb60b58a6bdc748
AndroidRustSessionTest.kt 69adf5a7ca2b719256609c36e58ee74c86c43d5fa7b242b84f87d52de5ba9c6a
source-log e51681dcbcb09103106044b487c69a3558ed7a2cb9ef3dafbabbf1727dbdd2df
session-results dfdfc1dd094e753ab3be4ae585da6d7f03bb0cb8bd5559dbe05b757bb5e40d4e
main.jar f32fd1b9db7cc930ebc7ef57ab99cdf25068154b852f69d83697df32e71f5063
unit.jar c8da2079192917b854b486f6e4189d9028f4ee78d12008fe809700690c192ab4
instrumented.jar 058246d344329d728c7464a1d408e05915eeb0ead93dd1cf154fffbd1d2ef78b
host-receipt 46818e50b136ec88a6a97280239fcb372c66642770c41c3dad3b919e06b378f2
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR, direct-LAN TLS
bidirectional synchronization/recovery, connected API31/34/35, complete desktop
parity and signed reproducible release are still required. Local source and
in-process owner tests do not supply those product gates.
