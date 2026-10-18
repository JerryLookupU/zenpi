# Sibling Worktree Quota Scope

Provisional S2-018/S2-019 progress observed 2026-10-02T09:51Z. Previous goal
turn made progress: live-quota lifecycle regression. Current worktree inspected;
HEAD remains 1a8ff1acf72e0816fad503a05df2b1d18e07361b, user changes preserved.
All 36 Blueprint rows remain [ ]. No nested agents, execution-worker admission,
commit, push, cleanup or acceptance promotion.

## Fix And Evidence

Inspection found that a live quota commit blocked on any sibling worker Agent,
even when the quota intent changed only its target tab. Once one tab committed,
the project's subtab revision also made the other unchanged live runner stale;
fresh requests could remain refused until it finished. This contradicted the
independent worktree execution contract.

The runtime-quota commit now skips sibling execution locks only for same-master-
session owners whose entire before/candidate tab is unchanged and whose captured
root/kind agrees with that tab. Target workers still require identical runner
claims; changed/foreign-session owners retain existing fences. No sibling writer
is closed or retired through this exception. The exact intent's full before-state
and workspace CAS remain authoritative.

WorktreeRunner refresh_unchanged_scope advances project-wide revision only if
its master session, stable tab, validated canonical captured root/kind and
configured quota still match, and revision does not regress. Pending quota
owns its captured revision and refuses refresh. WorktreeHost refreshes before
fresh prompt/quota admission; old exact receipt queries still use existing
deduplication semantics. It does not skip request revision checks.

One new controller test runs a main and contained directory lane simultaneously,
then changes each quota while both original writers are held. Fresh sibling
tasks remain admitted under the newly committed revision; four independent
owners and four successful requests are verified, followed by retirement and
empty master turns. A new pool test injects changed quota/root/kind, removed
tab, session change and revision regression, checking refusal without captured
revision advancement; a pending quota also refuses refresh. These are local
real Agent/Echo fixtures, not LAN/provider HTTP or device evidence.

## Verification

Cwd /Users/mac/Github/zenpi; Rust environment ZENPI_HOME=/private/tmp/zenpi-sibling-quota.
Completed commands exit 0:

- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  8 passed, 415 filtered, 2.18 seconds.
- rtk cargo test --offline --locked --lib quota_intent_tests -- --test-threads=1:
  12 passed, 412 filtered, 1.42 seconds.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test remote_concurrency --test remote_parity --test runtime
  -- --test-threads=1: 113 passed, six suites, 27.43 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  7 passed, 417 filtered, 0.33 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host -- --test-threads=1:
  8 passed, 416 filtered, 0.81 seconds.
- rtk cargo test --offline --locked --test remote_migration --test remote_subtab
  --test remote_worktree -- --test-threads=1: 22 passed, three suites, 6.94 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider
  --test-threads=1: 31 passed, two TCP fixtures excluded, 3.56 seconds.
- rtk cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: 103 passed, one TCP fixture excluded, 6.11 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.103 seconds.
- Direct toolchain cargo fmt --all -- --check, git diff --check and
  /usr/bin/python3 tools/validate_stage2_android2unix_blueprint.py --strict pass;
  strict checker reports 36 incomplete items.

Unique successful Rust cases: 304. All observed handles terminal, no timeout
triggered duplicate execution. Android source unchanged; no new JVM/Gradle/APK/
device claim. No restricted TCP/Gradle/adb bypass. Results remain provisional,
not acceptance-grade exact stdout/stderr-hashed receipts.

SHA256:

```text
src/project_workspace.rs 153a5b13bb3d9f516e9a586dc7728599db4543ad9e0b99fe0e22d9f826d89af9
src/remote/worktree_runner.rs 7352ca947651715843aed00f6edf9a7b42cb374a548494b8ff2aaa5ed4dcf49b
src/remote/worktree_host.rs 1f0c999743d06887f18bf9ff5c92d6b6510536505ecc6a06e864033f8e27258c
protocol 71979415770c8ad1f57ebb45caa1f56eae79c6ae66f16c854c86196cfe9b0b6f
```

## Remaining Scope

This repairs same-project sibling operation, not full 64-owner, simultaneous
intent, device/API or release acceptance. Local TUI worker controls, full event/
transcript parity, settings and interactive PTY remain unfinished. Current APK
API31/34/35 camera QR, actual LAN desktop sync, disconnect/replay/process
lifecycle and reproducible signed release remain unverified. Full objective
active; no Blueprint item accepted.
