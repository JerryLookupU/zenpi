# Approval And Terminal Subscription Wakeups

Provisional S2-020/S2-036/S2-054 progress. Previous turn added journal/provider
wakeups with verified tests. This turn wires existing approval lifecycle and
parsed desktop PTY output into those session-scoped notifications. Full QR/LAN/
Rust-desktop objective remains active; 36 Blueprint rows remain `[ ]`.

## Production Changes

Agent tool/attachment runtimes create ApprovalCoordinator with a shared session
notification scope. Registering pending approval, answering, caller cancellation,
cancel_all and failed-audit response retraction notify the session. Invalid
digest/identity does not change state or signal. Generic standalone coordinators
remain unbound. Notifications contain no tool arguments, answer or credentials.

Successful session replacement rebinds the shared notification scope and wakes
old/new watchers. A failed prepare leaves scope unchanged. The existing approval
coordinator is retained rather than replaced: owner control and host-held clones
must continue reaching the same rendezvous after resume. An initial draft
considered replacing the coordinator, but inspection of ProjectOwnerPool cached
control handles showed that would break ownership; final code rebinds only the
shared scope. Digest-fenced one-shot/audit-before-side-effect rules are unchanged.

TUI pump_shell notifies after real PTY output reaches the parsed bounded screen.
ensure_shell replacement notifies old/new scoped sessions. No raw screen text,
PID, path or input travels in the signal. Subsequent subscribed view still
checks device Read authorization, actual project/session/root and redaction.
No remote shell execution/input is enabled. Workspace/unpumped output remains
covered by the existing 250 ms fallback and bounded subscription limits.

## Verification

Observed source timestamp 2026-10-02T00:05:52Z; dirty worktree preserved.

- Initial targeted approval_owner/remote_approval_wakeup/remote_workspace run:
  **37 passed** (14/1/22). Workspace approval projection test now verifies
  pending and answered notifications while retaining privacy/non-consuming
  reads. New notification test exercises Agent attachment runtime, pending,
  wrong digest no-op, deny, persisted answer, failed resume no scope change,
  successful resume rebind, surviving host clone, cancel_all and caller cancel.
  Test worker handles cancel and join on unwind.
- Final expanded regression: approval_cap 9, core_session 39,
  remote_approval_wakeup 1, remote_parity 4, remote_subscription 10,
  remote_subscription_tls 7, session_recovery 45, stage1_session_tree 13:
  **128 passed across eight suites**. One independent-process tree helper is
  ignored in the parent suite but executed by its parent process tests.
- Focused TUI library test existing_terminal_projects_only_matching_live_session_
  and_root: **1 passed**, 366 unrelated tests filtered. It starts an actual
  owned PTY, emits split printf output, pumps real TUI screen, verifies scoped
  signal/foreign-signal silence and existing project/session/root refusal.
  Idle Agent::with_echo is owner setup, not a provider/LAN parity claim.
- `cargo check --offline --locked` passes with 19 existing library warnings
  and one vendored crossterm warning. Focused library build has eight existing
  warnings. Formatting differences in new blocks corrected; final fmt and
  whitespace checks pass.
- 44 Stage 2 Python tests pass. Strict Blueprint validation passes with all
  36 items unfinished; no acceptance checkbox advanced.

Commands used ZENPI_HOME=/private/tmp/zenpi-approval-wakeup-20261002 with offline
locked Cargo. All observed command handles terminalized. Android source and
artifact counts are unchanged; no new APK/device execution or TCP/LAN run is
claimed. Historical receipts remain unchanged. No nested agent, commit or push.

Source SHA-256 at verification:

```text
src/approval.rs bc004c8672d65df6e8ba0fae831328fd39dece9c06523d0de6e8ddf574618bf3
src/core.rs 2b31faff819a7c8ba1441b4875d179647e6a9804bddbfdabf6fe999611a5e849
src/tui.rs ae1a1a49bc6f1bae6156a672a4ffc0f3bcfcf289ed44c5d6b67778367379008f
tests/remote_approval_wakeup.rs faaa70e3cc163a0a9b29531bf88ac896e5662c3d1397485e3cd8c336700bf907
tests/remote_workspace.rs 5671c97d331790a60046bf3665063dd4b12923289ca98353c369706d568efa99
```

Provisional note, not checksum-valid Master acceptance. Remaining gates still
include fresh APK/host authenticated subscription screenshots, physical phone
QR/LAN, real socket shutdown/deadline matrix, full stream event/backpressure,
worktree executable scheduling 1..64, interactive PTY/bulk, supported API
lifecycle/reset/Keystore, migration/uninstall and signed release/security.
