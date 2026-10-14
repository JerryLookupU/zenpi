# Android Steering Unknown-Outcome Polling

Provisional Stage 2 progress at 2026-10-02. No blueprint checkbox was
promoted; this is an uncommitted worktree change.

The Rust host intentionally records a remote steering request as
`unknown_outcome` before its sole session writer durably acknowledges the
queued input. The Android client previously stopped immediately when it saw
that typed state, leaving a valid queued steer displayed as an unresolved
failure. `HostClient` now treats the typed unknown outcome as a bounded poll
condition for steering only (the same request envelope and request ID are
replayed, so the host receipt remains idempotent). Workspace mutations other
than steering still stop immediately on unknown outcome and require explicit
resolution, preserving the no-automatic-retry rule for non-idempotent writes.

The cached worker-status pagination fix from the preceding turn remains in
the same source check. A real Android device and LAN run is still unavailable
because adb cannot start its local server in this sandbox.
