# Stage 2 Acceptance Receipt Audit

The objective is incomplete. No authoritative checkbox was changed and no
controller worker was launched. The previous goal work made source and fixture
progress; the intervening RTK installation check did not advance product
acceptance. This turn addresses a concrete false-acceptance path and refreshes
stale release evidence instead of repeating an unavailable device run.

## Receipt Gate Repair

Previously, strict Blueprint validation allowed all items marked accepted when
an empty receipts directory existed. Partially accepted items required no
receipt at all. Strict mode now requires each accepted item to have accepted
dependencies and a matching successful Master receipt. It checks item/run and
filename identity, current requirement/specification digests, declared source
paths, baseline/patch digest shape, timezone-aware integration time, exact
declared command order, integer-zero exit statuses and stdout/stderr digest
shape. Failed attempts remain historical evidence; symlink receipts and a
symlink receipts directory cannot justify acceptance.

The controller now emits requirement/specification digests into its existing
receipt format. It does not retroactively rewrite historical receipts.

This is a receipt consistency gate, not complete behavioral acceptance or
cryptographic authentication of a Master. The remaining full receipt contract
(integrated revision, controlled environment, retained logs/artifacts,
expected/actual criteria), named G-* coverage, final completion surfaces and
controller reconciliation still require work. Strict success with unfinished
items reports structural consistency, not completed product delivery.

## Current Release Evidence

Two new fresh builds completed with exact argv retained in
/private/tmp/stage2-current-release-compare-20261004-audit.json. Lane A used
the temporary runtime; lane B used target/stage2-release-compare-mic84e8k/b.
Both exited zero and match the attributed target/release/zenpi exactly:
ba38f1b03712475e47cf22e9a63cd1977eaec46965fe10746eb1c78cd6453d47.
Current-source verification and all three strict ad-hoc signature checks passed.
Input digest: 96ad81b583669ddc487434fe47f4fdb6f18b304b256d795702f5ba7a01087fe1.
Receipt digest: 9fb36e8fa259b37bdba376930d962e5f122b7d0939d566ff71ca5250a73d0eee.
The old worker-lease receipt remains unchanged and is historical, not current.

## Requirement Coverage Boundary

Every Blueprint ID remains unfinished. This table is evidence interpretation,
not a second checklist or authority; only the Blueprint owns state.

| IDs | Evidence available | Still required before acceptance |
| --- | --- | --- |
| S2-001, S2-002, S2-003 | Frozen selector/specification; parser, projection and schema sources/tests | Full receipt/final-surface contract and prescribed per-item Master gates |
| S2-010, S2-012, S2-013 | Bounded frame, invite, pinning, device proof and ACL test sources | Current complete gate receipts, including real admission/revoke behavior where applicable |
| S2-011, S2-021 | Opt-in listener and full-screen QR implementation | Production listener, camera-read QR and cancel/refresh runtime evidence |
| S2-014, S2-015, S2-016, S2-017, S2-018, S2-019, S2-020 | Host-owned domain, routing, CAS, replay and local interop coverage | Full applicable production host/network/owner-race acceptance and receipts |
| S2-030, S2-031, S2-032, S2-033, S2-034, S2-035, S2-036 | Kotlin/Compose sources; prior source/JVM and Rust interop runs | Actual current Gradle app build/tests, device UI and pinned TLS against production host |
| S2-037, S2-038 | Keystore/revoke/lifecycle sources and connected test harnesses | Current connected Keystore, recreation, offline/network recovery runs |
| S2-050, S2-052 | Protocol/security adversarial test and fuzz sources | Complete prescribed current Rust/Gradle gates and applicable negative evidence |
| S2-051, S2-053, S2-054 | Production-host, network and parity harnesses | Actual production TCP/LAN prompt/stream/approval/cancel/replay and dual-client matrix |
| S2-055, S2-056 | API lifecycle/migration/backup/wipe test sources | Connected API 31/34/35 matrix, process death, backup/uninstall and migration behavior |
| S2-070 | Current attributed release and fresh local repeat evidence | Supported-host runtime, operator runbook/rollback drill and prerequisite acceptance |
| S2-071 | Packaging, signing configuration and read-only APK inspector | Current debug/release APK, operator signer, install/upgrade/reproducible signing |
| S2-072, S2-073 | Threat review, limits and user guide documents | Review against accepted runtime/release evidence and residual-risk sign-off |
| S2-099 | Current strict structural gate and controller status | All dependency/item gates, full Rust/Android runs, final receipts and scoped cleanup |

## Verification

- Blueprint receipt fixtures: 15 tests passed, including empty/partial receipt
  refusal, dependency closure, stale identity/digests, malformed/symlink evidence
  and boolean/float/string/nonzero exit statuses.
- Controller fixture suite: 15 passed; emitted digest/command fields asserted.
  Its existing integration fixture lacks a Gantt generator and logs that fact;
  this is not evidence of production completion-surface reconciliation.
- Stage 2 Python suite: 171 passed. Broader tools discovery: 265 total, 263
  passed, one failure and one error in existing runtime benchmark fixtures:
  time(1) cannot read kern.clockrate under this sandbox, so peak RSS is absent.
  The broader suite is not reported green and unrelated benchmark code is intact.
- Current strict Blueprint validation, cargo fmt --check and git diff --check
  passed. Controller has zero active/live/authenticated/request/queue entries;
  all 36 authoritative items remain unfinished.
- Android build verify-only still refuses missing APK provenance. No Gradle,
  ADB, camera or production LAN run was retried without external-state change.

## Next Actual Product Gates

Build the current debug and instrumentation APKs in an environment that allows
Gradle daemon TCP, then run the v7 physical harness on a dedicated freshly
installed API 31+ phone in the same private LAN. The harness requires actual
camera QR/SAS, bidirectional transcript updates, Activity recreation and
foreground replay, and self-revoke verified against the host registry. No
adb reverse or URI injection substitutes for that run. Follow with the named
API 31/34/35 lifecycle matrix and operator-signed release install/upgrade gates.
Full Rust tests and supported-host runtime still require their complete gates;
the locally reproducible binary does not remove those prerequisites.
