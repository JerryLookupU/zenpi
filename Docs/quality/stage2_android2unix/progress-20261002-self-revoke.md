# Android Device Self-Revocation

## Progress

The preceding turn made verified secondary cache progress. This turn advances
S2-013/037 by adding a host-backed device self-revoke action instead of treating
local key removal as host revocation. `device_revoke_self` is authenticated and
fenced by the current device ID/revision. Authentication now returns the device
registry revision. Empty arguments and absent project/session/turn/sequence
are mandatory. No project grant is needed, no other target can be supplied,
and stale authenticated handles fail after a permission revision change.
The registry lock covers revision validation and durable revocation. A terminal
confirmation is written before connection closure and bypasses only the
ordinary post-dispatch current-device check that would otherwise suppress its
own successful revoke response. It does not bypass authentication. The path
also exists without a workspace adapter. Registry storage errors report unknown
outcome, not guaranteed absence of a side effect.

Android provides a separate Revoke device on host confirmation dialog and
labels local-only Forget explicitly. Production ViewModel uses the existing
pinned TLS, signed connection challenge, bounded framing and 15-second owned
socket cancellation path. Matching host confirmation precedes local encrypted
metadata/key removal. Pending workspace requests prohibit revocation. A failed
or unknown host response retains credentials and reports unconfirmed status;
there is no automatic retry. Removed keys are not recreated for revocation.

## Evidence

Six Rust suites passed 36 tests, including a new registry/dispatch test with
two devices, wrong target, extra arguments, wrong revision, project binding,
stale authenticated handle, no-project-grant self-revoke, durable restart and
revoked challenge refusal. Rust build passed with no errors and 20 existing
warnings. Formatting, whitespace checks, strict blueprint validation, five
blueprint tests, four harness tests and Python compilation passed.

Android debug/release unit tests each passed 30 tests; debug and test APK builds
passed. New Compose coverage checks confirmation/cancel separately from local
forget. A connection-refused production ViewModel fixture proves unconfirmed
revocation keeps saved metadata and the same Keystore signing key.

The production desktop/Android direct-private-interface harness passed fifteen
initial instrumentation tests, one actual process-death resume/revoke test and
two independent-process cache fixtures. After restoring a killed process and
resolving the exact pending workspace request, the test clicked the actual
production self-revoke confirmation. It required the ViewModel's confirmed
status, absent local metadata and absent signing alias. The harness independently
read the durable host registry and required `revoked=true`; it did not perform
the CLI revocation for this successful path. Current artifact digests match
`desktop-android-self-revoke-current-20261002.json`, which records
`android_self_revoke_verified=true`.

- Rust SHA-256: `beea4df4a79b09917aceadb4a0e8b8ed8cd8871085fa13d05e61a4da8a3dad82`
- APK SHA-256: `cba4be826f35e7c77c31017e9acc408082fc8ec4627bb095b783339869885879`
- Test APK SHA-256: `4faf53088aef987ef192a953519fe549057be136c2f6c639e8b4e2dd194a3d80`

The first harness receipt is preserved as failed. Its first test still searched
for the old local Forget label, then fixture cleanup caused later dependent
tests to lack their saved host. The selector was updated to the intentional
new local-only label; the full harness was rerun rather than weakening gates.
The final screenshot renders transcript/controls but its scrolled label area
remains clipped; full responsive UI acceptance is not claimed. Owned emulator
PID 10645/session 88464 exited. Wi-Fi/mobile data were enabled, reverse mappings
were empty and temporary desktop/provider processes were cleaned up.

## Remaining Scope

The terminal self-revoke has no durable Android intent/receipt. Confirmation
loss, cancellation or process death after host commit can leave retained local
credentials while the key is revoked on host. Future authentication denial is
not treated as proof that this request committed; desktop inspection is still
required. Receipt retrieval for a revoked identity, explicit unknown-outcome
recovery UI, local wipe partial-failure recovery, compatibility negotiation and
standalone pairing-only runtime verification remain open. No claim of full
S2-013/037 acceptance or network security suitability is made.

Remote Git reconciliation, real scheduler quota, execution cwd, prompt/approval/
diff/PTY parity, live events/ACK, physical camera/LAN and complete API/release
matrix remain required. All 36 blueprint items stay unchecked; no workers,
commit, push or goal completion is claimed.
