# Headless Network Runner Progress

Partial S2-014/S2-019/S2-054 implementation. The full Android/desktop goal
remains active and all 36 authoritative checklist items remain unfinished.
The preceding user turn verified the installed RTK/Rust environment only;
this continuation implements a missing production command path. No commit,
push, nested agents, controller worker launch or Android source change.

## Runtime Change

Non-TUI `host pair --workspace ... --session ...` previously exposed reads
without a model command runner. It now owns a bounded mailbox service thread
and the existing BackgroundRunner, using the existing ProjectOwnerPool Agents.
It does not create a second session writer or parallel worktree pool.

Prompt admission reuses the checkpoint binding, exact-envelope deduplication,
ACL lock and journal reservation before submission. Execution reuses the
captured SubtabExecution and StartIfIdle Agent pipeline. Workspace listing
advertises Control only for locally granted checkpoint-backed projects and
projects device-scoped live/durable progress. Steering, cancellation and
approval use the existing adapters. An active captured job prevents its project
or subtab from closing, including the queued-before-Agent-lock interval.
Unauthorized close requests still fail authorization rather than exposing Busy.

Listener shutdown/expiry stops admission, requests cooperative cancellation,
and joins the service and bounded runner. An interrupted final record remains
unknown rather than claiming success. The existing non-cooperative detach
limitation is unchanged. Normal TUI and stdio modes do not start listeners.

## Evidence

`tests/remote_headless_commands.rs` covers:

- Actual production binary, configured Responses HTTP provider, pinned TLS 1.3
  and fresh signed device authentication, with a local Control grant. A network
  prompt reaches the provider and original journal, exact retry stays deduped,
  durable status succeeds, and listener expiry closes the socket.
- Real HTTP Chat Completions/tool turn reading CHILD_ONLY from the selected
  directory, not MAIN_ONLY; transcript and original workspace restoration;
  Read-only refusal, changed-envelope conflict and restored-host exact retry.
- Explicit controlled blocking backend for cancellation races, not claimed as
  HTTP evidence: another device cannot cancel, another prompt is not admitted,
  original cancel is idempotent, and cooperative shutdown releases the owner.

Seven remote suites passed 48 tests. Eight additional/shared suites passed 172
tests with five ignored helper/existing cases. Two command unit tests passed.
The HTTP test helper explicitly restores accepted sockets to blocking mode:
an earlier broader run exposed macOS accepted-socket WouldBlock and failed;
that failure was not promoted to passing evidence. Corrected broad regression
passed. A final narrow rerun covers the added active-close authorization asserts.

Python harness evidence tests passed 14; Blueprint tests passed five. Strict
Blueprint and controller validate/status passed, with zero active claims,
transports, handoffs or outstanding requests. Format/diff checks passed.
Existing compile warnings remain. No Android emulator or physical phone test
was run for this new host mode; prior APK/LAN receipts are not relabeled.

## Remaining Requirements

Per-tab concurrency remains configuration/display only and is not advertised
as executable Android control. Independent worktree owners/quota, Android
headless-mode instrumentation, attachment/resource parity, PTY/diff/bulk,
live push/durable ACK, broader crash recovery, API 31/34/35 lifecycle matrix,
physical camera/LAN and release/security gates remain unfinished. This runner
does not establish whole-item or whole-goal acceptance.
