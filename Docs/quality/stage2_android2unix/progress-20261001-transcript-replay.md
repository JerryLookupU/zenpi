# Transcript Replay Progress

Provisional Master implementation; the full Android/LAN/desktop goal remains
active and no Blueprint item is accepted by this evidence alone.

## Implemented

- Separate durable transcript event cursor, reconstructed from validated
  committed session turns rather than confusing opaque journal sequence with
  remote event sequence. Unknown/private journal records and turn metadata are
  excluded. Content still passes existing redaction.
- Authenticated Read-authorized Resume through the shared owner mailbox.
  At most 128 ordered event frames and 96 KiB per page, 256-turn retained
  window, explicit snapshot for old cursors/oversize turns/tree sessions.
- Terminal response binds cursor, scope and completion after event frames.
  Android only updates after contiguous owner/sequence/terminal checks pass;
  invalid/truncated replay never advances the old projection.
- Foreground ViewModel automatically attaches and pulls committed transcript
  updates, with bounded fetches/backoff and cancellation on background. No
  manual Refresh is needed to observe a new desktop answer in the test.

## Verification

`remote_events` covers reopen-stable cursor, opaque event exclusion, page
limits, empty replay, old/future/foreign cursors and oversized events.
`remote_headless` checks actual adapter event frames, project denial and a
258-turn gap yielding a bounded 64-turn bulk snapshot. Android reducer tests
cover contiguous application, duplicate/out-of-order/foreign messages,
terminal cursor mismatch, no-progress pages and snapshot replacement.

The production PTY/Android harness requires the foreground_replay_verified
marker, not only a green instrumentation summary. A ViewModel restored from
encrypted paired metadata enters foreground, automatically fetches initial
state, then observes a new real desktop provider answer as replay events. The
test verifies unchanged session, increasing journal/event cursors, new turns,
nonzero replay event count, offline transition on background, Compose display,
host revocation and listener teardown. It is adb reverse, not actual LAN or
camera evidence. Receipt: `desktop-android-replay-20261001.json`, screenshot
with the same stem. Ordinary snapshot mode remains separately tested.

Validation: 39 Rust remote tests passed; Android Debug and Release each passed
13 unit tests; API 36 instrumentation passed three tests including the real
foreground ViewModel replay path. APK/test APK assembly, formatting, strict
36-item Blueprint validation and five Blueprint tooling tests passed. Existing
unrelated Rust warnings remain.

## Required Follow-On

This event projection covers committed transcript turns, not in-flight model
deltas, tool progress, approvals or all TUI-facing state. Durable ACK/cache,
NetworkCallback/WorkManager/process death and API 31/34/35, live push, remote
mutations with CAS/dedup, worktrees, LAN/camera and release acceptance remain.
Controller workers/cron are not active. No commit or push is made.
