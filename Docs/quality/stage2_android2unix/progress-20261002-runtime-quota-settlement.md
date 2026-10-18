# Runtime Quota Settlement

Provisional S2-018/S2-019 progress observed 2026-10-02T09:24Z. Previous goal
turn made progress: intent-bound worker growth. HEAD remains
1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty user worktree preserved.
All 36 Blueprint rows remain [ ]. No nested agents, execution-worker admission,
commit, push, cleanup or acceptance promotion.

## Implementation

Host-only begin_reserved_quota validates the exact unresolved intent and current
configured scope, prepares additional independent slots if needed, submits a
scheduler quota command and retains its application receipt. It fences new
task submission, direct quota commands, duplicate begin and further growth.
poll_reserved_quota does not consume worker events: it waits for the exact
application acknowledgement, checks the last scheduler-applied limit, commits
the matching candidate/receipt and only then advances configured quota and
subtab admission revision. Retained ownership capacity is separate.

The shared checkpoint commit has a restricted runtime-quota path: validated
quota-only intent plus identical runner worker handles permit that runner's
runtime-owned slots to remain executing. Foreign owners still block, master/
arch must be idle, ordinary subtab commits retain their original busy fence.
Lowered capacity's executing owners are not closed or removed at commit;
their claims remain through runtime shutdown. The original six-field receipt
and owner are atomically persisted with configuration. Worker admission now
checks configured quota instead of retained capacity. Progress tracking accepts
only monotonic revisions for the same runner instance/project/session/subtab;
tracked request/device identities remain unchanged.

An unapplied/backpressured command cannot commit. A post-application checkpoint
conflict retains the unresolved receipt, old configured scope and task fence;
it cannot be overwritten by another runtime quota command. Shutdown then
explicit local intent recovery remains available. Command rejection before
application does not create a succeeded receipt. The production remote path
remains idle-only: remote ACL lifetime, pending/effective wire receipts and
Android running-state controls are not integrated by these host APIs.

Three new pool tests cover a held real Agent/Echo turn, backpressured application,
submission fences, atomic growth settlement, lower while running, re-raise
within retained capacity and further growth to four, monotonic scope updates,
duplicate settled identity refusal, later task admission, checkpoint failure
after application with restart/local repair, and shutdown before application.
Existing tracker test now refuses revision regression/session/instance changes.
No LAN, provider HTTP or Android-device claim.

## Verification

Cwd /Users/mac/Github/zenpi; Rust environment
ZENPI_HOME=/private/tmp/zenpi-quota-settle. Final commands exit 0:

- rtk cargo test --offline --locked --lib quota_intent_tests -- --test-threads=1:
  final 11 passed, 407 filtered, 1.29 seconds.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test remote_concurrency --test remote_parity --test runtime
  -- --test-threads=1: 113 passed, six suites, 27.24 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  7 passed, 410 filtered, 0.33 seconds.
- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  3 passed, 415 filtered, 0.43 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host -- --test-threads=1:
  8 passed, 410 filtered, 0.71 seconds.
- rtk cargo test --offline --locked --test remote_migration --test remote_subtab
  --test remote_worktree -- --test-threads=1: 22 passed, three suites, 7.06 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider
  --test-threads=1: 31 passed, two TCP fixtures excluded, 3.56 seconds.
- rtk cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: 103 passed, one TCP fixture excluded, 5.98 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- env PYTHONPATH=tools python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.052 seconds.
- cargo fmt --all -- --check, git diff --check and
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: pass;
  strict validator reports 36 incomplete items.

Unique successful Rust cases: 298. Focused/repeated pool tests counted once;
the previous turn's separate CLI case was not rerun and is not included.
All observed process sessions terminal; no timeout restarted work. Android
source unchanged, no new JVM/Gradle/APK/device evidence. No restricted
TCP/Gradle/adb bypass. These are provisional results, not acceptance-grade
exact stdout/stderr-hashed receipts.

SHA256:

```text
src/project_workspace.rs 5ecc1d0b8d50af5b303e6a3ae99c6837472868f560c40ea6d024618104c9788f
src/remote/worktree_runner.rs 44d4e3c9ef1077eee08ca832a86175c4a6ca2df313956f5338e2c180b1f1704c
src/remote/worker_admission.rs 993b4e027e17cbdf1197d8ac1d7cf2e4b1a10309049b3d3252ccdd816f54115a
src/remote/worker_progress.rs 792b6337b78b0256e3ccae3810e309a39a28872a6c8814a1fbf60b3a069da3b9
src/remote/worktree_host.rs 4d84ac00d7796210f507c7772810abb531643d9014ae83da699f74de3ee0d2ef
protocol ad7c04d1cfc64308a6818ee2307b7bb89fad8d3f9e3c0ceae1cc04f1f8452bea
```

## Remaining Scope

Remote controller live-resize ACL/retry/status integration and Android effective
receipt/running controls remain unfinished, despite the new host settlement
path. Local TUI worker controls, full event/transcript parity, settings and
interactive PTY remain incomplete. Current APK API31/34/35 camera QR, actual
LAN desktop sync, disconnect/replay/process lifecycle and reproducible signed
release remain unverified. Full goal active, no Blueprint item accepted.
