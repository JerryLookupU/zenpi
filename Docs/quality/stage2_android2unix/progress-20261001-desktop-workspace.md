# Desktop And Android Workspace Control

Provisional Master progress for S2-015/019/034, not full item acceptance or
completion of the original Android/LAN/QR/desktop synchronization objective.

## Implemented

The desktop owner-thread mailbox now admits workspace open/select/close through
the same ProjectOwnerPool as local actions. Before close it rejects projects
with the active runtime job, queued TUI inputs, pending owner input controls or
session-replacement work. Pool publication still checks actual main/arch owner
locks and input preparation. Successful publication reconciles stable-ID TUI
tabs, per-project drafts, active metadata and owner, then refreshes resource and
Gantt projections. No second desktop owner is introduced.

Queued requests use an atomic claim/cancel state. An unstarted request that
times out cannot dispatch later. A claimed write that exceeds the caller's
two-second deadline yields unknown_outcome; its receipt can subsequently be
resolved with the original request, not an invented new ID. The mailbox remains
eight slots and drains at most eight requests per desktop iteration.

Live project listings advertise workspace_control only for explicit Control
grants and a durable pool checkpoint. Android builds an immutable request with
the observed owner, revision, project and session. Tabs select the host project
for authorized control clients; Read-only clients retain client-side selection.
Open and close dialogs require explicit actions. Unknown write outcomes disable
new controls and retain a same-ID retry action. A read refresh does not clear
that ambiguity. Cached display intentionally strips mutation authority until
a live authenticated attach. Generic mutations_available remains false because
other operation families are not yet admitted.

## Evidence

`desktop-android-workspace-ui-final-20261001.json` is the passing receipt for:

- production Rust desktop PTY and independently decoded terminal QR;
- explicit local Read/Control grant for the initial and canonical child IDs;
- Android TLS workspace open followed by real desktop checkpoint inspection
  proving the active child tab exists;
- identical retry with unchanged revision and no duplicate tab;
- stale revision refusal;
- Android close followed by desktop checkpoint inspection proving the child is
  gone and the original project active;
- subsequent desktop provider updates, foreground transcript replay, encrypted
  cache restore, proved-live Android force-stop and new-process cursor replay;
- host revoke refusal and desktop listener shutdown.

Five initial instrumentation tests pass, including Compose stable-ID selection,
empty-directory disabled state, open/close confirmation and unknown-result
control disabling/same-request retry. One network process resume test and two
independent cache tests pass on API 36. Android unit suites pass 18 tests per
Debug/Release variant. A failed intermediate UI receipt is preserved; it tested
the unmerged Text child rather than its disabled Button, so the selector was
corrected to the merged actionable node without weakening the requirement.

Rust regression: 136 passed, one intentionally ignored child entry in ten
suites. Seven remote_workspace tests now include real TUI owner/draft
reconciliation, admitted-close refusal, expired unstarted writes and claimed
timeout/receipt resolution. Six other remote protocol/transport/event suites
pass 34 tests. Build, formatting, diff, strict Blueprint and five tooling tests
pass. The screenshot shows the rendered authorized workspace controls and
transcript on the narrow emulator; the partially clipped Refresh button is the
normal scrolled content edge below the fixed top bar.

## Unfinished Scope

The network tests still use adb reverse, not actual LAN routing or camera
scanning. The wire writes and Compose controls are separately proven; real
production ViewModel-driven button-to-host end-to-end mutation, durable pending
write recovery across process death, full busy/conflict/unknown fault matrix,
worktree actions, prompts/approvals/diff/PTY and shared-schema generation remain
required. No automatic retry or unsafe claim is substituted for these gates.

All 36 Blueprint items remain unchecked. No workers/cron are claimed live, no
commit/push is made, and the full original goal remains active.
