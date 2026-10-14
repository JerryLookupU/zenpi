# Cross-Lane Quota Serialization

Provisional S2-018/S2-019 progress observed 2026-10-02T10:02Z. Previous
development goal turn made progress (sibling quota scope); intervening RTK
inspection verified local tools but did not change product state. Current
worktree inspected; HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b.
User changes preserved. No nested agents, worker admission, commits, pushes,
cleanup or Blueprint acceptance promotion.

## Implementation

An unresolved quota reservation does not advance the global workspace revision.
Previously another live lane could reserve the same revision, apply its runtime
command, then become permanently uncommittable after the first reservation
settled. The pool now refuses any new quota identity while a durable unresolved
quota intent exists, including another project. Live-lane and idle-subtab
adapters expose Busy before reservation, worker preparation or scheduler
submission. Exact receipt queries and changed-content identity conflicts still
precede this guard. Runtime settlement continues to require the original
receipt and exact workspace/subtab before-state; no CAS was weakened.

An unresolved request survives restart and device revocation. Another device
cannot take over its revision just because the original device lost permission.
An authorized exact settlement or explicit local stopped-host reconciliation
releases the reservation. Existing sibling execution is not stopped.

The new pool test covers a second device and a second project before and after
checkpoint restore, unchanged checkpoint bytes, original null receipt,
explicit repair and fresh-revision subsequent reservation. The controller
sibling test now forces backpressure on two actual independent Agent/Echo
writers: the first quota remains unknown, a second-device sibling quota is
Busy through both adapters with no checkpoint/capacity/pending change, then
both quotas settle sequentially and four tasks finish. The revoked-device
test also checks that a replacement authorized device cannot bypass the
unresolved reservation. These are local fixtures, not LAN/device acceptance.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-quota-serialization.
Completed commands exit 0:

- cargo test --offline --locked --lib quota_intent_tests -- --test-threads=1:
  13 passed, 412 filtered, 1.51 seconds (final cross-project source).
- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  8 passed, 417 filtered, 2.03 seconds (final revoked-device source).
- rtk cargo test --offline --locked --lib remote::headless_host -- --test-threads=1:
  8 passed, 417 filtered, 1.01 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  7 passed, 418 filtered, 0.28 seconds.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test remote_concurrency --test remote_parity --test runtime
  --test remote_subtab --test remote_worktree --test remote_migration -- --test-threads=1:
  135 passed, nine suites, 34.18 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel --test-threads=1:
  134 passed, three TCP fixtures filtered, seven suites, 9.56 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.063 seconds.
- cargo fmt --all --check; git diff --check; /usr/bin/python3
  tools/validate_stage2_android2unix_blueprint.py --strict: pass. All 36 rows incomplete.

Unique successful Rust cases: 305. All observed command handles are terminal;
no observation timeout triggered a duplicate execution. Android source
unchanged; no new JVM/Gradle/APK/device claim. No restricted
TCP/Gradle/adb bypass. Evidence is provisional, not an acceptance-grade
stdout/stderr-hashed receipt. A process inventory command was denied by the
sandbox; execution terminal states are observed via their actual tool handles.

SHA256:

```text
src/project_workspace.rs 0d48cb54ff95e5676610b87c87dfa2a46ca41bef5b3e942963c9d3b8eab00d9b
src/remote/subtab.rs 499c3c82f1c79360653c47fae24087b67cf2e9a4847e90517f0ec21d112335ae
src/remote/worktree_host.rs 23b58c953932d8b0a344d49fdb27a02d060862549bb0d791a1f999c1930d22f0
protocol b99c9730a868fd3dea94e98a42e4b4ce15c9d4ee54a26a419dccf880afa20f92
```

## Remaining Scope

Global 64-owner boundary, local TUI worker controls, full worker event/transcript
parity, settings and interactive PTY remain unfinished. Unrelated workspace
mutations may still invalidate an unresolved quota, requiring explicit operator
handling; this change serializes quota identities, not all workspace operations.
Actual API31/34/35 camera QR, LAN desktop synchronization, disconnect/replay/
process lifecycle and reproducible signed release remain unverified. Goal active;
no item accepted.
