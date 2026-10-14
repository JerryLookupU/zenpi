# Durable Quota Intent Recovery

Provisional S2-018/S2-019/S2-056 progress observed 2026-10-02T09:12Z.
Previous goal turn made progress: host-only runtime quota application. HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b unchanged; dirty user worktree
preserved. All 36 Blueprint rows remain [ ]. No nested agents, controller
worker admission, commit, push, cleanup or acceptance promotion.

## Implementation

WorkspaceReceipt now has an optional strict quota_intent with exact request,
before state and candidate state. Restore verifies digest, operation/arguments,
owner/project/session shape, integer 1..64 quota and exact single-revision
transition. A quota intent cannot coexist with a Git intent or a succeeded
result. Legacy checkpoints omit it without migration churn. The ordinary
SetConcurrency path checks all existing ACL/CAS/busy gates, durably reserves
identity before the final configuration commit and returns unknown rather than
failed/no-effect if that later commit fails. Reservation changes neither quota
nor revision. Exact remote retries of an unresolved receipt never replay;
changed content conflicts, and current ACL still gates receipt access.

Local exclusive --list-quota-intents and --reconcile-quota REQUEST --device
DEVICE modes run before identity/listener creation under explicit workspace/
existing-session attachment. Repair requires unchanged saved before-state,
session and workspace revision plus idle master/arch and no owned runtime
workers. It locks existing external slot leases through commit to fence another
host or detached writer missing from the new pool. Existing-only lease probes
create no paths, refuse live locks, symlink/noncanonical/nonprivate directories
and aliased lease files. Repair does not start workers or replay scheduler
commands: it explicitly accepts configuration for future startup. Result owner
is retained from the original request across restart. Newer state refuses old
repair. Git and quota unresolved intent listings are separate.

Four new pool tests cover real reservation durability/restart, duplicate and
wrong-candidate refusal, busy master/runtime/foreign runtime lease, stale intent,
pre-reservation and post-reservation checkpoint failure, strict corrupted
intent rejection, explicit idle repair and no master turns. New worker lease
test covers noncreating probes and live/hard-link/symlink/nonprivate refusal.
One new integration test reconstructs the exact persisted crash boundary,
checks remote unknown/CAS identity/ACL behavior and invokes the real production
binary's list/repair modes, then validates the original-owner success receipt
without workers, network identity or turns. Existing CLI policy test covers
exclusive quota actions and malformed combinations. This is not a live remote
resize or LAN/device fixture. Protocol and user guide describe the distinction.

## Verification

Cwd /Users/mac/Github/zenpi. Rust environment
ZENPI_HOME=/private/tmp/zenpi-quota-intent. Final commands exit 0:

- rtk cargo test --offline --locked --test remote_workspace
  --test remote_worktree_runner --test remote_worker_owner --test remote_concurrency
  --test remote_parity -- --test-threads=1: final 98 passed, five suites,
  25.64 seconds. Earlier same-suite run passed and is not counted twice.
- rtk cargo test --offline --locked --lib quota_intent_tests -- --test-threads=1:
  final 4 passed, 407 filtered, 0.27 seconds.
- rtk cargo test --offline --locked --test remote_tui_pairing
  host_cli_is_explicit -- --test-threads=1: 1 passed, 3 filtered, 0.00 seconds.
- rtk cargo test --offline --locked --test remote_migration --test remote_subtab
  --test remote_worktree --test runtime -- --test-threads=1:
  37 passed, four suites, 6.52 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  final 7 passed, 404 filtered, 0.23 seconds.
- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  final 3 passed, 408 filtered, 0.43 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host -- --test-threads=1:
  final 8 passed, 403 filtered, 0.69 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider
  --test-threads=1: 31 passed, two TCP fixtures excluded, 3.70 seconds.
- rtk cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: 103 passed, one TCP fixture excluded, 6.14 seconds.
- rtk cargo check --offline --locked: final zero errors, 20 existing warnings.
- env PYTHONPATH=tools python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.053 seconds.
- cargo fmt --all -- --check, git diff --check and
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: pass;
  strict validator reports 36 incomplete items.

Unique successful Rust cases: 292. Initial pool tests failed to compile due to
missing test-only Arc/Mutex imports; corrected before all final runs. All
observed process handles terminal; no timeout triggered a replacement run.
Android source unchanged this turn, no new JVM/Gradle/APK/device evidence.
No restricted TCP/Gradle/adb bypass. These provisional results do not replace
acceptance-grade exact stdout/stderr-hashed receipts.

SHA256:

```text
src/project_workspace.rs b0f0735db5f513acd5674923d418d3f258ebd6bd9ff70e2b958ca9a308017e3e
src/remote/subtab.rs ba4c511a9ef271c8ed8563c5f8278ab2cfd8e9aba3e8869dd06e1427748ac389
src/remote/worker.rs b91bca148855ce3e8ca32e036be53b3b27df223a2c945b92827f98452f800939
src/host_pair.rs 0a2c4232b993f0c8fe0d203caf1bd3778e4496b6b25b28019e2544725fff0a28
tests/remote_workspace.rs b1e78ac2d47087ff74984cefc6dd1c2081954a281fe97c3de35202aa6b4f4a46
tests/remote_tui_pairing.rs b1221f6cb6609db12af242be0108742d304d2393d6ba852ffdf121542ac80a5e
protocol 285aa912fe3306eba2be449a0432250a4e22946c2ea58a64ad296958afb66e8b
user-guide 6ee46965c18ee53a4800d3941a619649c525ab318d0cb92b8ae7f4985e8015bc
```

## Remaining Scope

Intent persistence is now present, but remote live resizing still needs runtime
application/rollback or unknown recovery, newly leased slots for growth and
synchronized prompt admission scope. The current remote path remains idle-only.
Local TUI worker controls, full event/transcript parity, settings and interactive
PTY remain unfinished. Current APK API31/34/35 camera QR, actual LAN desktop
sync, disconnect/replay/process lifecycle and reproducible signed release
remain unverified. Full objective remains active; no Blueprint item accepted.
