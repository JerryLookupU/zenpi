# Phone Live Status Priority

Provisional S2-019/S2-020/S2-036 progress at 2026-10-02T11:55:48Z.
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. Current dirty worktree,
blueprint and execution skill inspected. Previous turn made progress (ambiguous
phone cancellation). User changes preserved. No nested agents, execution-worker
admission, commit/push/cleanup or authoritative checkbox promotion.

## Implementation

Phone status previously iterated oldest tracked jobs and stopped at 16, so
retained terminal records could hide a later live task. The project aggregate
could similarly keep an earlier lane's terminal tail ahead of a sibling lane's
live work. A tracked job with no drained runtime event was absent altogether,
even after actual submission and accepted admission.

Per-lane projection now orders runtime-active jobs first by admission job ID,
then recent terminal entries by descending job ID. A submitted tracked job
before first event drain projects accepted plus runtime_active. This is derived
from real tracking, not reconstructed from durable running evidence. Project
aggregation deduplicates as before, then orders runtime-active entries ahead of
terminal entries across lanes before applying the existing 16-entry wire cap.
Device/project/current-master-session filtering, internal-field removal and
read-only behavior remain intact. No raw prompt is projected or journal rewritten.

This corrects prioritization, not full task-list coverage. More than 16 live jobs
can still exceed the display cap; paged status browsing remains required. Android
source is unchanged because it already decodes accepted plus runtime_active.

## Verification

Cwd /Users/mac/Github/zenpi; ZENPI_HOME=/private/tmp/zenpi-live-status-priority.
Completed commands exit 0:

- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  11 passed, 429 filtered, 0.54 seconds (includes progress/admission). New fixture
  keeps 40 tracked entries: 38 completed and two not yet event-drained. Both live
  accepted identities appear first, followed by the recent terminal tail, bounded
  to 16; foreign device is empty, prompt absent and journal bytes unchanged.
- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  15 passed, 426 filtered, 6.11 seconds. Controlled two-lane fixture submits 16
  terminal-producing jobs to the earlier sorted lane and one held job to the
  later lane. Holding master lock retains the terminal tail; aggregated status
  still puts later live task first under the cap. Release/poll retires owners.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test core_session --test runtime --test remote_parity
  -- --test-threads=1: 143 passed, six suites, 40.62 seconds. Production desktop
  fixture queries WorkspaceList before controller polling and observes its actually
  submitted phone task as accepted plus runtime_active.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text --test headless_project_workspace
  --test tui_project_workspace --test tui_composer --test remote_concurrency --test remote_migration
  --test remote_subtab --test remote_worktree -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel --test-threads=1:
  168 passed, three restricted TCP fixtures excluded, eleven suites, 17.67 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.074 seconds.
- cargo fmt --all --check; git diff --check; python3
  tools/validate_stage2_android2unix_blueprint.py --strict pass; all 36 unfinished.

All observed command handles terminal. No restricted TCP/Gradle/adb retry or
permission bypass. Controlled runtime and production-domain fixtures are not
provider HTTP, LAN/camera or Android device evidence. No new JVM/APK claim.
Provisional evidence is not acceptance-grade stdout/stderr capture hashes.

SHA256:

```text
src/remote/worker_progress.rs 61058e36d026973315baa56d5534217522542ef3f335c3773ec4720920a12d84
src/remote/worktree_host.rs 6d7c4a86d6059fb90ce32bf9edc207fa1ae37afa9467e41df01233c5c6ee9e5d
tests/remote_workspace.rs 1826da7d72957c0e5d09f84cc5e1ee2fec721f27ea83ea66cb830ed7bae62891
```

## Remaining Gates

Full status paging beyond 16, local task submission/quota UX, live/failed partial
history, omitted invocation paging and interactive PTY remain unfinished. Real
API31/34/35 camera QR/LAN desktop sync, disconnect/replay/address changes, process
lifecycle and signed APK/reproducible release remain unverified. Full goal active.
