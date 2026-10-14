# Durable Worker Execution Progress

Provisional S2-018/S2-019/S2-020 progress observed 2026-10-02T07:33Z.
Previous goal turn made progress: durable worker admission and lease-release
regression repair. HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty
changes preserved. All 36 authoritative rows remain [ ]; full goal active.
No controller worker admission, subagents, commit, push, cleanup or acceptance.

## Implementation

New WorkerProgress associates runtime job IDs with admitted request/device/
project/master-session/subtab identity. dispatch_tracked registers submission
before the post-submit journal append, so an ambiguous admission append cannot
hide later execution observation. A tracker binds only after authenticated
submission and belongs to one runner generation; identical scope after a
runner restart is not sufficient because job counters restart. Retention is
bounded at 128. Full/wrong-generation trackers refuse new submission before
checkpoint reservation but exact receipt queries remain possible.

Runtime Accepted/Queued/Started/Rejected/Completed become typed execution
states, separate from immutable admission. Queue rejection becomes Failed/Busy,
not Accepted forever. Agent failure maps only typed codes, panic becomes
InternalError and ordinary cancellation remains distinct. Shutdown cancellation
is UnknownOutcome because detached code may outlive the runner. Terminal
observation ignores later stale Started events. No prompt text, Agent diagnostic,
provider response or worker path is included in progress.

Append remote_worker_progress_v1 to the master journal. Held master locks retain
latest progress/terminal results for later flush. Successful terminal append
removes the pending tracker record and never repeats. Nonterminal state remains
available live without repeated appends. Append error is ambiguous: retain a
bounded live UnknownOutcome record, clear dirty state and never blindly retry
the append, even if the original journal becomes writable again. Session
replacement or missing/poisoned owner drops publication rather than writing
into a replacement session. Every successful append wakes the host session.

Listing is bounded to 16 entries and filters device/project/master-session.
Live values override durable records. Restart retains confirmed terminal
states, while unconfirmed durable Accepted/Running becomes UnknownOutcome,
never a fabricated live task. Listing itself is a host-domain projection;
external callers must enforce ACL before publishing it remotely.

Three integration tests run real admission/worker execution and cover held
master completion, restart terminal identity, exact-retry tracker stability,
privacy/scope, append failure with later file recovery but no second append,
and bounded runtime rejection after an Accepted admission. Two library tests
cover rejection/cancellation/shutdown/failure/panic, typed-only diagnostics,
stale late event refusal, tracker generation binding and 128-entry cap.

## Verification

Cwd /Users/mac/Github/zenpi. Final commands use
ZENPI_HOME=/private/tmp/zenpi-worker-progress-final and exit 0:

- rtk cargo test --offline --locked --test remote_workspace
  --test remote_worktree_runner --test remote_worker_owner --test remote_concurrency:
  **79 tests**, 4 suites (47/9/12/11), 10.51 seconds after final
  no-mutation-on-refused-tracker binding adjustment. The earlier run also
  passed; duplicate runs are not counted again. No Android/device/network
  acceptance claim.
- rtk cargo test --offline --locked --lib remote::worker: **4 tests**,
  397 filtered out, 0.22 seconds after final binding adjustment. Includes two
  new progress tests plus existing admission reservation and lease regression.
- rtk cargo test --offline --locked --test remote_workspace
  -- --test-threads=1: **47 tests**, 19.90 seconds before final binding
  adjustment, not counted again in unique totals.
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
  36 items incomplete. Unique Rust tests across listed suites: **194**.

SHA256 at final verification:

```text
src/remote/worker_progress.rs c174df25340be327dc840623a010737c4e9efe85f60d0fed27ab1e46e3476d68
src/remote/worker_admission.rs 24c08e42acc5e7c68ae7c91c322753b3ee0f728eb254cb6e71133eb5642ece50
src/remote/worktree_runner.rs fc075a5648c423714e603bfcb255653ec72ef5eff85ff762761918ba88fd151e
tests/remote_workspace.rs e3585676fdbfc596865e2be9568fde3c8bd84466c3659ac7948dc6d87046a224
Docs/stage2_android2unix_protocol.md e8eb6381eed4741d1b074cf697685ca19d2e7222a019cd87b30fe9b4487c2c46
```

## Residual Scope

Need production TUI/headless/network integration and lifecycle ownership for
this tracker, worker cancellation/approvals, live text/transcript routing,
status publication under device ACL and effective quota receipts. Existing
remote prompts remain master discussion turns and new worker argument routing
is not yet installed. Android worker controls/concurrency_control remain
disabled. S2-018/S2-019/S2-020 are not accepted. Generic WorktreeRunner job
callbacks still own their execution behavior; progress observes outcomes, not
side-effect rollback or a guarantee detached code has stopped.

Native Android camera QR, real same-LAN desktop synchronization, current APK/
API31/34/35 lifecycle, settings/interactive PTY and reproducible signed release
remain unfinished or unverified. No Android source change or fresh APK/device
claim this turn. Restricted TCP/Gradle/adb constraints not bypassed. This is a
provisional note, not an acceptance receipt. Full objective stays active.
