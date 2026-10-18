# Independent Headless Project Execution

Provisional S2-014/S2-019 and scheduler groundwork. Previous goal turn repaired
network monitor restart identity. This turn removes headless's global active
prompt bottleneck: distinct ProjectOwnerPool Agents can now run concurrently
without sharing a session writer. Full QR/LAN/native Android/Rust desktop goal
remains active; all 36 rows `[ ]`, S2-018 not accepted or represented complete.

## Implementation

HeadlessHost owns a project-keyed map of captured job + existing bounded runner.
Admission checks only that project's active lane plus a 64-lane host bound.
Runner worker reuses the exact published Agent, existing progress persistence,
request-scoped SubtabExecution/root validation and provider preview pipeline.
Completion persists terminal progress, removes/joins its lane. Steering and
cancel select by request project; active project/subtab close is refused while
idle unrelated project close can proceed. Admission job IDs increment across
the host instead of repeating each fresh runner's local JobId. Durable
request binding/reservation/receipt, ACL and CAS behavior unchanged.

Shutdown first emergency-cancels approvals and signals every active runner,
then joins them; possible detached noncooperative work remains conservative
unknown outcome, never invented success. One writer per project is retained.
This does not implement independent worktree owners within one project,
configurable 1..64 worktree quotas/queue policy, TUI parallel lanes or Android
concurrency stepper. concurrency_control stays false.

## Verification

Cwd /Users/mac/Github/zenpi, observed 2026-10-02T01:31:02Z UTC;
ZENPI_HOME=/private/tmp/zenpi-project-lanes-20261002 for tests.

- Two new tests run actual HeadlessHost mailbox/runner/Agents with controlled
  local backends and signed device registry. First holds a project backend,
  proves another independent owner completes, job receipts differ, wrong-
  project cancel cannot affect first, unrelated project closes, first cancel
  completes and old prompt retry does not reinvoke backend. Second holds both
  owners, proves cancellation affects only one and shutdown signals the other
  within 3 seconds with both owner locks released. **2 pass**.
- Existing focused headless_cancel and registry_busy tests **2 pass**. Initial
  new fixture failed cancellation after closing another project because the
  request's workspace revision was stale; test now uses actual close receipt
  revision, preserving host CAS. Grant fixture uses current authentication
  after multi-project permissions, not stale device revision.
- `cargo test --offline --locked --test remote_headless --test remote_parity
  --test remote_subscription --test remote_subscription_tls --test remote_workspace`:
  **47 pass**, exit 0 (1/4/10/10/22).
- `cargo test --offline --locked --test runtime`: **12 pass**, exit 0.
  Cargo check, cargo fmt and git diff checks pass; existing warnings remain.
- Focused real HTTP test headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit:
  exit 101 at TcpListener::bind, PermissionDenied/Operation not permitted,
  before provider/runner execution. No network pass or skip manufactured.
- Python smoke 34/lifecycle 11/Blueprint 5: **50 pass**; strict Blueprint pass
  with 36 incomplete rows. Android source unchanged, no new APK/device run,
  permission bypass, agents, commit/push or historical receipt promotion.

SHA-256:

```text
src/remote/headless_host.rs 826f4d944a33382827f3cef4fbfc64057e0f93f1560a23d8cb7c6f2c0efe9385
tests/remote_headless_commands.rs 42c7f3c7f3d9cdbd91c5bd45775dda362e1c249c926359630ea1727ef121ef3e
```

Residual requirements include independent-worktree quotas 1..64 with real
scheduler gates, complete TUI/headless parity, real provider/TCP execution,
fresh API31/34/35 APK/socket/Keystore/lifecycle, physical camera QR/same-WiFi
sync, full stream delta replay/backpressure, interactive PTY/bulk and signed
release/migration. Existing dirty worktree preserved; goal remains active.
