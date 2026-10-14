# Pending Write Recovery And Production Buttons

Provisional Master implementation for S2-033/034/037/038/053/056. The complete
original Android/LAN/QR/desktop objective and all Blueprint gates remain active.

## Implementation

Android persists the exact bounded workspace request in the same encrypted
no-backup host/display record before sending it. It retains the original
request ID, project/session, owner generation, revision and strict arguments.
The request object is immutable; restoration rejects unknown fields,
unsupported operation families, wrong types, invalid owner and invalid paths.
No token, signing key or provider credential is stored in that record.

Read caching preserves any unresolved request. Beginning a different request,
changing its host, resolving a different envelope or pairing over the record
is rejected. Success clears it atomically with the new display/cursor; a
correlated explicit write refusal clears it without claiming success. Transport
or authentication/read errors leave it pending. Cancellation and re-scanning
preserve it, and process startup restores it offline without automatic sending.
Only explicit retry submits the original envelope; Forget wipes the record.

The ViewModel commits local pending state before invoking the production TLS
client and persists its resolution before publishing the confirmed projection.
Failure to persist pending state means the request is not sent. Failure to
persist resolution leaves the pending request available for recovery.

## Production Evidence

Passing receipt: `desktop-android-pending-write-verified-20261001.json`.
The test renders the normal PairingScreen bound to the real PairingViewModel,
clicks Open, enters a host directory, confirms, then clicks and confirms Close.
No injected HostClient or fake callback supplies these operations. The harness
checks the production desktop checkpoint after each action, and the ViewModel
and encrypted store must clear their pending state. An invalid directory then
returns explicit invalid_request, leaves the initial project selected, and
clears the refused request without success.

For the controlled crash boundary, a live Android process saves a workspace
selection request, commits it on the real desktop, and deliberately leaves its
pending record unresolved. The harness proves that process identity, force-stops
it and checks the app PID is absent. In the passing run the killed PID is 4663
and restored PID is 4721. Android is stopped while another desktop provider
answer commits. The new process must restore the pending envelope, preserve it
through Cancel/rescan, explicitly retry it, receive ordered missing transcript
events and atomically clear the request. A further identical request must not
advance workspace revision. Host revocation and desktop listener shutdown also
pass. The harness requires the pending recovery marker in control mode, so
silently losing the record cannot produce a green result.

Two additional independent processes validate encrypted fixture pending/display
restore, pending preservation through read caching, different-request and
host-change refusal, matching resolution and key/record wipe. The full test
sequence passes six initial instrumentation tests, one process recovery test
and two cache tests on API 36. Android unit tests pass 19 per Debug/Release
variant, including strict immutable pending envelope restore. Twelve related
Rust owner/auth tests pass. Strict Blueprint, five tooling tests, Python harness
compilation and git diff whitespace checks pass.

An earlier failed receipt is preserved. The reducer had already restored the
expected transcript but was in its subsequent automatic CONNECTING phase when
the assertion ran; the wait predicate now requires both CONNECTED and resolved
pending state, not merely the expected text. No success assertion was removed.

## Remaining Requirements

This remains adb-reverse emulator evidence, not actual camera/LAN acceptance.
Crash-before-send, partial network submission, lost reply, host restart,
revocation during ambiguous recovery, corrupted pending storage, explicit reset
and upgrade/backup/uninstall cases need the broader fault/migration matrix.
The selection crash boundary must be extended to destructive and runtime write
families as they are implemented. Worktree/concurrency, prompt/approval/diff/PTY,
NetworkCallback/WorkManager, API 31/34/35 and release gates remain unfinished.

All 36 Blueprint items remain unchecked; no worker/cron is claimed live, no
commit/push is made, and the complete original goal remains active.
