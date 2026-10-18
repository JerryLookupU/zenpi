# Host-Owned Stable Subtab Domain

## Implementation

S2-016 now has a typed host-owned checkpoint domain in `src/remote/subtab.rs`.
Schema version 2 defines project-keyed ordered tabs, stable ID selection and
monotonic per-project revision. IDs derive from versioned project/kind/root
identity rather than names or view indices. Rename, reorder, selection and
concurrency edits preserve identity. The main tab stays first; duplicate
identities, roots and names, invalid selection, unknown schema/fields, invalid
paths, and concurrency outside 1..64 are rejected. The existing 32-subtab cap
applies. Revision overflow fails rather than wrapping.

ProjectOwnerPool persists these domains in the same bounded atomic CAS
checkpoint as projects, sessions and workspace retry receipts. Legacy owner
checkpoints without the field remain readable. Project publication retains
other domains and removes only closed projects. Session new/resume writers
carry the domain forward. Stale host checkpoint writes do not overwrite a
winner or publish the losing in-memory state.

Production desktop startup imports old TUI layer-2 tabs and active indices
once, then projects host IDs back into view indices. Later starts ignore stale
UI layer-2 state in favor of the committed host domain. The production event
loop publishes local view changes before processing remote reads; shutdown
also flushes final edits. A failed subtab commit restores the prior view and
reports an explicit checkpoint rejection. Pane/draft state stays in the UI
checkpoint, not this domain.

## Evidence

Eight `remote_subtab` tests cover deterministic migration, rename/order identity,
stable selection, concurrency bounds, capacity and overflow, malformed restore,
atomic commit/restart, two-host conflict, project close retention, real desktop
migration/local edits/restart and desktop rollback after external checkpoint
change.

Six workspace/session suites passed: 85 tests passed, one existing test ignored.
Eight remote protocol/auth/event/listener suites passed: 39 tests passed.
`cargo build --locked --offline` passed with existing warnings; formatting and
diff whitespace checks passed.

`desktop-android-subtab-domain-20261001.json` records a passing production-binary
regression through the private host interface without adb reverse: ten initial
instrumentation tests, one post-death restore test and two independent cache
process tests. Existing pairing, transcript replay, workspace buttons, network
loss recovery, WorkManager new-turn read, pending-write recovery, revocation
and listener teardown remain functional with the expanded owner checkpoint.
This run verifies regression, not Android secondary-tab functionality.

## Remaining Scope

S2-016 remains provisional. The desktop input/render boundary still uses
indices; remote subtab command routing, operation receipts and revision-fenced
wire results are not implemented. Git creation/removal/move safety, dirty-tree
and busy-owner handling, and failure reconciliation remain S2-017 work.
Scheduler quota/admission behavior needs S2-018 validation; the domain's
concurrency values alone do not prove no N+1 worker. Android secondary tabs
and stepper remain S2-035 work. Existing planned legacy in-place roots may be
absent; no path-validation result here authorizes a destructive operation.
No remote subtab capability was enabled by this change.

Physical phone camera/LAN, complete session parity, push/ACK/backpressure,
Doze/system scheduling and API 31/34/35 coverage remain open. All 36
authoritative blueprint items remain unchecked. No commit, push, item
acceptance or goal completion is claimed.
