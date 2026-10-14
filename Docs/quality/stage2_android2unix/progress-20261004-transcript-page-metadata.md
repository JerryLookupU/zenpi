# Stage 2 Retained Transcript Page Metadata

Recorded 2026-10-04T07:45:42Z. Partial S2-033/S2-036/S2-054 evidence,
not acceptance. Prior goal turn progressed recovery probe and host audit.
Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved;
no nested agents, reset, push or permission bypass. All 36 Blueprint rows remain
[ ]. Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

TranscriptNavigation checked bytes/digest/offsets and equality across retained
pages, but did not validate each page's identity/role/optional metadata shape.
A contiguous chain with identically malformed metadata could therefore pass
navigation admission even though current wire decoding enforced stronger shape.

Each retained page now validates bounded nonblank project/session/turn IDs,
supported role, paired worker request and canonical subtab ID, paired tool ID/
name with assistant role and bounded names, and canonical u64 historical journal
revision only outside worker pages. Existing byte/document/chain bounds remain.
The predicate is shared by remember, previous/navigation and continuation request.
No content is cached anew, no host execution, ACK or journal changes are added.

Added whole-chain metadata refusal matrix: malformed IDs/role, unpaired worker/
subtab/tool fields, invalid subtab, oversized tool name, nonassistant tool page,
bad historical revision and worker/master revision mixing. Valid worker tool
page and maximum-u64 historical revision remain legal. Current Rust historical
detail pages are mutated in AndroidRustHistoryTest to verify retained metadata
refusal before the original page is remembered.

## Verification

First source run exited 1 because the new valid-worker-tool fixture retained the
larger message document cap instead of the tool's 64 KiB cap. Corrected fixture
only, not production limit; observed terminal process before restarting.
Final source run 2026-10-04T07:42:51Z through 07:45:28Z exited 0, no source
edits after launch. Canonical cwd; OUT and per-lane ZENPI_HOME isolated under
/private/tmp/stage2-transcript-page-metadata-final-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-transcript-page-metadata-final-20261004:
  main/unit/instrumented compilation, 572 ordinary JVM tests, production QR
  corpus and explicit Rust/Kotlin lanes pass. Includes actual historical detail
  mutations, Busy-owner frames, ACK/restart, six-test desktop target and actual
  Agent prompt/tool/running-worker cancellation. Instrumented assertions compiled
  only, not device-run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-transcript-page-metadata-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 verifier
  fixture tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  current debug host reattributed, production Rust sources unchanged.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/socket/ADB constraints not retried or bypassed;
  no historical APK installed.

Failed log /private/tmp/stage2-transcript-page-metadata-20261004.log;
final log /private/tmp/stage2-transcript-page-metadata-final-20261004.log.

```text
TranscriptNavigation.kt 17fc5e53d78690171c64327426328af631aa16d58ce86779734d9257b65c4607
TranscriptNavigationTest.kt 5f5eda9357cc6c7b76dc6d5d92689fdbea0e5b3c075e44c1e1b47ea24dfa96fe
AndroidRustHistoryTest.kt 764e33288f59e047d130a3071e8e3af59c09d3a7d2518e4b6a87c38a6fbe136b
source-final-log 9ef41ad1d04e186077e1598d2f2c6885393499733e15e32a3b3fd21536da41ce
python-log 39e14eebb631c445a316059d555a93fd7570040525b322de1be7aae2129d9fb2
main.jar 2d05d6fc9b28d9fb837571e5f203e0c4f2b67172c569bdd781a7aaccc2f5394b
unit.jar 2b9d562e803850a26992a2542a1a41f8a820d76085d3e46051d6ec6e04aabf70
instrumented.jar 7d5296e147572595b9301eff81d6a857db7eb1ee2ee0e4e1faf063576505c80a
host-receipt 7a19ecb3a83ab5ef18fd9abbaa055fd98401b194adf7de67395cdd807116d720
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required product gates.
