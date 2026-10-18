# Durable Remote Reasoning Selection

Provisional S2-003/S2-019/S2-032/S2-036/S2-054 progress observed
2026-10-02T04:11:15Z. Previous goal turn added durable model selection and
behavioral tests: progress. Full Android/QR/LAN/Rust desktop objective remains
active. HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; existing changes preserved.
All 36 authoritative rows remain [ ]; no acceptance promotion.

## Implementation

Agent.reasoning_choices projects only registry levels validated by the actual
backend for the current model. Null (provider default) is independently
validated; low-level non-registry owners have no choices, and ChatCompletions
without explicit reasoning support advertises only default. No name inference,
network discovery, credential, source metadata or pricing. model_status adds
the bounded reasoning_choices field; Android accepts the legacy shape as no
choices and strictly rejects duplicate/unknown/non-string choices.

Agent.select_reasoning checks the shared work fence and model-selection CAS,
validates effort against its current model/backend, writes a complete selection
snapshot and only then performs the infallible backend effort commit. It retains
model/account/endpoint. Null is distinct from the explicit none level.

reasoning_select Control operation uses the same bounded durable admission
implementation as ModelSelect with its own remote_reasoning_admission_v1 kind.
The checkpoint namespace permits ReasoningSelect and restore imports its journal
bindings. IDs cannot be reused for model/prompt/workspace/steer operations.
Receipt lookup rechecks ACL/session before returning the original terminal
result, regardless of later selections. Unresolved receipts and bound retries
behind locked owners remain unknown and never reexecute. Shared/mailbox,
headless active-lane and desktop pending/admitted gates route it consistently.
Listing exposes separate reasoning_control with durable Control authority.

Android dropdown contains only host-supplied choices and explicit Apply. It is
absent on old/read-only hosts and disabled offline/loading/unresolved writes.
New command, strict null-sensitive receipt validation and ViewModel use the
existing encrypted pending write, pinned authenticated TLS, exact retry and
foreground/network policy; no optimistic success. Status remains transient.

## Verification

Cwd /Users/mac/Github/zenpi, ZENPI_HOME=/private/tmp/zenpi-reasoning-select.
Cargo uses RTK 0.50.0 with offline locked dependencies.

- remote_workspace/remote_protocol/remote_parity/remote_subscription_tls/
  remote_auth final combined run: exit 0, **95 pass** (workspace 37, protocol 11,
  parity 5, actual fragmented rustls/P256 TLS 31, auth 11).
  Three new workspace reasoning tests cover null/none, retained model, ACL,
  host Busy, stale selection, restart/legacy namespace import, later local
  changes, invalid effort retry, cross-model IDs, malformed values, bound locked
  owner, unresolved journal reservation and staged-attachment failure with
  fresh-request success after clearing. Existing model/status tests also pass.
- New TLS 1.3 test sends provider-default reasoning selection twice, checks
  identical terminal receipts, exactly one selection event, unchanged model,
  no projected credentials and default restored. This is in-memory fragmented
  caller-owned streams, not TCP/LAN or Android device execution.
- stage1_reasoning_owner excluding only
  real_requests_use_new_effort_and_restart_preserves_model_effort_and_default_reset:
  exit 0, **12 pass**, one socket-dependent HTTP test excluded. New choice/wire/
  typed CAS/invalid-level/failed-append tests; existing staged/queued/recovery/
  approval fence cases now also exercise the typed reasoning API.
- remote_headless_commands excluding
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit and
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider:
  exit 0, **8 pass**, two socket-dependent tests excluded. New idle mailbox
  selection/exact retry after later local default and running-lane rejection;
  zero provider turn and unchanged model assertions.
- Android source check: main/unit/instrumented compile and **160 JVM tests**
  in 31 classes. New choice/default/none/exact receipt/old-host/invalid-shape
  tests; foreground policy includes reasoning commands and exact pending
  retries. New Compose dropdown/explicit Apply/offline behavior test compiles
  only; not executed on a device.
  Exact argv: zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-reasoning-select-android-verified, exit 0.
- Five Python smoke/lifecycle/blueprint modules: exit 0, **57 pass**.
- cargo fmt --all -- --check, git diff --check, strict blueprint: exit 0.

Intermediate Rust compile rejected an unmapped serde_json error; corrected to
AgentError. Initial Kotlin compile lacked the existing icon import; added it
and reran. Unrelated compiler/deprecation warnings left unchanged.

Compiled SHA256:

```text
main.jar 7dcd1bf18c2ffc02a42b72d49f555ab24fe216c25ea3ae0934030a69e92b4499
unit.jar 80751b095c609ae6a3136f5b3598ef19290eb8a9aa896e198ece4ed27ec7599e
instrumented.jar 81856f6daa25afff1be52a112f2110f385e4d36ba8d170f3d2adb4fc8ad11483
```

## Remaining Scope

Current-source APK, real camera/QR/LAN and API-device lifecycle matrix remain
unverified. No claim of APK success from Kotlin/JVM results. Previous Gradle
daemon ServerSocket EPERM diagnosis remains relevant; no permission bypass,
repeated installation or network download performed. Model catalog chooser,
account/settings parity, per-worktree quota scheduler, interactive PTY,
provider-delta replay/ACK, signing and migration remain unfinished. No agents,
worker admission, commits, pushes, cleanup or goal completion.
