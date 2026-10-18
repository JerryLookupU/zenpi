# Durable Remote Model Selection

Provisional S2-003/S2-019/S2-032/S2-036/S2-054 progress observed
2026-10-02T04:03:59Z. Previous goal turn repaired shared selection fences and
added behavioral tests: progress. Full Android/QR/LAN/Rust desktop goal remains
active. HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty changes preserved.
All 36 authoritative rows remain [ ]; no acceptance promotion.

## Implementation

Added model_select Control operation with exact model/selection_revision
arguments, workspace owner/revision and project/session identity. Agent's typed
model-only selection checks the shared work fence, selection CAS, retained
backend/effort/history and registry descriptor; it writes one full selection
snapshot before an infallible model update. No backend/account replacement,
provider request or remote projection of the private snapshot.

The existing bounded prompt_bindings namespace now admits an optional operation
kind. Missing kind remains Prompt, preserving old checkpoint serialization;
ModelSelect is the only new valid kind. Restore imports prompt and model journal
admissions and rejects mismatched kind/cross-operation identity. Checkpoint then
journal reservation precede selection; terminal journal admission holds the exact
receipt. Ambiguous reservation/append stays unknown and never reexecutes. A bound
retry behind a locked owner stays unknown, rather than claiming definite Busy.
Current ACL and session remain required; receipt lookup precedes current
owner/workspace/selection revision and busy checks, so old retries cannot undo a
later local model choice. Model changes do not advance workspace revision.

Shared adapter, desktop mailbox and headless project lanes route the operation.
Desktop pending/admitted jobs and headless active lanes fence newly admitted
selection before the Agent acquires its lock. Desktop's active discussion model
projection/catalog is refreshed from the actual Agent, not the old receipt.
Listing advertises model_control under durable Control authority. Android older
hosts default false; offline cache does not retain this permission.

Android Model ID/Apply action requires advertised model control and a scoped
registry-bound status. The immutable command uses the existing encrypted pending
write ledger, pinned/authenticated HostClient, confirmation callback, exact retry,
foreground/network gates and unknown-result handling. Receipt validates exact
IDs/model and strictly newer unsigned selection revision (journal sequence, not
workspace +1). No optimistic selection state. Read-only dialog remains usable.
Compose controls disable offline/loading/pending and are absent without live
authority. Status is transient and requires a fresh read after selection.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-model-select,
offline locked dependencies. RTK 0.50.0 used for compact Cargo output.

- remote_workspace: exit 0, 34 pass, including five new model-selection tests:
  CAS/ACL/Busy, exact durable retry, retained effort, later local choice,
  restart, mailbox route, cross-operation IDs, checkpoint failure, missing
  terminal receipt after a committed selection, legacy namespace import,
  operation-kind corruption and staged-attachment failure replay.
- remote_protocol, remote_parity, remote_subscription_tls initial combined
  run with workspace: exit 0, 78 pass; later additional workspace fence test
  combined run: 79 pass. Final rerun: exit 0, **80 pass** (workspace 34,
  protocol 11, parity 5, TLS 30), including the fifth workspace test.
  New actual rustls TLS 1.3/P256 fragmented stream test sends ModelSelect twice,
  verifies equal receipts and exactly one model_selected event, no credentials.
- stage1_reasoning_owner with the known socket-dependent HTTP test excluded:
  11 pass (combined workspace/reasoning run 44 pass before last workspace test).
  New typed CAS/invalid retained reasoning/failed durable append test keeps
  actual model, budget, effort, revision and stored bytes unchanged.
- remote_headless_commands excluding only
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit and
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  exit 0, 7 pass, 2 excluded. New running-owner selection rejection and idle
  mailbox selection/exact-retry-after-local-change with zero provider turns.
- remote_auth: exit 0, 11 pass.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-model-select-android-final: exit 0, 159 JVM tests in 31
  classes; main/unit/instrumented sources compile. New JVM command/receipt
  scope/model/revision/shape/invalid-label/old-host checks. Existing foreground
  policy tests now cover model write and exact pending retry. New Compose
  model-control/read-only/offline tests compile only, not device execution.
- Five Python smoke/lifecycle/blueprint modules: exit 0, 57 pass.
- cargo fmt --all -- --check, git diff --check, strict blueprint: exit 0.

Android compiled hashes:

```text
main.jar 41c60b3183ecc33d0961d605c2e66832699fa08b02e53fc9e2c2615bfade0920
unit.jar cd7729515f4d39ab18bfd60ac77421a515f2de590a771d19d70ea654c742a9b8
instrumented.jar e98ecc342aa66c070a23a8ce1b95eef264434d04f06e5ee6623dd5b48a7b8567
```

Intermediate TLS fixture captured a borrowed mode in a spawned thread; compiler
rejected it. Corrected to a captured bool, granted explicit Control for the new
mode and reran. Existing compiler/deprecation warnings left unchanged.

## Not Yet Accepted

This is model ID selection, not a complete catalog chooser or reasoning/account
settings parity. Current-source APK/camera/QR/LAN and API lifecycle device tests
remain unverified; source/JVM/TLS stream results are not their substitute. Prior
APK diagnosis reached Gradle daemon ServerSocket EPERM even with a writable temp
home; no repeated installation, sandbox bypass or APK success claimed here.
Complete per-worktree quota scheduler, interactive PTY, provider-delta replay/ACK,
release signing, migration and controller worker admission remain unfinished.
No agents, commits, pushes, cleanup or goal completion.
