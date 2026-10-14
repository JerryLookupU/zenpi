# Durable Android Revocation Intent

## Progress

The preceding goal turn made verified device self-revoke progress. Current
inspection found that unknown revocation existed only in ViewModel memory,
so process/lifecycle restoration could hide it and start ordinary sync. This
turn advances S2-037/038 by atomically saving a canonical UUID intent in the
existing host/device-bound AES-GCM metadata record before sending self-revoke.
The same UUID is used on the wire. Storage failure prevents transmission.
Old records without the field remain readable; invalid intent types/UUIDs and
simultaneous workspace/revoke intents are refused.

Unknown or interrupted revocation retains the signing key, metadata, display
and intent. Cancel, rescan, network replacement, foreground restore and refresh
do not clear or resend it. Re-pair, new workspace writes, cache overwrite and
background resume admission are fenced at storage as well as the ViewModel/UI.
Existing background CAS proposals are invalidated by the intent write. Live
matching confirmation checks saved host/intent, removes the signing key first
and then metadata; key removal failure retains the marker. Explicit local
Forget abandons state but never claims host revocation success.

Cold-start invitation handling now waits for initial metadata loading before
accepting a QR URI. This closes a path that could bypass a stored pending
workspace/revoke marker before asynchronous restore completed. Cancel before
restore clears only the queued URI; it does not suppress metadata loading.
Forget no longer creates a replacement signing key merely to delete it.

## Evidence

Debug/release unit tests each passed 30 tests. Debug and instrumentation APK
builds passed. Expanded instrumentation checks unconfirmed connection failure,
same UUID restored in an offline ViewModel, cold-start URI fencing, repeated
network/cancel/rescan/refresh/revoke refusal, stale background CAS, wrong intent
completion, failed key removal retention and confirmed removal. Compose checks
scan is absent and refresh/revoke disabled while local Forget remains explicit.

The production private-interface desktop/Android harness passed seventeen
initial instrumentation tests, one actual process-death network recovery and
confirmed self-revoke test, two independent display-cache process fixtures and
two new independent revoke-intent process fixtures. The latter force-stop
between seed/restore and require the identical UUID supplied by the first
process, retained host/key and absent background resume snapshot. This proves
durable intent restoration, not a lost host-confirmation network scenario.

The authoritative current passing receipt is
`desktop-android-revoke-intent-verified-20261002.json`; it records
`independent_process_revoke_intent_verified=true` and all artifact hashes match:

- Rust: `beea4df4a79b09917aceadb4a0e8b8ed8cd8871085fa13d05e61a4da8a3dad82`
- APK: `8c1c2e324afe1c1e9e8530e8c7cb8a5b8f1f3aea93bf416a0f854b8dd8cf0d41`
- Test APK: `ee107e0adcb460bbc1721af666e88f8174e4caab6beef390c6c004a31a2c680e`

Two earlier receipts passed. A subsequent final run is preserved as failed:
the emulator had no route to the host (`ENETUNREACH`) despite enabled radio
settings. Its route was inspected, owned-emulator data toggled, host-interface
ping succeeded, then the entire unchanged final harness was rerun and passed.
No failing assertion was relaxed. The final screenshot was inspected; its
scrolled label area remains clipped and is not full responsive UI acceptance.
Strict blueprint validation, five blueprint tests, four harness tests, Python
compilation and whitespace checks passed. No Rust source changed this turn.
Owned emulator PID 22338/session 68731 exited; radios were enabled and reverse
mappings empty before shutdown. Temporary desktop/provider processes stopped.

## Remaining Full Scope

The saved intent is uncertainty, not a host receipt. A confirmed-revoke reply
lost on the network still requires desktop inspection because revoked keys
cannot use normal authentication to query a receipt. Exact revision/envelope
receipt recovery, real confirmation-loss/process-kill network injection, local
wipe partial-failure resolution and durable cross-process storage locking remain
open. Current synchronization prevents conflicting instances in one Android
process; no new claim of arbitrary multi-process concurrent writers is made.

Git crash reconciliation, worktree execution cwd/real scheduler quota, prompt,
approval/diff/PTY and live event/ACK parity, physical camera/LAN, API 31/34/35
matrix, signed/reproducible release and execution-controller acceptance remain
required. All 36 blueprint items stay unchecked; no commit, push, workers or
full-goal completion is claimed.
