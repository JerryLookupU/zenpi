# Zenpi Stage 2 — Android ↔ Unix 工作机联动执行 Blueprint

> 本文件是 Stage 2 Android 联动项目的唯一执行清单。Stage 1 文档是前置阶段的历史权威清单；本阶段只改变 Stage 2 的状态，不回写 Stage 1 状态。

```yaml
schema_version: execution-blueprint/stage2
blueprint_version: 2.0.0
revision_date: 2026-09-20
review_date: 2026-09-20
status: bootstrap-active
authoritative: true
predecessor: Docs/stage_1_v3_pi_mono_blueprint.md
reference_research: Docs/researches/android2unix.md
reference_proposal: Docs/researches/android2unix_proposal.md
stable_id_pattern: '^S2-[0-9]{3}$'
status_values: '[ ]|[_]|[x]'
per_item_code_loc_cap: 5000
per_item_code_loc_rule: 'estimated_loc < 5000'
target_repo: /Users/mac/Github/zenpi
target_revision: 918b8c36072cbc1dbffd25a63d5117d9b9d03d5f
target_baseline: current-worktree-with-user-changes
product_modes: [tui, headless, android-control]
worker_transport: tmux_codex_tui
app_server_workers: forbidden
nested_agents: forbidden
worker_lifecycle: bounded
runtime_root: .ops/stage2_android2unix
execution_spec: Docs/stage2_android2unix_spec.md
gantt_projection: Docs/stage2_android2unix_gantt.md
completion_policy: '[x] is written only by Master after integrated behavioral gates; [_] is a checksum-valid worker handoff and remains unfinished; [ ] is unclaimed or incomplete'
```

## 1. Scope and non-negotiable decisions

This stage turns the pure Unix-like Rust `zenpi` TUI/headless runtime into the
working-machine half of a phone-to-desktop development tool. The Android side
is a native Kotlin/Compose control and display client. Agent execution,
provider credentials, tools, git, PTY, TUI and headless ownership remain on
macOS/Linux.

The first deliverable is intentionally LAN-first: the user runs
`zenpi host pair`, the Unix host starts a temporary TLS/TCP listener, and the
Android app scans a QR containing an address, port, host-key fingerprint,
short-lived one-time token and expiry. QR does not solve arbitrary NAT. Public
Internet ICE/STUN/TURN, relay operation and background push are later optional
work, never hidden MVP dependencies.

The host is the source of truth for session journals, context checkpoints,
provider/tool side effects, approvals, workspaces, worktrees and concurrency.
Android resumes with event cursors and receives replay or an explicit bounded
snapshot plus tail. Android never executes shell/git/provider calls and never
receives provider credentials.

The protocol is transport-independent. MVP uses bounded length-prefixed JSON
over TLS/TCP; a future QUIC transport may reuse the same envelope and replay
semantics. Every write is owner-checked, revision-checked and request-ID
deduplicated. A failed worktree deletion is visible and does not silently
remove the durable tab.

## 2. Repository evidence and authority boundary

The baseline is the current worktree, not a presumed clean checkout. Existing
evidence includes:

- `src/core.rs` accepts only `tui` and `headless` today; no network listener or
  Android target exists.
- `src/protocol.rs` already has bounded v1/v2 JSONL request, response and event
  envelopes suitable for an adapter, but stdin/stdout is not a network server.
- `src/headless.rs` has `run_async_streams`, owned project routing, local replay
  and owner registration. Its replay WAL and live-owner registry are host-local
  primitives, not network authentication.
- `src/project_workspace.rs` owns layer-1 project identities and owner pools.
  Layer-2 tab data is currently TUI layout state rather than a remote owner
  domain.
- `src/tui.rs` implements in-place/worktree subtabs and a 1..64 concurrency
  control, while `/worktree` headless dispatch is intentionally non-executing.
- `Docs/researches/android2unix_proposal.md` is the product decision record;
  this Blueprint converts it into executable work and acceptance gates.

No existing implementation is promoted to `[x]` by evidence inspection alone.
The Master must integrate code, run the prescribed validators and record
behavioral evidence before accepting an item.

## 3. Execution contract

### 3.1 Checklist and DAG

Each item has one stable ID, one owner scope, explicit repository-relative
owned paths, explicit validators, rollback behavior, dependencies and an
integer implementation/test forecast. `[ ]` means unfinished, `[_]` means a
worker produced a checksum-valid handoff, and `[x]` means Master integrated and
revalidated it. Workers never write `[x]`.

Dependencies are real dataflow or acceptance dependencies, not document order.
The validator rejects missing IDs, duplicate IDs, cycles, unsupported marks,
unsafe paths and LOC forecasts outside the strict 0..4999 range.

### 3.2 Isolation and lifecycle

The frozen execution specification is
[`Docs/stage2_android2unix_spec.md`](stage2_android2unix_spec.md). Every claim
uses a task-local root under `.ops/stage2_android2unix/tasks/<item>/<run>/`, a
private writable `CODEX_HOME`, its own tmux socket/session and exactly one
interactive `/goal`. `codex exec`, app-server workers, shared tmux, shared
Codex state, no-tmux workers and nested agents are forbidden. The canonical
checkout and this Blueprint are Master-owned write targets.

The lifecycle is `reserved -> materialized -> tmux_started -> goal_pasted ->
request_leased -> goal_submitted -> authenticated -> handoff_ready ->
integrated -> finished`. A task-local result is provisional until Master
harvests it and reruns the repository validator. Slow work runs outside the
global scheduler lock. Runtime counters distinguish logical claims, admitted
executions, starting lanes, live transports, authenticated goals, running
turns, request starts, in-flight requests, handoffs, integration and repair.

### 3.3 Evidence and completion surfaces

The mandatory companion is the same-prefix projection
`Docs/stage2_android2unix_gantt.md`. It is generated read-only, contains a
renderable Mermaid Gantt and a monitoring index with every item exactly once,
state, dependencies, owner, startup/live state, handoff/integration/repair
state and scheduling note. It contains no mutable checklist marks and is never
the source of truth.

Item gates use these names:

- **G-FILE**: declared files, owner boundary, error/cancel/recovery behavior;
- **G-DIR**: cross-file ownership and call-graph closure where a directory item
  exists;
- **G-CODE**: deterministic unit/integration/property tests and restart,
  cancellation or conflict cases that apply;
- **G-HOST**: real production binary/TUI/headless entry with local fixtures,
  not an Echo shortcut or metadata-only assertion;
- **G-ANDROID**: Android unit/instrumented/Compose behavior on supported API
  levels, including lifecycle and accessibility-relevant states;
- **G-SECURITY**: key/token/pinning/replay/path/ACL fail-closed evidence;
- **G-NETWORK**: real loopback/LAN disconnect, address change and replay
  behavior; no claim of arbitrary NAT success;
- **G-RELEASE**: reproducible build, packaging, migration and rollback.

The canonical evidence root is `Docs/quality/stage2_android2unix/`. Receipts
must include item ID, requirement/specification digest, baseline and integrated
revision, exact argv/cwd, controlled environment names, timestamps, exit code,
stdout/stderr hashes, expected/actual criteria and artifact paths.

## 4. Architecture contract

### Host boundary

Add a host-owned adapter around the existing `ProjectOwnerPool`, Agent,
`run_async_streams`, session replay and TUI state. It must expose one typed
domain to local TUI, local headless and Android. The network listener is opt-in
and has no socket when `zenpi host pair` is not active.

### Wire envelope

Use bounded `u32 big-endian length + UTF-8 JSON`. Preserve `schema_version`,
`id`, `session_id`, `project_id`, `turn_id`, `sequence`, `code`,
`execution_state` and `required_owner`. Separate logical control, event and
bulk channels even while MVP uses one TLS connection. Request IDs are durable
enough to answer an ambiguous retry without repeating a side effect.

### Pairing and security

The host creates a persistent identity and a short-lived invite. Android pins
the host public key/certificate, confirms a six-digit safety code and stores its
device key in Android Keystore. MVP may use pinned host key plus one-time token;
mTLS and device ACL/revocation are required before calling the feature suitable
for untrusted networks.

### Android lifecycle

Compose uses `PrimaryTabRow` for workspaces and `SecondaryTabRow` for worktrees.
The foreground connection is opportunistic. Doze, process death and network
switches close the client connection; the host continues execution and Android
returns through `resume/replay`. WorkManager is for opportunity-based resume
and queue flush, not socket persistence. A user-visible FGS is optional and
bounded; it is not an infinite keepalive promise.

## 5. Execution checklist

### L0 — authority, specification and scaffolding

- [ ] **S2-001** — Freeze Stage 2 authority, baseline digest, selector and status contract；layer `L0` | Depends: — | Owner scope: Stage 2 requirement authority and immutable execution metadata | Owned paths: `Docs/stage2_android2unix_spec.md`, `Docs/execution/stage2_android2unix_active_requirement.json` | Validators: G-FILE; `python3 tools/validate_stage2_android2unix_blueprint.py --strict` | Rollback: remove only Stage 2 selector/spec additions and leave Stage 1 and user changes untouched | Estimate: one bootstrap transaction | Estimated LOC: 500
- [ ] **S2-002** — Build portable Stage 2 Blueprint validator and same-name Gantt generator；layer `L0` | Depends: S2-001 | Owner scope: parser, DAG, LOC, path, digest, Gantt and portability gates | Owned paths: `tools/validate_stage2_android2unix_blueprint.py`, `tools/generate_stage2_android2unix_gantt.py`, `tools/test_validate_stage2_android2unix_blueprint.py`, `Docs/stage2_android2unix_gantt.md` | Validators: G-CODE; `python3 tools/test_validate_stage2_android2unix_blueprint.py` | Rollback: remove only new Stage 2 tooling and generated projection | Estimate: stdlib-only checker with fixture repositories | Estimated LOC: 1600
- [ ] **S2-003** — Freeze host/API schema version and compatibility policy；layer `L0` | Depends: S2-001 | Owner scope: wire schema, version negotiation, compatibility and frame/resource caps | Owned paths: `Docs/stage2_android2unix_protocol.md`, `src/remote/protocol.rs`, `tests/remote_protocol.rs` | Validators: G-FILE; G-CODE; `cargo test --locked --test remote_protocol` | Rollback: reject schema version and remove adapter behind feature gate; preserve existing v1/v2 JSONL | Estimate: schema table plus bounded codec tests | Estimated LOC: 1100

### L1 — Unix host domain and secure local transport

- [ ] **S2-010** — Introduce transport-independent bounded frame I/O；layer `L1` | Depends: S2-003 | Owner scope: read/write framing, limits, EOF, timeout and malformed-frame refusal | Owned paths: `src/remote/frame.rs`, `tests/remote_frame.rs` | Validators: G-FILE; G-CODE; `cargo test --locked --test remote_frame` | Rollback: disable remote adapter and retain stdio path | Estimate: length-prefixed JSON codec with boundary tests | Estimated LOC: 1400
- [ ] **S2-011** — Implement opt-in TLS/TCP listener lifecycle for `zenpi host pair`；layer `L1` | Depends: S2-010 | Owner scope: bind policy, shutdown, listener task, connection limits and no-listener default | Owned paths: `src/remote/listener.rs`, `src/remote/mod.rs`, `src/core.rs`, `tests/remote_listener.rs` | Validators: G-HOST; G-NETWORK; `cargo test --locked --test remote_listener` | Rollback: remove host subcommand and keep production binary TUI/headless-only | Estimate: loopback/LAN fixture and clean shutdown | Estimated LOC: 2200
- [ ] **S2-012** — Add QR invite, host identity and pinned-key pairing；layer `L1` | Depends: S2-011 | Owner scope: invite encoding, expiry, one-time use, fingerprint and SAS confirmation | Owned paths: `src/remote/pairing.rs`, `src/security.rs`, `tests/remote_pairing.rs`, `Docs/stage2_android2unix_protocol.md` | Validators: G-SECURITY; G-CODE; `cargo test --locked --test remote_pairing` | Rollback: invalidate all invite records and disable listener admission | Estimate: no credentials in QR and replay/expiry tests | Estimated LOC: 2300
- [ ] **S2-013** — Add device registry, project ACL and revocation；layer `L1` | Depends: S2-012 | Owner scope: device identity, permissions, revoke and fail-closed authorization | Owned paths: `src/remote/auth.rs`, `src/remote/pairing.rs`, `tests/remote_auth.rs` | Validators: G-SECURITY; G-CODE; `cargo test --locked --test remote_auth` | Rollback: revoke generated devices and require fresh pairing | Estimate: read-only/control/approval capability matrix | Estimated LOC: 1900
- [ ] **S2-014** — Adapt owned headless streams to the network host；layer `L1` | Depends: S2-003, S2-010, S2-013 | Owner scope: caller-owned stream adapter, owner epoch and error envelope | Owned paths: `src/headless.rs`, `src/remote/host_adapter.rs`, `tests/remote_headless.rs` | Validators: G-HOST; G-CODE; `cargo test --locked --test remote_headless` | Rollback: leave `run_async_streams` public and remove only network adapter | Estimate: production JSONL behavior over fixture TLS stream | Estimated LOC: 2600
- [ ] **S2-015** — Define host-owned workspace control API；layer `L1` | Depends: S2-003, S2-014 | Owner scope: project open/select/list/close, context and revision response | Owned paths: `src/remote/workspace_api.rs`, `src/project_workspace.rs`, `tests/remote_workspace.rs` | Validators: G-FILE; G-CODE; G-HOST; `cargo test --locked --test remote_workspace` | Rollback: route only to existing typed owned project command | Estimate: CAS and busy-owner behavior | Estimated LOC: 2100
- [ ] **S2-016** — Move layer-2 tabs into a host-owned stable-ID domain；layer `L1` | Depends: S2-015 | Owner scope: subtab schema v2, stable IDs, migration, ordering and atomic checkpoint | Owned paths: `src/remote/subtab.rs`, `src/project_workspace.rs`, `src/tui.rs`, `tests/remote_subtab.rs` | Validators: G-FILE; G-CODE; G-HOST; `cargo test --locked --test remote_subtab` | Rollback: read legacy TUI layout as read-only and discard incomplete migration checkpoint | Estimate: old index migration plus concurrent revision tests | Estimated LOC: 3400
- [ ] **S2-017** — Expose safe in-place/worktree add/select/move/rename/close actions；layer `L1` | Depends: S2-016 | Owner scope: canonical paths, branch/name policy, busy gate and visible delete failure | Owned paths: `src/remote/subtab.rs`, `src/project_workspace.rs`, `tests/remote_worktree.rs` | Validators: G-SECURITY; G-HOST; G-CODE; `cargo test --locked --test remote_worktree` | Rollback: disable destructive subtab actions, preserve checkpoint and worktree | Estimate: git fixture, dirty tree and failure injection | Estimated LOC: 3000
- [ ] **S2-018** — Connect per-worktree concurrency to the scheduler；layer `L1` | Depends: S2-016 | Owner scope: 1..64 validation, effective quota, admission and persistence | Owned paths: `src/remote/subtab.rs`, `src/runtime.rs`, `src/view_model.rs`, `tests/remote_concurrency.rs` | Validators: G-CODE; G-HOST; `cargo test --locked --test remote_concurrency` | Rollback: return effective value to prior host quota and reject remote changes | Estimate: busy/queued/race tests with no N+1 worker | Estimated LOC: 3200
- [ ] **S2-019** — Unify TUI, headless and Android command routing；layer `L1` | Depends: S2-014, S2-015, S2-017, S2-018 | Owner scope: one owner, one state machine and no slash echo for remote mutations | Owned paths: `src/remote/command.rs`, `src/tui.rs`, `src/headless.rs`, `tests/remote_parity.rs` | Validators: G-FILE; G-CODE; G-HOST; `cargo test --locked --test remote_parity` | Rollback: route Android to read-only capabilities until parity adapter is repaired | Estimate: TUI plus owned headless dual-client scenarios | Estimated LOC: 2900
- [ ] **S2-020** — Add event projection, redaction, backpressure and bounded replay；layer `L1` | Depends: S2-014, S2-019 | Owner scope: event sequence, redaction, replay window, ACK and slow-client policy | Owned paths: `src/remote/events.rs`, `src/security.rs`, `src/session.rs`, `tests/remote_events.rs` | Validators: G-SECURITY; G-NETWORK; G-CODE; `cargo test --locked --test remote_events` | Rollback: close remote sessions on queue overflow and preserve host journal | Estimate: replay gap snapshot+tail and sensitive-field tests | Estimated LOC: 2800

### L2 — Android application and user workflows

- [ ] **S2-030** — Scaffold Android Kotlin/Compose app and shared schema module；layer `L2` | Depends: S2-003 | Owner scope: Gradle project, API floor, Compose state architecture and schema generation | Owned paths: `android/settings.gradle.kts`, `android/app/build.gradle.kts`, `android/app/src/main/`, `Docs/stage2_android2unix_android.md` | Validators: G-ANDROID; G-RELEASE; `./gradlew :app:test` | Rollback: remove Android module without changing Rust host | Estimate: clean build and deterministic schema fixture | Estimated LOC: 1800
- [ ] **S2-031** — Implement QR scanner, invite parser and pairing confirmation UI；layer `L2` | Depends: S2-012, S2-030 | Owner scope: URI validation, expiry, fingerprint display and SAS confirmation | Owned paths: `android/app/src/main/java/zenpi/pairing/`, `android/app/src/test/` | Validators: G-ANDROID; G-SECURITY; `./gradlew :app:test` | Rollback: disable scanner route and clear pending invite state | Estimate: malformed/expired/mismatched QR tests | Estimated LOC: 2200
- [ ] **S2-032** — Implement Android TLS client and bounded frame transport；layer `L2` | Depends: S2-010, S2-012, S2-030, S2-031 | Owner scope: pinned TLS, frame limits, cancellation and connection state | Owned paths: `android/app/src/main/java/zenpi/transport/`, `android/app/src/test/` | Validators: G-ANDROID; G-SECURITY; G-NETWORK; `./gradlew :app:test` | Rollback: ship pairing-only build and remove transport entry point | Estimate: loopback fixture with certificate mismatch tests | Estimated LOC: 3000
- [ ] **S2-033** — Build Android event reducer, cursor cache and snapshot replacement；layer `L2` | Depends: S2-020, S2-032 | Owner scope: event ordering, replay gap, durable small metadata and conflict state | Owned paths: `android/app/src/main/java/zenpi/state/`, `android/app/src/test/` | Validators: G-CODE; G-ANDROID; `./gradlew :app:test` | Rollback: clear local cache and force host snapshot on next attach | Estimate: property tests for duplicate/out-of-order/gap events | Estimated LOC: 2700
- [ ] **S2-034** — Implement workspace primary tabs and host-backed CRUD；layer `L2` | Depends: S2-015, S2-033 | Owner scope: Compose primary tabs, pending/error/conflict states and accessible actions | Owned paths: `android/app/src/main/java/zenpi/workspace/`, `android/app/src/test/` | Validators: G-ANDROID; G-HOST; `./gradlew :app:test` | Rollback: make workspace controls read-only while retaining transcript | Estimate: state-driven Compose tests and host fixture | Estimated LOC: 2300
- [ ] **S2-035** — Implement worktree secondary tabs and per-tab concurrency stepper；layer `L2` | Depends: S2-017, S2-018, S2-034 | Owner scope: add/select/move/rename/close and 1..64 host-backed stepper | Owned paths: `android/app/src/main/java/zenpi/worktree/`, `android/app/src/test/` | Validators: G-ANDROID; G-HOST; G-CODE; `./gradlew :app:test` | Rollback: hide destructive controls and preserve host state | Estimate: optimistic pending, CAS conflict and delete failure UI | Estimated LOC: 2900
- [ ] **S2-036** — Implement transcript, stream event, approval, diff and PTY projections；layer `L2` | Depends: S2-019, S2-020, S2-033 | Owner scope: complete TUI-facing control/display parity without local execution | Owned paths: `android/app/src/main/java/zenpi/session/`, `android/app/src/test/` | Validators: G-ANDROID; G-HOST; G-SECURITY; `./gradlew :app:test` | Rollback: retain status/transcript-only client and deny unsafe controls | Estimate: approval timeout, unknown outcome and bounded bulk output | Estimated LOC: 3600
- [ ] **S2-037** — Implement Android Keystore storage and device revoke UX；layer `L2` | Depends: S2-013, S2-030 | Owner scope: non-exportable key, encrypted metadata, logout/wipe and revoke request | Owned paths: `android/app/src/main/java/zenpi/security/`, `android/app/src/androidTest/` | Validators: G-SECURITY; G-ANDROID; `./gradlew :app:connectedAndroidTest` | Rollback: invalidate local credentials and require QR re-pair | Estimate: emulator/Keystore availability and wipe tests | Estimated LOC: 2200
- [ ] **S2-038** — Implement foreground/offline/resume lifecycle and NetworkCallback；layer `L2` | Depends: S2-032, S2-033 | Owner scope: stale/offline state, reconnect backoff, WorkManager resume and no false success | Owned paths: `android/app/src/main/java/zenpi/lifecycle/`, `android/app/src/androidTest/` | Validators: G-ANDROID; G-NETWORK; `./gradlew :app:connectedAndroidTest` | Rollback: disable background worker and retain explicit foreground reconnect | Estimate: process recreation, network switch and replay tests | Estimated LOC: 3100

### L3 — Cross-platform hardening and test evidence

- [ ] **S2-050** — Add Rust protocol/property/fuzz boundary tests；layer `L3` | Depends: S2-003, S2-010, S2-020 | Owner scope: malformed frame, resource cap, duplicate ID and compatibility refusal | Owned paths: `tests/remote_protocol.rs`, `tests/remote_frame.rs`, `fuzz/remote_protocol/` | Validators: G-CODE; `cargo test --locked --test remote_protocol`; `cargo test --locked --test remote_frame` | Rollback: disable new schema version and keep existing protocol tests | Estimate: bounded proptest/fuzz corpus, no unbounded input | Estimated LOC: 2600
- [ ] **S2-051** — Add Unix host end-to-end loopback/LAN integration suite；layer `L3` | Depends: S2-011, S2-014, S2-019, S2-020 | Owner scope: production binary listener through provider/tool/session fixture | Owned paths: `tests/remote_host_e2e.rs`, `tools/stage2_host_smoke.py`, `Docs/quality/stage2_android2unix/` | Validators: G-HOST; G-NETWORK; `python3 tools/stage2_host_smoke.py --binary target/release/zenpi` | Rollback: preserve failing receipt and disable host release profile | Estimate: prompt/stream/approve/cancel/replay and clean shutdown | Estimated LOC: 3200
- [ ] **S2-052** — Add pairing and security adversarial tests；layer `L3` | Depends: S2-012, S2-013, S2-020 | Owner scope: token replay, fingerprint mismatch, revoked device, ACL and redaction | Owned paths: `tests/remote_security.rs`, `android/app/src/test/` | Validators: G-SECURITY; `cargo test --locked --test remote_security`; `./gradlew :app:test` | Rollback: fail closed by rejecting all remote admissions | Estimate: deterministic key fixture and negative matrix | Estimated LOC: 2400
- [ ] **S2-053** — Add network fault and replay-gap acceptance matrix；layer `L3` | Depends: S2-020, S2-038, S2-051 | Owner scope: disconnect, half-close, timeout, duplicate ACK, address change and WAL gap | Owned paths: `tests/remote_faults.rs`, `android/app/src/androidTest/`, `Docs/quality/stage2_android2unix/network-matrix.md` | Validators: G-NETWORK; G-ANDROID; `cargo test --locked --test remote_faults`; `./gradlew :app:connectedAndroidTest` | Rollback: mark client unavailable and require explicit reattach | Estimate: controlled socket proxy and Android lifecycle matrix | Estimated LOC: 3000
- [ ] **S2-054** — Verify TUI/headless/Android parity and owner races；layer `L3` | Depends: S2-019, S2-034, S2-035, S2-036, S2-051 | Owner scope: same session, simultaneous local/remote mutations and revision conflicts | Owned paths: `tests/remote_parity.rs`, `Docs/quality/stage2_android2unix/parity-matrix.md` | Validators: G-HOST; G-ANDROID; G-CODE; `cargo test --locked --test remote_parity`; `./gradlew :app:test` | Rollback: release Android read-only until conflicting paths are fixed | Estimate: dual-client transcript/context digest and subtab matrix | Estimated LOC: 3400
- [ ] **S2-055** — Validate Android API 31/34/35 lifecycle behavior；layer `L3` | Depends: S2-037, S2-038, S2-053 | Owner scope: Doze/background FGS limits, process death, notification and foreground resume | Owned paths: `android/app/src/androidTest/`, `Docs/quality/stage2_android2unix/android-lifecycle.md` | Validators: G-ANDROID; G-NETWORK; `./gradlew :app:connectedAndroidTest` | Rollback: disable FGS path and require foreground-only connection | Estimate: API matrix with explicit unsupported-state receipts | Estimated LOC: 2800
- [ ] **S2-056** — Add migration, backup, uninstall and credential-wipe tests；layer `L3` | Depends: S2-016, S2-037, S2-038 | Owner scope: host checkpoint migration, Android cache migration and safe reset | Owned paths: `tests/remote_migration.rs`, `android/app/src/androidTest/`, `Docs/quality/stage2_android2unix/migration.md` | Validators: G-CODE; G-SECURITY; G-ANDROID; `cargo test --locked --test remote_migration`; `./gradlew :app:connectedAndroidTest` | Rollback: preserve old checkpoint and require explicit migration retry | Estimate: old/new schema fixtures and interrupted migration | Estimated LOC: 2500

### L5 — release and operational readiness

- [ ] **S2-070** — Produce reproducible Unix host release and operator runbook；layer `L5` | Depends: S2-051, S2-052, S2-054, S2-056 | Owner scope: release profile, listener permissions, pairing/revoke/backup and rollback | Owned paths: `Cargo.toml`, `Cargo.lock`, `tools/stage2_host_smoke.py`, `Docs/quality/stage2_android2unix/runbook.md`, `Docs/quality/stage2_android2unix/release.md` | Validators: G-RELEASE; G-HOST; `cargo build --release --locked`; smoke script | Rollback: remove host remote feature and restore prior binary/config | Estimate: clean build plus documented rollback drill | Estimated LOC: 1800
- [ ] **S2-071** — Produce signed Android debug/release packaging and install smoke；layer `L5` | Depends: S2-030, S2-037, S2-038, S2-055 | Owner scope: reproducible Gradle build, manifest permissions, package and upgrade path | Owned paths: `android/`, `Docs/quality/stage2_android2unix/android-release.md` | Validators: G-RELEASE; G-ANDROID; `./gradlew :app:assembleRelease`; install/upgrade smoke | Rollback: publish previous APK and invalidate new pairing format if necessary | Estimate: debug and release artifact hashes, no credentials | Estimated LOC: 1700
- [ ] **S2-072** — Run security/privacy and threat-model review；layer `L5` | Depends: S2-052, S2-054, S2-055, S2-070, S2-071 | Owner scope: secrets, logs, path boundary, replay, device revoke and disclosure | Owned paths: `Docs/quality/stage2_android2unix/security-review.md`, `src/remote/`, `android/app/src/main/` | Validators: G-SECURITY; G-RELEASE; `rg` secret/redaction audit plus Master review | Rollback: block release, revoke test devices and remove unsafe capability | Estimate: threat matrix with residual-risk sign-off | Estimated LOC: 900
- [ ] **S2-073** — Publish parity, support and known-limits documentation；layer `L5` | Depends: S2-054, S2-055, S2-070, S2-071, S2-072 | Owner scope: user setup, LAN limitation, lifecycle semantics, error states and recovery | Owned paths: `Docs/stage2_android2unix_user_guide.md` | Validators: G-FILE; G-RELEASE; link/path audit and Master review | Rollback: publish docs-only correction without changing runtime | Estimate: documentation and command examples only | Estimated LOC: 0

### L6 — Master completion gate

- [ ] **S2-099** — Master integration, full gates, completion surfaces and cleanup；layer `L6` | Depends: S2-070, S2-071, S2-072, S2-073 | Owner scope: canonical integration, final validator/Gantt, receipts, repair closure and task cleanup | Owned paths: `Docs/stage2_android2unix_blueprint.md`, `Docs/stage2_android2unix_gantt.md`, `Docs/stage2_android2unix_spec.md`, `Docs/quality/stage2_android2unix/` | Validators: all applicable G-FILE/G-DIR/G-CODE/G-HOST/G-ANDROID/G-SECURITY/G-NETWORK/G-RELEASE gates; validator strict mode; `cargo fmt --check`; `cargo check --locked`; `cargo test --locked`; Android tests/build | Rollback: restore last accepted revision and preserve all receipts; never delete user changes | Estimate: final dependency-ready integration and cleanup pass | Estimated LOC: 2200

## 6. Required acceptance commands

These commands are gates to be executed by Master, not claims that this
document has already passed them:

```bash
python3 tools/validate_stage2_android2unix_blueprint.py --strict
python3 tools/test_validate_stage2_android2unix_blueprint.py
cargo fmt --check
cargo check --locked
cargo test --locked
cargo build --release --locked
python3 tools/stage2_host_smoke.py --binary target/release/zenpi
(cd android && ./gradlew test)
(cd android && ./gradlew assembleRelease)
```

Android connected tests and API-level lifecycle tests are required for the
items that name them. Missing Android SDK/emulator capacity is an explicit
underfill/block reason, not a successful test.

## 7. Failure, rollback and cleanup policy

An ambiguous provider/tool/worktree result is `unknown_outcome`, never an
automatic retry. A stale or revoked device is denied. A replay gap causes a
bounded snapshot replacement. A full event queue closes the remote connection
without dropping the host journal. A revision conflict refreshes state before
any retry. A failed git worktree removal retains the subtab and surfaces the
error.

Completion requires zero `[ ]`, zero `[_]`, no pending handoff/integration or
repair queue, a fresh Gantt digest, all required gates passing and no live
Stage 2 task roots, tmux sessions, sockets or locks. Cleanup may remove only
controller-owned runtime artifacts and must preserve the canonical checkout,
user changes and accepted receipts.

## 8. References

- [`Docs/researches/android2unix.md`](researches/android2unix.md)
- [`Docs/researches/android2unix_proposal.md`](researches/android2unix_proposal.md)
- [`src/protocol.rs`](../src/protocol.rs), [`src/headless.rs`](../src/headless.rs),
  [`src/project_workspace.rs`](../src/project_workspace.rs), [`src/session.rs`](../src/session.rs),
  [`src/tui.rs`](../src/tui.rs)
- [RFC 8445 ICE](https://www.rfc-editor.org/rfc/rfc8445.html),
  [RFC 9000 QUIC](https://www.rfc-editor.org/rfc/rfc9000.html)
- [Android Compose Tabs](https://developer.android.com/develop/ui/compose/components/tabs),
  [Doze and App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby),
  [Foreground-service restrictions](https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start)
