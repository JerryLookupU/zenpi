# Durable Worker Output Binding

Provisional S2-019/S2-020/S2-036 progress observed 2026-10-02T10:32Z.
Previous goal turn made progress (worker output pages). Current worktree
inspected; HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. User changes
preserved. No nested agents, execution-worker admission, commits, pushes,
cleanup or Blueprint acceptance promotion.

## Implementation And Evidence

Output paging previously found an assistant turn by ID in a derived worker
journal, without requiring that journal to attest to the originating task.
The actual observed worker pipeline now persists remote_worker_output_v1 after
execution and before successful return. It binds original device/request/full
Envelope fingerprint, project/master session/subtab, actual worker session,
derived slot key, final assistant turn and raw-content SHA256. Persistence
failure propagates instead of returning a successfully correlated output.

The master progress event stores request fingerprint and output SHA256 as
internal metadata. TranscriptRead requires exactly one matching worker binding
and one assistant turn, verifies every scope field and slot key, and checks the
content hash against both worker and master evidence. Missing or duplicated
bindings, changed scope and replaced content fail closed. Live and durable
status projections exclude the internal metadata, keeping the Android wire
status schema unchanged. Old journals without dual evidence are not guessed or
automatically migrated. This detects mismatched/replaced worker content, not a
malicious local operator able to rewrite both journals.

The existing end-to-end local production pipeline test now rewrites each of ten
binding fields independently, replaces the answer, replaces both answer and
worker-side content digest while leaving the master unchanged, and appends a
duplicate binding. Each read is refused, then restoration returns the original
page. Retirement, restore, per-device ACL, digest continuity, redaction and
read-only/no-worker behavior remain covered.

Worker read size enforcement now reaches SessionStore's descriptor reader.
The new crate-private bounded readonly entry preserves the general reader's
existing default while enforcing the smaller cap both on opened-file metadata
and max+1 bytes read. A new test accepts exactly the journal length, rejects a
one-byte-smaller allowance and zero allowance, and preserves bytes/absence.
This closes the gap where growth after a preceding 8 MiB metadata check could
still be read using the generic 256 MiB limit.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-worker-binding.
Completed commands exit 0:

- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  final source 10 passed, 419 filtered, 4.79 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  final source 8 passed, 421 filtered, 0.39 seconds. This substring filter includes
  three worker tests, one worker_admission test and four worker_progress tests;
  separate worker_progress runs are not extra unique cases.
- rtk cargo test --offline --locked --lib remote::worker_progress -- --test-threads=1:
  final source 4 passed, 425 filtered, 0.17 seconds.
- rtk cargo test --offline --locked --lib remote::transcript -- --test-threads=1:
  2 passed, 427 filtered, 1.26 seconds.
- rtk cargo test --offline --locked --lib new_session_audit_recovery_tests -- --test-threads=1:
  final source 4 passed, 425 filtered, 0.11 seconds.
- rtk cargo test --offline --locked --test session_recovery --test session_cli
  --test core_session --test stage1_session_tree --test stage1_context_checkpoint
  --test session_new --test session_maintenance_owner -- --test-threads=1:
  reported 132 passed, 2 ignored, 26 filtered, nine suites, 17.22 seconds.
  Two ignored helpers are launched by their parent process tests. RTK summary
  includes child suite reporting; not an assertion of 132 unique parent cases.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test remote_concurrency --test remote_parity --test runtime
  --test remote_migration --test remote_subtab --test remote_worktree --test remote_events
  -- --test-threads=1: 143 passed, ten suites, 46.48 seconds before the final
  duplicate-binding/master-digest checks. The final focused rerun below covers
  shared worker execution after those changes.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner -- --test-threads=1: final source 83 passed, three
  suites, 28.42 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel --test-threads=1:
  134 passed, three TCP fixtures excluded, seven suites, 9.99 seconds before
  final internal master-digest metadata addition; final lib tests cover that.
- rtk cargo check --offline --locked: final source zero errors, 20 existing warnings.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.311 seconds.
- cargo fmt --all --check; git diff --check; /usr/bin/python3
  tools/validate_stage2_android2unix_blueprint.py --strict: pass, all 36 incomplete.

A new test digest call initially lacked its Digest trait import, producing a
compile error. Added import and observed that handle terminal before the final
passing rerun. No observation timeout started duplicate work. All command
handles terminal. Android source unchanged; no new JVM/device/build claim.
No restricted TCP/Gradle/adb bypass. Provisional evidence, not acceptance-grade
stdout/stderr-hashed receipts.

SHA256:

```text
src/session.rs 355e352427b5e845b0577fa28b2f378e5a784dc4b56e604c5f255565cdd99fab
src/remote/worktree_runner.rs 636831dfad0c3ea8d7d30624b2137120c3134a3106e4a10f32956ef0d9f9b182
src/remote/worker.rs 2a576a861b376cf7ae672a42386d7f0abb54dda3632a93382fe22b64930a350f
src/remote/worker_progress.rs 9c9a294427e1340cb7b261f1021fe45e287836407bce7c192d8fc02cdbcf234c
src/remote/transcript.rs 5f70f31cdd3ce0415bd24ac52101eb1c573f21cbe764cba5ac788e6f1904fcf9
src/remote/worktree_host.rs caee667043f8c152ba75455272a1117b3174562586d7b76bf119ba1f99ccfe8f
protocol 7aa0bca7ef50fe1fa6623b81acbe87e63ba27330b7063926a3b7f51fbf0adac5
```

## Remaining Scope

Final output binding is not full worker history/event/cursor parity, legacy
migration or device/LAN acceptance. Local TUI worker controls, settings and
interactive PTY remain unfinished. Actual API31/34/35 camera QR/LAN desktop
sync, disconnect/replay/process lifecycle and reproducible signed release
remain unverified. Goal active; no Blueprint item accepted.
