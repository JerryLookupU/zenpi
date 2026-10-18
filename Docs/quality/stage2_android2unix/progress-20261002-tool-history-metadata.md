# Task History Tool Invocation Metadata

Provisional S2-019/S2-020/S2-036 progress at 2026-10-02T11:18:29Z.
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; current dirty worktree
inspected and user changes preserved. Previous turn verified RTK availability,
not product progress. This turn changes the actual host/Android history path.
No nested agents, worker admission, commit, push, cleanup or acceptance marks.

## Implementation

Observed worker manifests now bind turn metadata SHA256 as well as content,
ID and role. The master-confirmed manifest digest covers that additional hash.
History and selected-message reads reject edited metadata, including an attack
that rehashes the worker manifest while retaining the original master evidence.
Old manifests without metadata hashes fail closed for history; no migration or
automatic rewriting is claimed.

Assistant history previews expose optional tool_calls/tool_calls_truncated.
Only invocation ID, tool name, redacted serialized argument preview and its
truncation flag are projected. Limits: eight calls, 128-byte ID, 64-byte name,
512-byte UTF-8 argument preview, 64 KiB raw encoded arguments. Execution,
policy, worker binding, annotations and model evidence are excluded. Structured
redaction precedes UTF-8 slicing. Duplicate call IDs are refused.

Encoded page size is checked after JSON escaping, reserving 4096 bytes below
the 256 KiB control cap for scope/envelope fields. Pages can contain fewer than
16 turns and resume without losing entries. A regression uses maximally escaped
content and eight invocation previews per message to require multi-page output.

Android accepts exactly the old four preview fields or the new paired optional
fields; rejects malformed values, extra execution fields, non-assistant calls,
duplicate IDs and byte-limit violations. History shows tool names and selectable
monospace previews even when assistant text is empty. Full argument paging,
live tool replay and failed/cancelled partial history are not implemented here.

## Verification

Cwd /Users/mac/Github/zenpi. Rust ZENPI_HOME=/private/tmp/zenpi-tool-history-home.
Final completed commands exit 0:

- rtk cargo test --offline --locked --lib remote::transcript -- --test-threads=1:
  five passed, 429 filtered, 1.81 seconds. Exact argument cap admitted; oversized
  refused; UTF-8 redaction/truncation, allowlist and escaped page continuity.
- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  twelve passed, 422 filtered, 5.23 seconds. Actual controlled-backend read_file
  loop exposes the correct call and file, rejects metadata-only tampering and
  forged worker metadata hashes, and restores reads after fixture restoration.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  eight passed, 426 filtered, 0.32 seconds (includes progress/admission tests).
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test core_session --test runtime --test remote_parity
  -- --test-threads=1: 142 passed, six suites, 33.43 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- /bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-tool-history:
  production/unit/instrumented Kotlin source compiled against cached API35;
  246 JVM tests passed, 1.182 seconds. Compose test for an empty assistant/tool
  invocation at 320 dp compiled, not executed on a device. Existing warnings.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.061 seconds.
- cargo fmt --all --check; git diff --check; python3
  tools/validate_stage2_android2unix_blueprint.py --strict: pass; all 36 unfinished.

Initial redaction fixture used generic token=, not a supported conventional
secret marker; the test failed. Corrected it to access_token= to exercise the
existing documented redactor. The failed run is not passing evidence and no
claim of recognizing arbitrary secret names is made. No TCP/Gradle/adb retry or
permission bypass. All observed process handles terminal. Evidence is
provisional, not an acceptance receipt with captured stdout/stderr hashes.

SHA256 source:

```text
src/remote/transcript.rs c0dfedd4a5a1814508eae5c28def16edc8e02d9a08025025277ff636106ad7bb
src/remote/worktree_runner.rs 295e67e7c79e0909be72d21b42425bf6666188b9e5695bcd2aec0fe100a33fe7
src/remote/worktree_host.rs 375cc74a498344271b4cf0569afda6592f68240e8d98ef777566b5402bb1126e
android/app/src/main/java/zenpi/transport/HostClient.kt d16b809d07a6ade7bdad17444214a9722618f759799d5c677f515b800a64f4a2
android/app/src/main/java/zenpi/transport/WorkerHistoryCodec.kt f6c4a192ce4f598347ade12035b66a21d9e5de614bdc1d6cd2d416f509d332de
android/app/src/main/java/zenpi/MainActivity.kt 6836bde69f9f4939c958465c0c4c8e97ed9385ea344cc014bcfc7212e21dfe25
android/app/src/test/java/zenpi/WorkerHistoryTest.kt 333966a55d79518852f8515e5c46c6e8005dd4faf1a3416a48cabb7b0dc16262
android/app/src/androidTest/java/zenpi/WorkerScreenTest.kt 4f1fa5b69b34c3e2077accf1d7ca1fc7726b2c4e15608768414856b2e148450c
```

Compiled artifacts under /private/tmp/zenpi-tool-history:

```text
main.jar 7ab7bc4458be155fabe8a8503e8343cf06ec885be986c4f0dd594a226ea7d2ba
unit.jar c4629f523ead3acb8964e2257ecf564517b14b66ea4863cbe8e52e8578b594a3
instrumented.jar 7dcd5720a83072c1664455bc6e69465113e62400d7c3a294b38ce3c486bd2095
```

## Remaining Gates

No full-goal completion claim. API31/34/35 real camera QR-to-LAN desktop sync,
network disconnect/replay/address changes, process lifecycle, signed APK and
reproducible release remain unverified. Full tool-argument paging, worker live
events/partial failed history, local TUI worker controls, settings and interactive
PTY parity remain unfinished. No authoritative checklist row is promoted.
