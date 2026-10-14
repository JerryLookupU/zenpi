# Stage 2 Worker Cancellation Metadata Admission

Recorded 2026-10-04T09:38:50Z; partial S2-036/S2-054 evidence, not acceptance.
Previous goal turn was progress: summary/saved-scope worker history/detail
metadata admission repaired. Current dirty worktree preserved; no nested
agents, reset, push, repeated denied Gradle/ADB attempt or permission bypass.
Blueprint remains 36 [ ], zero [_]/[x].

## Finding And Change

The preceding read repair introduced a complete WorkerStatusCodec display
predicate. Fresh worker cancellation still accepted summary targets based on
scope and canCancel alone, and command construction likewise did not require
the complete metadata predicate. Damaged subtab, failure/output evidence on a
running task or malformed live preview could therefore authorize a fresh
cancellation even though the same target was invalid for retained task reads.

WorkerTargetPolicy summary selection and both WorkspaceCommand.cancelWorker
construction paths now require the shared display predicate. Dispatch already
uses WorkerTargetPolicy and thus refuses the same damaged target. Page-only
selection keeps its complete metadata-only page/chain gate. Healthy accepted/
running bounded stream previews and unknown-outcome tasks with runtime evidence
remain cancellable, independent of new-task submission tabs.

An exact already-persisted cancellation still takes the existing pending-request
reconciliation path: original ID/envelope only, no replacement request. Scanner,
reset, revoke, foreground and network fences remain. No schema/host authority,
automatic write retry or cache change was introduced.

WorkerTargetPolicyTest adds invalid-summary construction/dispatch and saved
retry regressions, plus healthy live-preview/runtime admission without tabs.
AndroidRustWorkerCancelTest mutates actual running-task metadata at client
preflight and refuses selection/construction/dispatch while preserving an exact
saved request. The unchanged surrounding lane submits two actual Agent workers,
cancels only the target, lets its sibling finish and restores durable receipts
without starting another worker. Mutation negatives are client evidence, not
additional host-dispatched cancellations or physical Android/LAN behavior.

## Verification

- Source checker /private/tmp/stage2-worker-cancel-metadata-20261004: process
  32945 polled through exit 0; 586 ordinary JVM tests, main/unit/instrumented
  compilation and every explicit Rust/Kotlin lane passed, including actual
  running-worker cancel and six-test model/read/worker target.
- Python Stage 2 tools: exit 0, 145 tests.
- Lifecycle validator: 25 fixture tests pass, not connected API behavior.
- Blueprint fixtures: five pass; strict validator: exit 0, marks unchanged.
- cargo fmt --check and git diff --check: exit 0.
- Host debug build and verify-only after Cargo lanes: exit 0.
- Android verify-only: exit 1, missing APK provenance. No repeat of the
  authoritative denied daemon local-listener build attempt.

```text
WorkerTargetPolicy.kt a71a7e36ebedd3fa48b31857660321d6cb11482caa35da8afed006cd4d373967
HostClient.kt 37ccaaf5a2cbbc8b815f6acada385527cfa968b695185d5cd764f5c7af0df489
WorkerTargetPolicyTest.kt 008f6d12a9cb2deee410108d16ae6c950c62f5a38a2196dee495ef7b1dbc341a
AndroidRustWorkerCancelTest.kt 82bf66d8f895e6240a85ec0dbe8bed2f5f9c0d9b02169b8748485638f44b71d3
source-log 326ee70573ebc1463fc7c7ffaed82ca0ac7f30f0a034bb5fe4adaf2524c37dee
python-log d82049b8d4a677f6413a359dedc6a89a96ef162b00dde95b556c66f10b2f626f
worker-cancel-results 529deedb2364f1280db857fd9154eebd906b31dfcd414f3ca7f2ca7f97091b7a
main.jar 413bd027f6f71993cd49c4eda1e2d46c45272ad222bf3b2ae8b24080f11418a4
unit.jar a615bb3db0b640b85fd10bdc722f6d20595806864ec259c6440b98e32438d119
instrumented.jar f95a635bb1b8d33d66febe3cf15afe0db9afd4ca9d63fb00a5f126e9d1e69630
host-receipt 1bf65f4408ba0568c0bb87f4d88a067130ad5fbea8e6531b52c210b3ad12a236
host-inputs 8ccf956a10a2fcac07b143c4d28aa8d62d1f59af1b5623a338da499e0b09906b
host-binary 40f9f134ed62fd51a71dc6e8d24022f0122dced68faf12f6a52f8b5ea145607f
```

Goal active/incomplete. Current APK attribution, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete parity
and signed reproducible release remain required. This repair is progress,
not product completion or a declaration that further work is blocked.
