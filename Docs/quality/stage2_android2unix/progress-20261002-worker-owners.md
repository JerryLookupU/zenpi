# Independent Worktree Worker Owners

Provisional S2-018/S2-019 progress observed 2026-10-02T07:05:09Z.
Previous goal turn made progress: real bounded parallel runtime scheduling.
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty worktree preserved.
All 36 authoritative rows remain [ ]; full goal active. No worker admission,
subagents, commit, push, cleanup or acceptance promotion.

## Implementation

New remote/worker.rs owns independent WorkerOwner handles. ProjectOwnerPool
prepares a slot from its durable host subtab state, refusing unknown scope,
unavailable roots, slot outside the configured 1..64 quota, stale master
session, held/preparing master or more than 64 retained owners across the pool.
Each worker resolves normal selected-root project configuration/resources/
extensions using the existing Agent preparation API and explicit host auto
approval policy. It has a distinct Agent, input port, approval coordinator and
journal; no master Agent/backend/session writer is cloned.

The journal key hashes version/project/master-session/stable-subtab/slot.
Selection and rename preserve the writer identity. Worker journals and leases
are below the master's .zenpi/workers directory, covered by existing remote
file/diff/attachment exclusions. The workers directory is canonical/private/
user-owned; its .zenpi parent may retain ordinary non-writable read permissions
for compatibility with existing project config. Journals and lease files must
be private regular files without symlink or hard-link aliases. A lifetime flock
lease refuses duplicate writers across pools and processes before Agent
preparation. Every cloned WorkerOwner retains the lease, including when its
pool drops or its runtime outlives the owner thread.

Existing journals are checked for selected-workspace identity before owner
configuration can append anything. Restart preserves session identity and
history, with normal extension-selection startup events allowed; completed
turns are never automatically replayed. Permission/path checks reject the
tested preexisting substitutions. These checks are not a proof against every
malicious same-user concurrent parent-directory replacement race.

Subtab checkpoint commit now fences worker execution and closes removed or
out-of-quota slots only after durable compare-and-replace succeeds. Project
publication fences/retire workers when removing/replacing their master.
Failed checkpoint commits retain the original owner and checkpoint state.
Remote subtab Git actions check/hold worker guards before destructive work.
New/resume master-session operations refuse while retained worker owners exist;
an explicit retire_worker_owners method closes all idle workers atomically or
changes nothing if one is busy. External closed handles retain leases until
dropped, never permission to resubmit. close_all also closes worker Agents.
Worker control handles are separately accessible without acquiring their
execution locks; existing master control routing remains unchanged.

## Verification

Cwd /Users/mac/Github/zenpi. Final tests use
ZENPI_HOME=/private/tmp/zenpi-worker-owner-final; all following commands exit 0:

- rtk cargo test --offline --locked --test remote_worker_owner
  --test remote_concurrency --test remote_subtab --test remote_worktree
  --test remote_workspace --test remote_parity: **83 tests**, 6 suites,
  11.94 seconds. The subsequently added remote busy-worker test was verified
  by rtk cargo test --offline --locked --test remote_workspace: **40 tests**,
  6.78 seconds, replacing its prior 39-test suite result (84 unique total).
- rtk cargo test --offline --locked --test remote_worker_owner
  -- --test-threads=1: **12 tests**, 3.57 seconds; not added to unique totals.
- rtk cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel:
  **103 tests**, 1 restricted TCP fixture deliberately excluded.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit
  --skip production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  **8 tests**, 2 restricted TCP fixtures deliberately excluded.
- rtk cargo test --offline --locked --lib `remote::headless_host::`:
  **8 tests**, 389 filtered out. Unique Rust total across above: **203**.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  **63 tests**. cargo fmt --all -- --check, git diff --check and python3
  tools/validate_stage2_android2unix_blueprint.py --strict pass. Strict checker
  reports 36 incomplete rows, not behavioral item acceptance.

Twelve new worker tests cover isolated parallel turns, master journal unchanged,
restart identity/history, quota/stable selection, busy/failed-CAS nonmutation,
retirement and closed external handles, busy project close, aliases/private
directory/wrong-workspace refusal, normal auto policy, exactly 64 prepared
owners and refusal before creating a 65th journal, plus an independent child
process proving lease exclusion before last-handle release and admission after.
The new authenticated remote subtab test verifies Busy without checkpoint
mutation, successful later commit and exact receipt query while worker held.

During implementation an initial field insertion matched NewSessionPlan
instead of ProjectOwnerPool; compilation exposed this and it was corrected.
Initial tests exposed an overly strict byte-equality restart assertion (normal
extension startup appends evidence) and a missing checkpoint in the new remote
test. Final tests use prefix-preservation/history checks and explicitly commit
the initial subtab state. Earlier failures are not counted as passing gates.

SHA256 at final verification:

```text
src/remote/worker.rs 3b419a014f97a24c0c74f9f3c2206b1685edd4245f589ff1b9f8d3a8be69880e
src/project_workspace.rs b2d428cbbf20f0f77b8715defb163bad71911e2fdc59b8148cf0adefea1a298f
src/remote/subtab.rs fad753e8bf99385283ffd67375d4a85eabd9184c89e60e15b26448adae20e09a
tests/remote_worker_owner.rs 59701f357a92441246e63d6b1c6a18131eb3356756981f8f2c8fb4292bbbcfe6
tests/remote_workspace.rs 70d396574ffddc002db037eebc6fed7ccb9956251fbdbb6b189f3bc7819552c1
Docs/stage2_android2unix_protocol.md 5a4ef89817398c703f0b3cec7f0d72fce36765a13a32b961de2d79d1136d8db5
```

## Residual Scope

This provides independent host worker ownership, not finished per-worktree
scheduling. Existing remote prompts remain discussion-owner turns. Need typed
worker admission/routing, worker approval/event/status multiplexing and durable
request binding, effective quota application/receipt reconciliation, restart
admission, TUI/headless/Android parity and Android stepper. Current listing
concurrency_control stays false. No S2-018 acceptance or production host worker
provider/LAN behavior claim from deterministic Echo fixture tests.

Native Android camera QR, real same-LAN desktop synchronization, current APK/
API31/34/35 lifecycle, settings/interactive PTY and reproducible signed release
remain unfinished or unverified. No network/Gradle/adb permission bypass.
No Android source changed this turn; prior source compilation is not a fresh
device or APK result. This note is provisional, not an acceptance receipt.
