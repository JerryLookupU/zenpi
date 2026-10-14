# Reserved Independent Worker Growth

Provisional S2-018 progress observed 2026-10-02T09:18Z. Previous goal turn
made progress: durable quota intent and local recovery. HEAD remains
1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty user changes preserved.
All 36 authoritative Blueprint rows remain [ ]. No nested agents, execution
worker admission, commit, push, cleanup or acceptance promotion.

## Implementation

WorktreeRunner now holds a mutex-protected shared slot inventory. Executing
jobs clone only their independent worker handle and retain a slot guard that
returns availability on success/error/panic. Runtime claims are held by the
shared inventory and therefore survive detached jobs. Its host-only
grow_reserved method prepares new ownership only against an existing unresolved
quota intent, then appends slots without changing scheduler limit, durable
configuration, captured admission scope or request receipt.

ProjectOwnerPool validates device/request/digest, current before-state/session/
workspace/subtab revision, old configured capacity and the strictly validated
candidate root/quota. Master and arch owners must be idle. New slots count
against the shared 64-owner cap and use the normal private journal/lease path.
All owners and exclusive runtime claims are staged before publishing any new
handles. A partial failure drops staged claims/leases, preserves current runner
capacity and limit, and leaves the unresolved intent intact. Initialization
can leave private journals but admits no turns; no destructive rollback removes
those journals. A retry reopens history after the blocker is explicitly removed.

Four new tests cover actual running growth from one to three independent
Agent/Echo writers with a fourth job queued; prepared ownership cannot start
extra work before explicit scheduler acknowledgement. Foreign/missing/digest
and repeated/stale requests refuse preparation. A third-slot symlink failure
after staging the second slot leaves pool and runner at one; the original
runner still executes, and the corrected retry obtains all slots. Busy master
refusal and original checkpoint/scope invariants are checked. A newly grown
noncooperative writer retains claims and external leases after bounded detach;
a reopened pool cannot reconcile until actual thread exit. No master turns
or remote success receipt are fabricated.

## Verification

Cwd /Users/mac/Github/zenpi. Rust environment
ZENPI_HOME=/private/tmp/zenpi-quota-growth. Final commands exit 0:

- rtk cargo test --offline --locked --lib quota_intent_tests -- --test-threads=1:
  final 8 passed, 407 filtered, 0.88 seconds.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test remote_concurrency --test remote_parity --test runtime
  -- --test-threads=1: 113 passed, six suites, 28.53 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  7 passed, 407 filtered, 0.56 seconds.
- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  3 passed, 411 filtered, 0.99 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host -- --test-threads=1:
  8 passed, 406 filtered, 1.28 seconds.
- rtk cargo test --offline --locked --test remote_migration --test remote_subtab
  --test remote_worktree -- --test-threads=1: 22 passed, three suites, 6.48 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider
  --test-threads=1: 31 passed, two TCP fixtures excluded, 3.49 seconds.
- rtk cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: 103 passed, one TCP fixture excluded, 5.93 seconds.
- rtk cargo test --offline --locked --test remote_tui_pairing host_cli_is_explicit
  -- --test-threads=1: 1 passed, 3 filtered, 0.00 seconds.
- rtk cargo check --offline --locked: final zero errors, 20 existing warnings.
- env PYTHONPATH=tools python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.053 seconds.
- cargo fmt --all -- --check, git diff --check and
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: pass;
  strict checker reports 36 incomplete rows.

Unique successful Rust cases: 296; earlier focused 6/7-case pool runs not
counted twice. All observed sessions terminal; no timeout-triggered restart.
Fixtures use deterministic held jobs and real local Agent/Echo turns, not LAN
or HTTP. Android source unchanged; no new JVM/Gradle/APK/device evidence.
No restricted TCP/Gradle/adb bypass. Results remain provisional, not exact
acceptance-grade stdout/stderr-hashed receipts.

SHA256:

```text
src/project_workspace.rs a1a615f87203a3f7d604c98622306664defa77b0dabfc4b524219a5a6de9ba26
src/remote/worktree_runner.rs 8ee42318367758ad925ab820299d110d8a3c783769915e662752391faf27dddf
protocol 670a3d44e88816eae244c04bd06c87af10a69508ac85b8bfe6850b16724064e3
blueprint d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
specification 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
```

## Remaining Scope

Growth preparation is now available, but live application/configuration
settlement, rollback or unknown recovery and prompt-admission scope updates
still need integration before Android live-resize effective receipts. Remote
SetConcurrency remains idle-only. Local TUI worker controls, complete event/
transcript parity, settings and interactive PTY remain unfinished. Current APK
API31/34/35 camera QR, actual LAN desktop sync, disconnect/replay/process
lifecycle and reproducible signed release remain unverified. Full goal remains
active and no Blueprint item is accepted.
