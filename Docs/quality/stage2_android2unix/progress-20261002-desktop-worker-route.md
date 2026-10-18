# Desktop Remote Worktree Route

Provisional S2-018/S2-019/S2-036 progress at 2026-10-02T08:28Z. Previous goal
turn made progress: Android worktree task client. HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty worktree preserved. All 36
authoritative rows remain [ ]. No nested agents, controller worker admission,
commit, push, cleanup or acceptance promotion. Full goal active.

## Implementation

ProjectRuntimeHost now owns the same lazy WorktreeHost as HeadlessHost, instead
of a second scheduler/session state machine. Shared remote dispatch routes
subtab Prompt, worker Cancel and durable cancellation-receipt queries. Desktop
listing advertises worker_control under current ACL/checkpoint and publishes
device-scoped worker_statuses. Exact old receipts remain readable while new
local master/session/input admission is fenced.

The actual production TUI loop polls worker lanes before owner mailbox work.
It dispatches worker Prompt before the master-only prompt adapter, worker Cancel
before master cancellation, and switches the old direct Approve branch to the
shared master/worker router. That previous branch would otherwise shadow the
HostAdapter approval integration. Master prompt/model admission is busy while
the same project's worker lanes exist. Queued local input is deferred while
workers own its project; new local input is restored with a busy notice rather
than executed against the same project. The draft and master transcript are
not populated with the remote worker prompt.

Shutdown signals worker lanes before joining the master and uses the shared
bounded worker shutdown path before pool close. Terminal restoration remains
before provider joins. ProjectRuntimeHost Drop also shuts down retained worker
lanes for callers outside the full terminal loop. Worker closure lease/runtime
claim semantics remain unchanged for noncooperative detached work.

One new integration test uses actual ProjectRuntimeHost dispatch/polling and
independent Echo worker Agents. It checks read-only refusal and admitted-local
work refusal without worker creation; actual worker success; read-only control
false/device-scoped status absence; unchanged local draft and empty master
turns; exact receipts under admission fences; desktop -> headless -> desktop
restart receipt compatibility without worker startup or file byte changes;
and revoked-device receipt denial. This is a production owner-domain test,
not a full terminal/TLS/device-camera run.

## Verification

Cwd /Users/mac/Github/zenpi. Rust environment
ZENPI_HOME=/private/tmp/zenpi-desktop-worktree. Successful commands exit 0:

- rtk cargo test --offline --locked --test remote_workspace
  --test remote_worker_owner --test remote_worktree_runner
  --test remote_concurrency --test remote_parity -- --test-threads=1:
  92 passed, five suites (55/12/9/11/5), 25.78 seconds.
- rtk cargo test --offline --locked --test remote_workspace
  -- --test-threads=1: final 55 passed after expanded desktop restart/revoke
  assertions, 19.46 seconds. Rerun not added to unique totals.
- cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: 103 passed (19/28/56), one restricted TCP fixture excluded.
- rtk cargo test --offline --locked --test remote_auth
  --test remote_approval_wakeup -- --test-threads=1: 13 passed, 0.73 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  5 passed, 398 filtered, 0.32 seconds.
- rtk cargo test --offline --locked --lib remote::worktree_host
  -- --test-threads=1: 1 passed, 402 filtered, 0.26 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host
  -- --test-threads=1: 8 passed, 395 filtered, 0.76 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit
  --skip production_plain_host_routes_authenticated_tls_prompt_to_the_provider
  --test-threads=1: 8 passed, two restricted TCP fixtures excluded, 2.62 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  63 passed. cargo fmt --all -- --check, git diff --check and strict Blueprint
  validator pass. Validator still reports 36 incomplete items.

Unique Rust cases: 230. First cargo check exposed a mutable/immutable borrow
overlap at prompt admission; compute busy before borrowing the pool and rerun
successfully. First focused integration also passed, not added to totals.
All live verification handles were observed to completion. Build lock waiting
was preserved, not mistaken for terminal execution or restarted on timeout.
No Android source changed or current APK/device claims this turn.

SHA256 at verification:

```text
src/tui.rs b4b704332a2833ad912b3e73ab9a2920c269088a93c736cc5a44b61e03eec3da
src/remote/mod.rs 7bd4ea4d0aa25b57712090fb7b462074a9b11ef7736a1f66180bf5f6fcf56040
tests/remote_workspace.rs caa34be8e763f979eec2ebd4560626292d2e55ace77e71399e8c09f577a97776
Docs/stage2_android2unix_protocol.md 9195eccbe08b53e36fcc605b97e402cf959e9bc40a3222244ff38b46c25496cb
```

## Remaining Scope

Local TUI worker submission/cancel/status and exact worker approval owner UI
are not implemented here. Existing local approval presentation enumerates
master/arch controls, not independent workers; phone worker approval still
uses the shared remote route. Worker transcript/provider streaming, effective
live quota resize/stepper, full settings and interactive PTY parity remain.
Current APK API31/34/35 camera QR, same-LAN desktop sync, disconnect/replay and
process lifecycle plus reproducible signed release remain unfinished or
unverified. No restricted TCP/Gradle/adb bypass. This is provisional evidence,
not an acceptance receipt with captured stdout/stderr hashes. Full goal active.
