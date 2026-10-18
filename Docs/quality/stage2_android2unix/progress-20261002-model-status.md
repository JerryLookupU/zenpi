# Host Model Status

Provisional S2-003/S2-019/S2-032/S2-036 progress observed 2026-10-02T03:35:51Z.
Previous goal turn repaired resource-description privacy with behavioral tests:
progress. Full native Android/QR/LAN/Rust desktop objective remains active.
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty worktree preserved.
Strict blueprint validator keeps all 36 rows [ ]; no acceptance promotion.

## Implementation

Added model_status Read operation through the actual shared/headless adapter
and desktop owner mailbox. Exact empty arguments, required ID/project/session,
no outer owner/revision/turn/sequence; current Read ACL and session checked
under admission, nonblocking Agent lock. Result allowlists model, reasoning
level, registry-bound flag, model-selection revision, nullable existing provider
capabilities and effective context/reserved-output budgets. No descriptor
sources, catalog, endpoint, credential or connection snapshot. Labels are
bounded/control-free/redaction-checked and reject path/URL-like data. Results
are <=4 KiB, without journal/checkpoint/provider side effects.

Existing Agent.model_status now additionally includes its actual configured
model (backend fallback when Agent has no override), avoiding an invented
Unknown label for non-registry low-level backends. The remote projection still
explicitly reports registry_bound=false and nullable unknown capabilities.
No change to model selection, reasoning validation or provider invocation.

Android strict codec, pinned/authenticated HostClient route, transient ViewModel
job/generation and native model dialog are integrated. Read-only devices can
inspect without prompt control. Null capabilities display unavailable; reasoning
matches registry levels including ultra. Foreground/network/selection/read-tool
boundaries clear/cancel the state and reset joins the job before wiping keys.
Refresh is disabled offline/loading/pending-write; model title is bounded with
ellipsis. Main display/cursor cache is unchanged. Context budget is not measured
token usage, and model/connection/reasoning mutation remains unfinished.

## Build Diagnosis

Inspected cached Gradle distribution and allowed temporary writable storage.
Ran existing Gradle 8.11.1 binary with GRADLE_USER_HOME set to
/private/tmp/zenpi-stage2-gradle-build and JAVA_HOME=/opt/homebrew/opt/openjdk@17:

```text
.../gradle-8.11.1/bin/gradle --offline --no-daemon
  :app:assembleDebug :app:assembleDebugAndroidTest
```

Cwd android. Exit 1: temporary cache initialization succeeds, then Gradle's
single-use daemon fails java.net.SocketException: Operation not permitted at
ServerSocket bind before dependency/configuration/build execution. This is new
evidence that changing the cache path alone cannot produce an APK in the current
environment. No system permission change, sandbox bypass, huge cache copy,
network download or claimed APK/device execution.

## Verification

Cwd /Users/mac/Github/zenpi; Rust environment ZENPI_HOME=/private/tmp/zenpi-model-status.

- rtk cargo test --offline --locked --test remote_workspace
  --test remote_protocol --test remote_subscription_tls --test remote_parity:
  exit 0, **73 pass** (workspace 29, protocol 11, fragmented actual rustls/P256
  TLS 29, parity 4). Tests cover unknown/default model, real registry-bound
  model/effort/capabilities, credential/URL omission, owner/mailbox Busy,
  session mismatch, revoke between reads and unchanged journal/checkpoint.
- rtk cargo test --offline --locked --lib remote::model::tests: exit 0,
  **1 pass**, label boundary/private credential/URL/path refusal.
- Existing stage1_model_registry full suite attempted: **8 pass, 9 fail** at
  TcpListener::bind EPERM, not product assertions. Repeated with the nine exact
  socket-dependent tests excluded: **8 pass**, including failed switch,
  restart/status, registry override/config and admitted-turn read-only tests.
- stage1_reasoning_owner with
  real_requests_use_new_effort_and_restart_preserves_model_effort_and_default_reset
  excluded: **4 pass**, one socket-dependent test excluded. No claim of a green
  complete Stage 1 network suite.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-model-status-android-verified: exit 0, **158 JVM tests** in
  31 classes. Main/unit/instrumented compile. Two new codec tests cover exact
  scope/keys/label/numeric/flags, null capability/model and ultra. Compose
  read-only/offline-refresh test compiled only; not device execution.
- Five Python smoke/lifecycle/blueprint modules: exit 0, **57 pass**.
- cargo fmt --all -- --check, git diff --check, strict blueprint: exit 0.
  Existing unrelated compiler/deprecation warnings remain. Intermediate test
  construction corrected to existing new_with_settings API; unknown backend
  capability expectation corrected to null, not inferred true.

Compiled SHA256:

```text
main.jar affcc0196babbc0f1042decf50c04550b226f6cc9ec31bd819fc3a7ad1eecc7a
unit.jar 34a351570f68b6a107b8ca56247e39061c7e5389531138d90c3be44343bcd22e
instrumented.jar 0ccc95cae59cac1650879e9c2e734683ba49ad5debff1305ad18b8db0dc1b60e
```

No current-source phone/camera/LAN/API-matrix acceptance, full per-worktree
quota scheduler, model/settings mutation parity, provider-delta replay/ACK,
interactive PTY, signed release or migration completion. No agents spawned,
commit/push, permission bypass or unrelated rollback. Goal remains active with
meaningful implementation progress despite build/device environment limits.
