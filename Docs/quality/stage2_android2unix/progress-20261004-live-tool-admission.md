# Stage 2 Live Tool Argument Read Admission

Recorded 2026-10-04T06:33:06Z. Partial S2-036/S2-054 evidence, not acceptance.
Prior goal turn progressed history-page summaries. Canonical cwd
/Users/mac/Github/zenpi; dirty HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b.
Existing changes preserved; no nested agents, reset, push or permission bypass.
All 36 Blueprint rows remain [ ]. Blueprint SHA-256
d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

Live argument buttons/ViewModel admitted a matching call ID without checking
the full retained tool summary. Transcript request and page selection also
checked equality of the selected call but not its metadata. A mutated retained
summary could therefore construct or preserve an argument read despite being
invalid as an equivalent wire projection.

TranscriptReadPolicy.tool now checks lifecycle, unique turn/call, assistant
role and the shared summary predicate before initial UI/ViewModel admission.
TranscriptPageCodec ordinary retention and live/worker request construction
check that predicate; decoder inherits request admission. TranscriptReadPolicy
page matching rejects a malformed selected live/worker turn too. Summary
checks do not grant tool execution or workspace mutation authority.

Expected/actual: oversize/control-byte IDs/names/previews, NUL, duplicates and
invalid truncated-count metadata refuse initial tool actions, fresh and
continuation requests and retained page navigation, even with matching call ID.
Read-side refresh remains available; valid eight-call truncated summaries pass.
The existing Compose truncated-list fixture now contains eight distinct calls
instead of a contradictory one-call truncated list. Current real Rust tool-loop
metadata is mutated in Kotlin and refused by direct argument request creation.
Compose assertions compile only; no device execution is claimed.

## Verification

Source run 2026-10-04T06:29:09Z through 06:32:46Z, canonical cwd, exit 0.
No production/test edits after launch. OUT and per-lane ZENPI_HOME isolated
under /private/tmp/stage2-live-tool-admission-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-live-tool-admission-20261004:
  main/unit/instrumented compilation, 554 ordinary JVM tests, production QR
  corpus and all explicit Kotlin/Rust lanes pass. Includes actual tool-loop
  history/arguments, six-test desktop target and actual Agent prompt/tool/
  running-worker cancellation paths.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  /private/tmp/stage2-live-tool-admission-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, 5 pass.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  current debug host reattributed, no Rust source changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Unchanged Gradle/local-socket/ADB blockers not retried or bypassed;
  historical APKs not installed.

Final log /private/tmp/stage2-live-tool-admission-20261004.log.
Tool exchanges /private/tmp/stage2-live-tool-admission-20261004/android-worker-tool-results.json.

```text
TranscriptPageCodec.kt 86f619b068972cdbf4b37ed45bc374cb6129f75a7f9e4c106910d3fdb7d0a21f
TranscriptReadPolicy.kt e8afd4e5c33e6c7e9082210a23a50e7d32dfcd89dfa26546f008dcc617cf43ca
PairingViewModel.kt dc13dea2744069eeb42833c8c791c897c4d8bd735153e2268670324e593b1bc7
MainActivity.kt 52ad07b6369328770fde027147413804e3ba9f05e5958e0a558a2e4638c3cf49
TranscriptReadPolicyTest.kt db09b30d83451647bf9c80c034ec7aa3941c3ca19412f0c40a113d0a9d947dd9
AndroidRustWorkerToolTest.kt 6215f43cec7b244d5c128948c4261a2c519137ded6c0c08f2fa47717acab0696
TranscriptToolScreenTest.kt ed544004d1f0382e130b8d1688fdb60463f682f8fb70fcbb323ccabbfba774e7
source-log d7f5d70725d483510a7b0048f163594efb49f9e7eadc5dd9a00d71baf013bc4b
tool-results 94188643d30271539f488880a5844f3363bca253a83822177c973fb616f18abf
python-log d373526de1bbe3dd8b3fe73f141b4f5d2b0a17cf4f50c355ed235ae7b3196f33
main.jar 2bc86b069f5a5327c94384e91ddac6731134912134e00a0fb8bc480ffea99d22
unit.jar 2e70ad49f0aef27155ce964eeebf38ef667d06ce7e226d747df943681e4bdc55
instrumented.jar 3876a82e394dbb6166a48068159f07b7047e5d142141afa9d359a6ff85bc099c
host-receipt 494f88a19b295ad2e836e16a46540bfa29248b600f1df5534d82172833130113
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35 lifecycle, complete desktop parity and
signed reproducible release remain required product gates.
