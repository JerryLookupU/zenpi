# Session-Driven Subscription Wakeups

Provisional S2-020/S2-036/S2-053 progress. Previous goal turn added verified
encrypted subscription transport. This turn connects real journal/provider
updates to subscription wakeups rather than waiting unconditionally for the
250 ms sample interval. Full QR/LAN/Rust-desktop objective remains active;
all 36 Blueprint rows remain unfinished. No nested agent, commit or push.

## Implementation

New remote/wakeup.rs is a content-free counter plus condition variable per
watched session. Weak-reference registry is capped at 128 live session entries;
watch calls prune expired entries, same-session subscriptions share one signal,
and notifications never create entries. No messages/credentials/body queue or
durable cursor is stored. Capacity exhaustion falls back to timed sampling.

SessionStore emits only after successful append and in-memory commit of turns
and opaque events. Invalid/conflicting/idempotent turns and failed writes do
not report a new durable change. LivePromptText updates/removals (also desktop
preview through its existing wrapper) and PromptProgress transitions notify.
Subscription captures counter before authorized reads and waits while unchanged
after output. Changes during sampling or waiting are not lost. All wakeups still
rerun current-device/project/session authorization; signal is not authority.

Existing 250 ms timeout remains for approval/PTy/workspace paths not yet wired,
and external process changes. Socket deadline/duration/frame/byte limits remain.
Added 32-batch completion cap aligned with Android's reducer so rapid updates
cannot overflow its batch bound. Multiple source updates coalesce rather than
queueing raw provider deltas. This improves event-driven latency but is not
complete callback-driven delivery for every display class or durable delta ACK.

## Verified Results

Observed timestamp 2026-10-02T00:00:28Z, dirty canonical worktree preserved.

- Focused wakeup library tests: **2 pass**, 365 unrelated tests filtered.
  Tests cover scope, invalid ID, shared signal, exact capacity/reclamation,
  change-before-wait/change-during-wait and unchanged zero-duration wait.
- Remote integration regression: events 4, faults 3, headless 1, live text 5,
  protocol 11, subscription 9, encrypted TLS 7: **40 pass across seven suites**.
  Final focused subscription rerun after adding failed-append notification case:
  **10 pass**. These include actual durable session updates, invalid/idempotent
  turn no-op, provider/progress notifications, independent scope, continuous
  changes bounded at exactly 32 batches and prior encrypted adversarial cases.
- Core/session/runtime regression: core_session 39, runtime 12, session_recovery
  45, stage1_session_tree 13: **109 pass**, one process-helper entry ignored in
  the parent suite and executed by its independent-process tests. Tests cover
  journals/recovery/sequence exhaustion, owner/cancellation/runtime semantics,
  branch migration/selection and competing-writer/fork publication behavior.
- `cargo check --offline --locked`: passes with 19 existing library warnings
  plus vendored crossterm warning. Library-test build has eight existing warnings.
- 44 Stage 2 Python tests pass; strict Blueprint, formatting and whitespace
  validation pass. 36 `[ ]`, no acceptance mark advanced.

Commands used isolated ZENPI_HOME=/private/tmp/zenpi-wakeup-tests-20261002 and
offline locked Cargo. Focused library invocation was `cargo test --offline
--locked --lib remote::wakeup::tests`. Remote invocation named the seven suites
above; core invocation named session_recovery/core_session/stage1_session_tree/
runtime. No Android source changed, APK rebuilt or instrumented device test run
in this turn. Previous Android direct-compile/JVM evidence is not expanded into
a device acceptance claim. All observed command handles terminalized.

Source SHA-256:

```text
src/remote/wakeup.rs d40dc1db83f974b33a9a926273e8bb479d3516547cca570fce989a1641e359a8
src/remote/subscription.rs 72f1386b113e35c09605817f0c5ab8152a0e68161fe178358359eca95dc72bb4
src/remote/live_text.rs f7a3284ecfdea35ba86f8c31442cf7248e77be8ae069bc3b9feaa3b4438c940d
src/remote/command.rs 1a432f45fb3399554a3b3a1b6f8f271ded9c6b95250b4ae312b3ee898628d587
src/session.rs a28d877f242980617f42b3a45729070c8b9dc304bd131529885704e46b1d4483
tests/remote_subscription.rs 3e41bc6ea6f6dcc1563e6836c37774962bca45ee72309ea27c7f88bda0d8aa34
```

This note is provisional evidence, not a checksum-valid Master acceptance
receipt. Real socket/LAN/new APK/camera gates remain unproved in the restricted
environment. Remaining final scope includes full event sources/backpressure,
executable per-worktree 1..64 scheduling, interactive PTY/bulk, supported API
lifecycle/Keystore/reset, migration/uninstall and signed release/security.
