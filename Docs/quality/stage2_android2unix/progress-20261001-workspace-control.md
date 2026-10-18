# Workspace Control Progress

Provisional Master implementation for S2-015 and write fencing needed by
S2-019/034. This is not full item acceptance or completion of the original goal.

## Change

`src/remote/workspace_api.rs` uses the existing `ProjectOwnerPool` to implement
workspace open/select/close in the shared-pool adapter. It validates the exact
arguments, project/session identity, explicit local ACL, pool-generation owner
and current workspace revision. Canonical path opening also needs Read/Control
authorization for the resulting stable project ID; it cannot grant access by
choosing a path. The selected replacement after close must be authorized.

The pool persists revision and bounded successful-request receipts in the same
compare-and-replace checkpoint as workspace/session mappings. Exact retries
return the original result, including after pool restart. Changed content with
the same device/request ID is rejected. A new pool generation fences old fresh
requests. Local publication and session replacement use the same revision.
Legacy checkpoints without this state migrate on commit. Malformed receipt
records fail restore. The 128-entry ledger never evicts an ambiguous retry:
at capacity it refuses new request IDs while existing retries remain usable.

Device revoke/grant and mutation commits share the registry's cross-process
lock. A locked authorization view avoids reacquiring that non-reentrant lock
inside the transaction. Pool publication now holds replaced main/arch owner
locks across checkpoint commit, refuses running/pending/preparing owners, and
closes the removed handles only after commit. Failure preserves the tab and
durable checkpoint. Selecting an existing running owner needs no Agent lock.

The standalone host session attachment enables/restores its durable pool
checkpoint. The local CLI adds `--grant-device-control ID --project PROJECT_ID`
for explicit Read/Control grants, never Approval. Duplicate grant flags fail.

## Validation

Five `remote_workspace` tests cover canonical open/select/close, duplicate
retry with byte-identical checkpoint, request-ID conflict, restart receipt
replay, old owner fencing, local revision conflicts, two-client CAS with exactly
one winner, Read-only refusal, ungranted target path refusal, foreign session,
external checkpoint conflict, revoke refusal, ledger capacity and legacy or
malformed checkpoint restore. Main/arch busy close preserves the checkpoint.
The shared adapter's correlated success dispatch is also exercised.

Regression command:

```sh
rtk cargo test --locked --test remote_workspace --test remote_tui_pairing --test remote_auth --test remote_headless --test project_workspace --test headless_project_workspace --test tui_project_workspace --test headless_protocol --test session_new --test resume_compact_owner
```

Result: 134 passed and one intentionally ignored child entry in ten suites.
Six other remote transport/protocol/event suites pass 34 tests. `cargo check
--locked`, debug build, `cargo fmt --check`, `git diff --check`, strict Blueprint
validation and five Blueprint tooling tests pass; existing warnings remain.
The production desktop/Android cached replay and forced-process-death suite is
rerun against the changed host; its scoped receipt is
`desktop-android-workspace-final-20261001.json`. It still uses adb reverse.
One earlier regression run exposed a grant/revision race: an already
authenticated ungranted request could lose its device revision just before
emission and receive EOF. The host now sends a correlated unauthorized terminal
response before closing. The test's grant-wait loop tolerates only that explicit
revision refusal, not arbitrary transport failures. The earlier failed receipt
is retained as `desktop-android-workspace-regression-20261001.json`.

## Remaining Integration

The desktop owner-thread mailbox still denies write requests. TUI view
reconciliation and scheduler-admitted work fences must be integrated before
enabling those requests. Android workspace actions and pending/conflict/unknown
outcome UI remain required. Read responses deliberately continue to advertise
mutations_available:false. Busy failure taxonomy, ledger rotation/retention,
full write command parity and broader fault/security tests remain unfinished.

Real camera scanning/LAN routing, worktree control, prompts/approvals/diff/PTY,
complete lifecycle/API matrix and release gates remain required. All 36 items
remain unchecked; no workers or cron are represented as live. No commit/push
is made, and the full original goal remains active.
