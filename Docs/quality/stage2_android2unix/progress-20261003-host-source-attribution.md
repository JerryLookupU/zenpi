# Current Rust Host Source Attribution For Device Workflows

Provisional S2-051/S2-054/S2-070/S2-099 progress at
2026-10-03T19:24:45+0800. Previous goal turn was progress: host identity
nonblocking/type/alias safety changed Rust source and tests. This turn inspected
device harness preflight and artifact checks. APK provenance existed as a gate,
but recording a host binary hash did not prove it matched current Rust sources.
No nested agents, worker launch, checklist promotion, commit/push or cleanup.

## Implementation

stage2_host_build.py builds only the zenpi binary via offline locked Cargo and
uses structured compiler-artifact messages to require exactly the expected
target/debug/zenpi or target/release/zenpi executable. Inventory includes Cargo
manifest/lock, src, vendor, optional build/toolchain/Cargo config and builder
sources, including additions/deletions. Symlink input trees/files and binary
aliases are refused. The environment is narrowed to named basic shell/home/
locale/temp fields; compiler overrides are not inherited as environment vars.
Cargo/user configuration remains outside a hermetic release guarantee.

Before each build the tool invalidates its prior profile receipt. Failure,
unexpected/duplicate artifact or mid-build source change never publishes a new
receipt. Successful unchanged source and artifact hashes publish atomically
under target, alongside command/cwd/timestamps/output hashes/environment names.
This is local attribution, not signed attestation, fresh compilation of every
dependency or reproducible release evidence; Cargo incremental reuse is allowed.

Pair, desktop, headless and session Android harnesses now require current host
provenance after APK preflight and before any adb/device/child invocation.
They recheck it before recording success and store host_build_provenance beside
APK provenance. Scanner permission tests do not launch a host, so retain only
their actual APK dependency. Docs/runbook explain both guarded builds.

## Verification

Commands ran from /Users/mac/Github/zenpi.

- python3 tools/test_stage2_host_build.py: eight passed, exit 0. Covers current
  inventory, source/lock/config/vendor/artifact change, add/delete, malformed/
  failed receipt, symlink/unattributed path, failed build invalidation, wrong/
  duplicate compiler artifact, concurrent edits, controlled environment and
  all four device harnesses rejecting stale host before subprocesses.
- python3 tools/stage2_host_build.py: exit 0, real debug binary build and
  current-source receipt generated. --verify-only: exit 0, independently
  rereads current inventory/receipt/binary and matches their hashes below.
- Python existing session harness six, desktop fourteen, headless nineteen,
  pair five, APK builder ten, scanner six, host smoke seven and Blueprint
  fixture five tests pass, exit 0. Together with new host builder: 80 tests.
- git diff --check and strict Blueprint validator: exit 0, 36 unfinished.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. No device/LAN attempt or unchanged Gradle/ADB/TCP retry.
- target/debug/zenpi host pair --help: exit 1, current CLI reports
  "--help requires a value". This exploratory CLI call is not a passed gate;
  no listener was started. Build attribution does not claim CLI acceptance.

Rust/Android production source did not change this turn; no additional unit,
Compose or connected runtime pass claimed beyond real host compilation.
All required command handles are terminal.

## Digests

```text
tools/stage2_host_build.py 12fba13947abf83be2a3f0c98efb76bacfd090bee379f8e0474a3c6012aad350
tools/test_stage2_host_build.py 46a8feaff9741344a507a951e2e55ed70fad3ccb2367e279872d1a281d8db670
target/stage2-host-build-debug.json 86b92ecba0cbdd110773f2aeaf4b3fcabb5d0c377cfea773c0c5814d043233de
host source inventory b210999e55d2ce98f37b1cd6c2762aad2252a9adce64488b66ea325176ad6f6b
target/debug/zenpi 83726a362510611429c422f66d23e91fdd76fe025be498586a9e9b644f32e46a
```

Goal remains active. Current APK, physical camera/direct LAN, connected
UI/lifecycle, complete parity and signed reproducible release remain open.
An attributed binary is necessary evidence preparation, not whole-item or
end-to-end acceptance.
