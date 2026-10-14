# Device-Scoped Worktree Cancellation

Provisional S2-018/S2-019/S2-037 progress observed 2026-10-02T07:38Z.
Previous goal turn made progress: bounded durable execution progress. HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty changes preserved. All 36
authoritative rows remain [ ]; full goal active. No controller worker admission,
subagents, commit, push, cleanup or acceptance promotion.

## Implementation

WorkerProgress now retains the original typed JobId with each request and
provides a cancel adapter over the existing CancelAdmissions state machine.
Fresh cancellation resolves only a nonterminal request in the same runner
generation; current scope/revision/owner/ACL and original submitting device
are checked before persistent or runtime side effects. A reused job counter
after runner reconstruction cannot satisfy the generation identity. Device-
scoped request strings can coincide: choose the caller's own matching request
before a foreign one so another device cannot shadow its target.

Reuse the existing durable cancellation namespace: reserve UnknownOutcome in
the owner checkpoint before runtime try_cancel, then settle accepted/failed.
Exact retries answer the stored receipt even after terminal tracker removal
or host restart; they do not issue a second cancellation or target replacement
work. A revoked device cannot query its old receipt. Cancelled admission is
not claimed to mean arbitrary provider/tool side effects were rolled back.

Only the captured runtime JobId is cancelled. No broad emergency cancellation
is sent to all worker approval coordinators. Queued work never starts after
its cancellation is processed; active work receives its own cooperative token.
Approval waits already check this predicate. Independent worker slots remain
active and can succeed. Untracked/terminal/new-generation requests are refused
before checkpoint mutation. The existing runtime completion/progress path
persists cancellation distinctly from success and failure.

Three production-domain integration tests exercise running and queued jobs,
independent-slot success, other-device refusal, owner/session/revision refusal,
exact retry/restart and revoked receipt refusal, old tracker/new runner
rejection, plus two devices sharing the same request string. Execution uses
real WorktreeRunner and Echo fixtures; worker Agent locks provide deterministic
gates. Cancellation commands are observed before releasing locks, so tests
do not race a fast completed turn into a false cancellation claim. This is
not physical LAN, device UI or provider/tool rollback evidence.

## Verification

Cwd /Users/mac/Github/zenpi. Final commands use
ZENPI_HOME=/private/tmp/zenpi-worker-cancel-final and exit 0:

- rtk cargo test --offline --locked --test remote_workspace
  --test remote_worktree_runner --test remote_worker_owner --test remote_concurrency:
  **82 tests**, 4 suites (50/9/12/11), 10.76 seconds after final refusal coverage.
  Earlier intermediate suites also passed, not counted again.
- rtk cargo test --offline --locked --lib remote::worker: **4 tests**,
  397 filtered out, 0.17 seconds.
- rtk cargo test --offline --locked --test remote_workspace
  -- --test-threads=1: **50 tests**, 18.25 seconds; rerun not added to totals.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit
  --skip production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  **8 tests**, 2 restricted TCP fixtures deliberately excluded, 1.20 seconds.
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
  tools/validate_stage2_android2unix_blueprint.py --strict pass; checker reports
  36 incomplete items. Unique Rust tests across listed suites: **197**.

The first cancellation test also passed as one exact raw Cargo test before
the expanded coverage. It is not added to unique totals. All polling used live
test sessions and no observation timeout caused an automatic restart.

SHA256 at verification:

```text
src/remote/worker_progress.rs c6deb608225e3c517eb5dd42c2c19dca854595a759191d91a5b280f1408e33c2
tests/remote_workspace.rs b66aadbf849ffae5a056aea8dfcc51f444f8e2e23da7fb6b29afe3b661879f2f
Docs/stage2_android2unix_protocol.md bebc24435abf3f2d91373745a27fbf27584a31b250810d80d927fc6b0de94868
```

## Residual Scope

Need production TUI/headless/network wiring for worker admission/progress/
cancellation and lifecycle ownership, worker approvals/event/live text/
transcript routing, effective live quota receipts and Android worker controls.
Master remote prompts remain unchanged. Android concurrency_control remains
false; S2-018/S2-019/S2-037 not accepted from this host-domain increment.
Generic runtime cancellation remains cooperative and cannot undo side effects.

Native Android camera QR, real same-LAN desktop synchronization, current APK/
API31/34/35 lifecycle, settings/interactive PTY and reproducible signed release
remain unfinished or unverified. No Android source changed this turn; no new
APK/device claim. Restricted TCP/Gradle/adb constraints not bypassed. This is
provisional evidence, not an acceptance receipt. Full objective stays active.
