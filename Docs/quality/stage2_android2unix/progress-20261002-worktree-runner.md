# Host-Quota Worktree Runner

Provisional S2-018/S2-019 progress observed 2026-10-02T07:14Z.
Previous goal turn made progress: independent worker owners and journal leases.
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty worktree preserved.
All 36 authoritative rows remain [ ]; full goal active. No worker admission,
subagents, commit, push, cleanup or acceptance promotion.

## Implementation

New WorktreeRunner captures the host's persisted subtab quota and prepares
that many independent WorkerOwner slots. It drives the existing bounded
BackgroundRunner, starts jobs only on free slots and returns slots on success,
failure or panic. It exposes job-ID cancellation and bounded event polling/
orderly shutdown, not a second scheduler with different queue semantics.
Generic callbacks retain the ordinary host responsibility for job behavior.
The production spawn_prompts adapter runs bounded text via the existing
prompt-input/Agent/tool pipeline, validates the captured subtab root at actual
execution, rejects shell shorthand/NUL/private references and propagates
cancellation/completion markers. It does not implement remote admission or
durable request-ID deduplication by itself.

WorkerClaim ties each owner to at most one runtime. Idle runtime-owned slots
are now fenced during subtab/project mutation and owner retirement because a
queued job could acquire them next. Claim acquisition takes the Agent lock
before an atomic owner flag; concurrent retirement and duplicate runtime start
fail closed. Partial multi-slot startup releases acquired claims. Claims live
inside the shared job closure, retained by each active thread: noncooperative
detached work keeps its claim and journal lease until actual thread exit.
Orderly shutdown releases claims, after which the host can change the durable
quota and reconstruct the runner. This does not yet implement live resizing.

Nine integration tests cover host quota/N+1 queue behavior, parallel independent
journals/master bytes unchanged, even idle slots fenced against retirement or
quota mutation, successful later reconstruction with changed quota, partial
claim rollback, active/queued scoped cancellation, panic slot reuse, retained
claims after detach, persisted quota/session restart without automatic work,
the production prompt adapter's input/root restrictions and a registered real
Git worktree with separate worker/master journal and tool roots. Deterministic
Echo backends verify owner/scheduler behavior, not a provider-HTTP or production
host/network acceptance gate. Existing real tool tests remain regressions.

## Verification

Cwd /Users/mac/Github/zenpi. Final environment
ZENPI_HOME=/private/tmp/zenpi-worktree-runner-final. Commands exit 0 unless
explicitly noted:

- rtk cargo test --offline --locked --test remote_worktree_runner
  --test remote_worker_owner --test remote_concurrency --test runtime
  --test remote_subtab --test remote_worktree --test remote_workspace:
  **102 tests**, 7 suites, 15.81 seconds, with 8 runner tests at that point.
  After the Git-worktree test was added, rtk cargo test --offline --locked
  --test remote_worktree_runner: **9 tests**, 0.85 seconds. The later suite
  replaces the prior 8-test result: **103 distinct tests** across these suites.
- rtk cargo test --offline --locked --test remote_worktree_runner
  -- --test-threads=1: **8 tests** before the last Git test, 1.92 seconds;
  not counted again in unique totals.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit
  --skip production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  **8 tests**, 2 restricted TCP fixtures deliberately excluded.
- rtk cargo test --offline --locked --lib `remote::headless_host::`:
  **8 tests**, 389 filtered out, 0.33 seconds.
- env ZENPI_HOME=/private/tmp/zenpi-worktree-runner-final cargo test --offline
  --locked --test headless_project_workspace --test tui_project_workspace
  --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: **103 tests**, 19/56/28, 1 restricted TCP test excluded.
  Unique Rust total including the final runner suite: **222**.
  The earlier RTK-wrapped parallel invocation stalled after two suite results;
  its live handle was polled, then explicitly interrupted (exit 130). Only
  after termination was this raw serial diagnostic run started. It completed
  with every named test visible. No stuck test is called a pass.
- cargo check --offline --locked --test remote_worktree_runner
  --message-format=short: exit 0, 20 existing warnings. Earlier rtk cargo
  check --offline --locked also passed. An initial test compilation returned
  one error while implementation was in progress; final compiled suites above
  supersede it. No initial failure is promoted to a passing gate.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  **63 tests**. cargo fmt --all -- --check, git diff --check and python3
  tools/validate_stage2_android2unix_blueprint.py --strict pass; checker reports
  all 36 rows incomplete, not behavioral item acceptance.

SHA256 at verification:

```text
src/remote/worktree_runner.rs 6fe38b4a376eb1b42a967b956cee24562f35920a34c42d2d177beffcefe77e2b
src/remote/worker.rs 9e1af1c058e8f7f07b1b918b15211e3266980f0abc17d62100b94397ad4952ee
src/project_workspace.rs 39ea7b282abf05f74ce0c30aa548f79c30c5b3da9f5ae6732aed316c18bd9db1
src/remote/subtab.rs e408d86fb19aaf37939673355603827fbbaae237995dc2e3004a9250d66f1c62
tests/remote_worktree_runner.rs 9d36b7f26d19b27620f0a2fc98faeb1751998aa57c63914c9afa4fd56564a5f6
Docs/stage2_android2unix_protocol.md 6eb03c8a6443526350813f76683d66cc966d09dde2023d9d73d79a5192dc880d
```

## Residual Scope

The host-domain runner exists but TUI/headless/Android still need its real
admission endpoint, durable request binding, live effective-quota application/
receipts, worker approval/event/status routing and controller teardown. Existing
remote prompts remain master discussion turns. concurrency_control stays false;
neither S2-018 nor S2-019 is accepted. A caller-held worker mutex is still public
for existing owner APIs; hosts must not independently execute on claimed slots.
No universal rollback of noncooperative detached provider/tool side effects.

Native Android camera QR, real same-LAN desktop synchronization, current APK/
API31/34/35 lifecycle, settings/interactive PTY and reproducible signed release
remain unfinished or unverified. No Android source changed this turn; no fresh
APK/device claim. Restricted network/Gradle/adb boundaries were not bypassed.
This note is provisional, not an acceptance receipt.
