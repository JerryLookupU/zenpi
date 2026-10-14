# Foreground Network Callback Progress

Provisional Master implementation for S2-038/053. The original Android/LAN/QR/
desktop objective remains active and no full Blueprint item is accepted.

## Change

The Android Activity registers a ConnectivityManager default-network callback
only while started, with ACCESS_NETWORK_STATE permission. Initial route state
is published synchronously before foreground work can start. Callback delivery
is marshalled to the main thread and generation-fenced after stop, preventing
late delivery from restarting background work. Duplicate default-route events
do not reconnect, and late loss for an older network cannot invalidate a newer
default route. Route presence is used rather than Internet validation so an
offline LAN can still host Zenpi.

The production ViewModel fences and cancels its owned network operation on
route loss or replacement. Loss preserves transcript/cursor and displays
offline/stale state; manual refresh does not start an operation without a route.
Route recovery opportunistically reauthenticates foreground reads. A pending
write is preserved and stays offline for explicit same-ID retry, never automatic
submission. Interrupted pairing retains an unknown-outcome error rather than
remaining stuck at Connecting or silently retrying its one-use invitation.

## Behavioral Evidence

Passing receipt: `desktop-android-network-callback-final-20261001.json`.
The real API 36 emulator starts a production ViewModel and the same callback
implementation used by MainActivity. Its host session is the production Rust
desktop owner reached over adb reverse. Instrumentation snapshots the initial
Wi-Fi/mobile-data settings, disables both through the emulator shell, waits for
an actual callback route loss and requires Offline state. Cached display and
durable event cursor must remain unchanged. Manual refresh must stay Offline.
It restores the original radio settings, waits for a real default route and
requires Connected with the same session/cursor, without rescan. It unregisters
the callback and backgrounds the model; finally also restores radio settings
on failure.

The same sequence passes workspace wire control and production Compose button
control with desktop checkpoint inspection, explicit invalid-directory refusal,
foreground transcript replay, encrypted cache restore, proved-live process
death, durable pending-request recovery, host revoke and listener shutdown.
Pending recovery additionally simulates loss/replacement through the ViewModel
entry point and proves no pending envelope is cleared or automatically replayed.
That supplementary policy check is not represented as a real network switch.

Seven initial instrumented tests pass, plus one process recovery and two cache
runs. Android Debug/Release unit suites pass 21 each, including two default-route
ordering/duplicate/loss tests. Gradle APK/test build, Python harness compilation,
strict 36-item Blueprint validation, five tooling tests and diff checks pass.
Wi-Fi and data return to their pre-test enabled state. The production host and
adb port mapping are cleaned by the harness.

## Remaining Scope

Real callback state loss is proven, but adb reverse does not prove loss of the
actual LAN TLS route. Physical phone Wi-Fi/cellular/LAN/VPN switching, address
change, camera scanning, offline-only LAN routing and callback registration
failure/recreation matrix remain required. WorkManager/background recovery,
Doze, API 31/34/35 and release behavior remain unfinished. No FGS or persistent
background socket is introduced or claimed.

Worktree/concurrency, remote prompts/approvals/diff/PTY, full fault/migration
matrix and all other original deliverables remain in scope. All 36 Blueprint
items are unchecked, no execution worker or cron is claimed live, and no
commit/push is made.
