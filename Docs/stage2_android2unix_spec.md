# Stage 2 Android ↔ Unix execution specification

```yaml
schema_version: execution-spec/stage2
spec_version: 2.0.0
authoritative_blueprint: Docs/stage2_android2unix_blueprint.md
gantt_projection: Docs/stage2_android2unix_gantt.md
gantt_naming: exact-prefix-Blueprint-to-Gantt
worker_transport: tmux_codex_tui
app_server_workers: forbidden
nested_agents: forbidden
worker_lifecycle: bounded
product_modes: [tui, headless, android-control]
per_item_code_loc_cap: 5000
loc_policy: every Blueprint item has an integer Estimated LOC with 0 <= value < 5000; scope is a per-item forecast of implementation/test code attributable to the row (Rust/Kotlin/Gradle/Python/Shell), not a current-file inventory; docs/config/generated artifacts count 0; the declared forecast has a strict upper bound of 5000 (exclusive); aggregate repository LOC is informational
blueprint_parser: tools/validate_stage2_android2unix_blueprint.py
gantt_generator: tools/generate_stage2_android2unix_gantt.py
runtime_root: .ops/stage2_android2unix
evidence_root: Docs/quality/stage2_android2unix
max_logical_claims: 35
max_admitted_executions: 3
max_starting_lanes: 3
max_live_transports: 3
max_authenticated_goals: 3
max_running_turns: 3
max_outstanding_requests_per_execution: 1
max_request_starts_per_60s: 3
max_inflight_requests: 3
max_integration_lanes: 1
max_validator_lanes: 1
launch_fanout: 3
request_breaker_threshold: 3 starts or 3 failures in 60s
request_breaker_reset: explicit Master/operator receipt only
cron_marker: disabled-until-explicit-install
completion_cleanup: zero unresolved checklist, handoff, integration, repair, live transport, task root and lock
```

## Frozen policies

The blueprint is the only checklist authority. The Gantt is a generated
read-only projection. A worker can create a durable self-tested handoff and
the controller can write `[_]`; only the canonical Master can write `[x]`.
The existing dirty worktree is preserved. No reset, stash, checkout, broad
process kill, or unrelated cleanup is permitted.

The default lifecycle is bounded. Claims are not persistent services. Nested
agents are forbidden. Each worker generation has exactly one task-local tmux
server, interactive Codex TUI, private `CODEX_HOME`, thread, goal and one
outstanding request. `codex exec`, app-server workers, shared tmux and shared
state are hard failures.

The controller must report logical claims, admitted executions, starting/live
transports, authenticated/running goals, request starts, in-flight and
outstanding requests, handoffs, integration and repair independently. It may
launch a maximum of three dependency-ready conflict-safe claims in one wave,
but must not treat fanout as a hidden overall cap. Slow tests, builds, network
fixtures and integration run outside the global scheduler lease.

Every underfilled slot gets a persisted concrete reason: dependency,
ownership conflict, startup deadline, host resource, validator, route,
external Android SDK/emulator capacity or breaker. The request breaker is
fail-closed and never resets from a cron/watchdog tick.

## Claim, task and handoff policy

Each task root is:

```text
.ops/stage2_android2unix/tasks/<item-id>/<run-id>/
  work/
  codex-home/
  tmux.sock
  claim.json
  result.json
```

Only declared owned paths may be changed. The canonical checkout, Blueprint,
specification, selector, Gantt and completion receipts are Master-owned. A
claim card binds item ID, run ID, requirement/specification digests, baseline,
dependencies, exact owned paths, validators, rollback, deadline and result
path. A result manifest binds changed paths, patch checksum, commands,
outcomes and `status=self_tested`; it is copied into immutable queue storage
before a stale task is pruned.

The Master integrates only dependency-ready, conflict-safe handoffs. Failed
validation preserves the handoff and moves the item into bounded repair. A
repair is a fresh bounded execution with a new run ID. Completion cleanup
requires zero `[ ]`, zero `[_]`, empty handoff/integration/repair queues, a
fresh Gantt and no controller-owned task process, socket or lock.

## Required validator behavior

`validate_stage2_android2unix_blueprint.py` must be stdlib-only and reject:

- missing/duplicate/malformed stable IDs, unsupported marks, missing
  dependencies and dependency cycles;
- unsafe or absolute owned paths, duplicate ownership without an explicit
  serialized integration owner, missing validator/rollback/estimate fields;
- any `Estimated LOC >= 5000` or negative forecast;
- stale/missing selector, requirement/specification digest mismatch, and
  source/specification changes that are not reflected in the selector;
- missing or stale same-prefix Gantt, duplicate/missing monitoring IDs,
  invented timing and mutable checkboxes in the Gantt;
- forbidden Codex transport commands, nested-agent claims and full-checkout
  task templates in executable controller surfaces.

The validator must have negative fixture tests for every refusal and at least
two unlike fixture repositories with different names, paths, languages,
validators and ID prefixes. `--strict` requires receipts and the final
completion surfaces; normal validation may inspect an unfinished DAG.

## Required Gantt behavior

The authoritative filename `stage2_android2unix_blueprint.md` maps to the
exact same-prefix companion `stage2_android2unix_gantt.md`. The generator
atomically writes a Mermaid dependency-layer view plus a monitoring index.
Every S2 ID appears exactly once. The index derives state, dependencies,
owner, startup/live, handoff/integration/repair and scheduling note from the
Blueprint and durable ledgers; it never becomes an input. Timing is either a
recorded timestamp or an explicitly frozen estimate. Unknown timing is shown
as unscheduled.

## Repository gates

Master runs only commands declared by this specification and the relevant
item. Baseline gates are:

```text
python3 tools/validate_stage2_android2unix_blueprint.py --strict
python3 tools/test_validate_stage2_android2unix_blueprint.py
cargo fmt --check
cargo check --locked
cargo test --locked
cargo build --release --locked
(cd android && ./gradlew test)
(cd android && ./gradlew assembleRelease)
```

Connected Android tests are required only where named by the item. An absent
SDK/emulator is recorded as a binding reason, not as a pass. Production host
tests must use the real Unix binary and bounded local provider/tool fixtures;
Echo or metadata-only tests do not satisfy G-HOST.

## Network and security acceptance contract

MVP transport is opt-in TLS/TCP on LAN or a user-configured reachable host.
The host has no listener until `zenpi host pair` is invoked. QR invites are
short-lived and one-use. The client pins the host key, confirms a safety code,
and never receives provider credentials. Revoked devices, expired invites,
unknown schema, ACL failures, malformed frames, replayed request IDs and path
escapes fail closed.

The host continues accepted work when Android disconnects. Resume uses a
transport event cursor separate from the session journal cursor. A replay gap
returns a bounded snapshot plus tail. A timeout with unknown side effect is
not automatically retried. Remote subtab mutation uses stable IDs and
project/subtab revisions; a conflict refreshes before retry. Worktree removal
must succeed before durable deletion, otherwise the tab remains with a visible
error.

Doze, process death, Android network changes and foreground-service limits are
expected states. Android must show connected/stale/offline/replay-gap/unknown
outcome distinctly. WorkManager is an opportunity-based resume mechanism, not
a socket keepalive guarantee. No acceptance gate may claim arbitrary public
NAT traversal without an independently deployed signaling/STUN/TURN system.
