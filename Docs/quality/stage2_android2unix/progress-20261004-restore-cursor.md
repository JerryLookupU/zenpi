# Stage 2 Retained Replay Cursor Integrity

Recorded 2026-10-04T07:30:35Z. Partial S2-032/S2-033/S2-038/S2-053
evidence, not acceptance. Prior goal turn progressed storage identity admission.
Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved;
no nested agents, reset, push or permission bypass. All 36 Blueprint rows remain
[ ]. Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

TranscriptRestorePolicy checked selected project/session and nonnull cursor,
but not canonical u64 shape, event <= journal or complete retained project
membership/uniqueness. Malformed retained values could be normalized by numeric
conversion before resume, or permit Busy fallback to reuse contradictory state.
Encrypted-cache decoding was stronger, but policy admission must not rely on
every retained projection having passed that separate boundary.

The shared replay/Busy predicate now requires canonical bounded event/journal
counters, event <= journal, at most 64 unique project IDs and exactly one
selected-project member with matching session. Invalid state refuses replay;
Busy fallback preserves the original refusal instead of inventing a usable
cursor. Fresh snapshot recovery and omitted tool-summary behavior remain.

Added TranscriptBusyRestoreTest noncanonical/overflow/missing/ahead-of-journal
counter and missing/duplicate/oversized/foreign project matrix, with zero and
maximum-u64 valid cases. Current Rust snapshot consumed by RustAndroidWireTest
now also validates normal replay admission and mutations of real cursor/journal/
membership. No wire schema, host replay window, cache or request identity change.

## Verification

Source run 2026-10-04T07:27:51Z through 07:30:26Z exited 0; no source edits
after launch. Canonical cwd; OUT and per-lane ZENPI_HOME isolated under
/private/tmp/stage2-restore-cursor-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-restore-cursor-20261004:
  current main/unit/instrumented compilation, 570 ordinary JVM tests, production
  QR corpus and explicit current Rust/Kotlin lanes pass. Includes actual current
  Rust snapshot negatives, Busy-owner frames, ACK/restart, six-test desktop target
  and actual Agent prompt/tool/running-worker cancellation. Instrumented assertions
  compiled only, not device-run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-restore-cursor-python-20261004.log.
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

Final log /private/tmp/stage2-restore-cursor-20261004.log.

```text
HostClient.kt 45043388a1d264f67a503fcd5340cec340216bb2a37a1d28f13b9030e2a6e992
TranscriptBusyRestoreTest.kt 6c59c947bdd14bf3e1ba847ba707b0577fc49a60f47a05622293562d62d51574
RustAndroidWireTest.kt 6717bc8d1622b0ffde788f88705230840a76281380465dff842a73f9d2ecdbdf
source-log 9480fb199a413a2a06a8d93a7c249e82eca3035e2cc07b308fcc56aba44042c3
python-log 085bd5aec0c88594aa6034382f4662b2d96b55b1989a4b69fc7f510c3e89a9ac
main.jar d49750860ef31e25e370cf123f9e26c7e69533031e2ab59c80fb9b30b242628d
unit.jar 949d12ed014cc7f5d3d727b3ae79641afad0d090f0f3c7e6cc68e08c49180252
instrumented.jar 7d5296e147572595b9301eff81d6a857db7eb1ee2ee0e4e1faf063576505c80a
host-receipt 5f8cf9ef4a081e5342619f9c2373fbc35de82ad4d5053255e698e917ca5a9e99
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required product gates. A concise
question about available same-LAN Android phone was submitted; no answer yet.
