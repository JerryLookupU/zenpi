# Durable Worktree Request Admission

Provisional S2-018/S2-019 progress observed 2026-10-02T07:24:43Z.
Previous goal turn made progress: host-quota worktree execution and runtime
claims. HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty changes preserved.
All 36 authoritative rows remain [ ]; full goal active. No worker admission,
subagents, commit, push, cleanup or acceptance promotion. Product runtime
workers are unrelated to skill/controller Codex worker admission.

## Implementation

New worker_admission::dispatch accepts an authenticated device and existing
WorktreeRunner, exact Prompt arguments text/subtab_id/subtab_revision, and the
ordinary owner/session/workspace-CAS envelope. It validates Control plus Read
grants, runner's captured project/master-session/subtab/revision, current host
quota and safe input before any write. WorktreeRunner now retains that scope.

Bind the device/request/full-envelope fingerprint in the existing durable
workspace prompt namespace before master-journal reservation or execution
submission. Append remote_worker_admission_v1 reserved then accepted/failed
events; receipts include the request/job/project/master-session/subtab, never
provider credentials, worker paths or prompt text. Ambiguous checkpoint/journal
failure returns UnknownOutcome, without automatic retry. Exact retry reads
the original admission receipt before current owner nonce/revision checks,
including after restart, but still validates current ACL and session. Changed
content and reuse by a master prompt fail RequestIdConflict. Busy master
returns unknown without reservation. Restart imports worker journal bindings
alongside prompt/model/reasoning bindings; checkpoint-only reservations remain
unknown and never submit again.

Four integration tests exercise actual production admission plus real worker
runtime: read-only/revoked ACL, safe-input/exact-argument refusal, owner/quota/
revision/scope guards, held master, exact retry before/after completion and
restart, worker journal unchanged on retry, cross-owner request-ID conflict,
checkpoint-only crash state and binding import from master admission journal.
A same-host checkpoint-only reservation is tested directly in the library.
These deterministic Echo fixtures prove admission/ownership behavior, not
production TLS/TCP/LAN or provider tool execution acceptance.

Regression exposed a lifetime lease-release race when another thread forked a
child and temporarily inherited the file descriptor. WorkerState Drop now
explicitly releases flock at last-owner destruction, matching the existing
session lock pattern. A deterministic dup/open-file-description test proves
the last worker handle unlocks even with the inherited descriptor still open,
and later closing that old descriptor cannot release the new owner's lease.
Existing cross-process/retained-runtime tests still pass. Active handles and
detached jobs continue retaining leases until their real last-handle drop.

## Verification

Cwd /Users/mac/Github/zenpi. Final commands use
ZENPI_HOME=/private/tmp/zenpi-worker-admission-final and exit 0:

- rtk cargo test --offline --locked --test remote_workspace
  --test remote_worktree_runner --test remote_worker_owner
  --test remote_concurrency --test remote_protocol: **90 tests**, 5 suites,
  10.42 seconds, after the lease-release correction.
- rtk cargo test --offline --locked --lib remote::worker: **2 tests**,
  397 filtered out, 0.15 seconds (admission reservation plus lease duplicate
  descriptor regression). Earlier focused admission test also passed;
  duplicate runs are not counted again.
- rtk cargo test --offline --locked --test remote_workspace
  -- --test-threads=1: **44 tests**, 17.61 seconds, after the correction;
  rerun not added to unique totals.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit
  --skip production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  **8 tests**, 2 restricted TCP fixtures deliberately excluded.
- cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: **103 tests**, 19/56/28, 1 restricted TCP fixture excluded.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  **63 tests**. cargo fmt --all -- --check, git diff --check and python3
  tools/validate_stage2_android2unix_blueprint.py --strict pass, reporting all
  36 items incomplete. Unique Rust tests across listed suites: **203**.

An initial test compile used an incorrect revoke method arity; corrected
before final tests. A later parallel workspace rerun exposed lease retention
and failed 1 of 44; the explicit unlock and duplicate-descriptor regression
above address it. Earlier failures are recorded, not promoted to passing gates.

SHA256 at verification:

```text
src/remote/worker_admission.rs 1b99f3998e0d8081c32675331aa346af828818e97a24e2b77c682e388fee75d3
src/remote/worktree_runner.rs da47fe7deefd46c8c25dafda38f2a7ac5b26c58a420c2721568e576572cd1030
src/remote/worker.rs 91988fb2342ef18a6e890bc46e751c6ad911c30abeab9185016e21f18dc1d97e
src/project_workspace.rs d38850aa2a74743ab185fb41b310ab7f95ac87808c04237a1652d179618db539
tests/remote_workspace.rs be867e3d7c693ae2538f5af262bea7fad4aa3a037bbc7dca86cf10361ad7c3b2
Docs/stage2_android2unix_protocol.md b4b9822d42695eafa13679ef050b4787254eb06861f0a0c80182879a2bc52534
```

## Residual Scope

This is host-domain admission, not an enabled production network endpoint.
Need actual HostAdapter/TUI/headless routing and runner lifecycle, durable
execution/rejection status reconciliation, worker approvals/event/live text/
transcript multiplexing, effective live quota application and Android worker
controls. Accepted means admission, not successful execution; runtime command
queue admission can later yield bounded scheduler rejection. Ordinary Prompt
still rejects worker arguments in its existing production route. Android
concurrency_control remains false. S2-018/S2-019 are not accepted.

Current APK/API31/34/35, camera QR and real same-LAN desktop synchronization,
settings/interactive PTY and reproducible signed release remain unfinished or
unverified. No Android source changed this turn; no fresh device/APK claim.
No restricted TCP/Gradle/adb permission bypass. Full objective stays active;
this note is provisional, not an acceptance receipt.
