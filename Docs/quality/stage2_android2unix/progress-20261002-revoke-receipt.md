# Signed Revocation Receipt Recovery

## Progress

The preceding goal turn made verified durable-intent progress. This turn
advances S2-013/037/038 by resolving the post-host-commit/pre-local-completion
boundary with a signed read-only receipt query. The host now persists the
self-revoke request ID atomically alongside its device's revoked state.
Legacy records remain readable and local-operator revocations have no matching
self-revoke receipt. No normal authentication or mutation permission is restored.

`revoke_status` is a distinct first-message path after TLS hello/challenge.
It verifies a signature over the host fingerprint, fresh connection nonce,
device ID, original request ID and query ID with a separate purpose tag. The
challenge is one-use even on failed proof. A revoked key may retrieve only
`{device_id, request_id, committed}` for itself; the server closes the connection
without creating an authenticated handle or exposing project/session data.
True requires the exact request ID in the durable revoked record. False is
not proof of remaining authorization and never re-sends a mutation.

Android's explicit Check revocation status button queries using the retained
key and pending UUID. A matching committed response completes saved-host/intent
checked key/cache removal. Missing receipt, invalid proof, cancellation or
transport failure retains local state. No foreground/background query retry
is automatic. New response/message fields do not change existing schema number;
compatibility negotiation with older deployed peers remains a required gate.

## Evidence

Six Rust suites passed 37 tests. The final six-test auth suite reran with added
request/query binding assertions and passed. The new auth test proves persisted
receipt after registry restart, wrong-key denial, consumed challenge refusal,
cross-connection replay denial, original/query ID signature binding, unmatched
receipt false and continued normal authentication refusal for revoked keys.
Rust build passed with 20 existing warnings/no errors and formatting passed.
Android debug/release unit suites each passed 31 tests, including a new strict
receipt-message field/routing test. Debug and test APK assembly passed.

`desktop-android-revoke-receipt-20261002.json` records a passing production
desktop/Android private-interface run without adb reverse: seventeen initial
instrumentation tests, one actual process-death network restoration test and
four independent display/revoke-intent process fixtures. The restoration test
then persists an intent, sends real host revoke and deliberately leaves local
completion undone. A new ViewModel restores it; normal authentication must be
unauthorized and another request's receipt must be false. Clicking the actual
production Check button requires confirmed status and absent signing alias and
metadata. The harness independently verifies host durable revoked state. Receipt
flag `revoke_receipt_recovery_verified=true` proves this boundary recovery, not
an actual lost TLS response or a second OS-process death after revoke commit.

All current artifact hashes match that receipt:

- Rust: `d2d47163cdd462b034426682172faf33b7a6bc1f23b5a0b224f832fbfae50f9f`
- APK: `4dab599e57b1bd9d72262b7bae35d445aa5beae5dff6f8d3e2d703d7b2e5f38c`
- Test APK: `478d8b3728db580e99a3db035292dc828ef91bc0b84b3e60edd2b5b189301850`

Strict blueprint validation, five blueprint tests, four harness tests, Python
compilation and whitespace checks passed. The final screenshot was inspected;
the existing scrolled tab-label clipping remains and full UI acceptance is not
claimed. Owned emulator PID 42042/session 86633 exited; radios were enabled
and reverse mappings empty before shutdown. Temporary desktop/provider hosts
were cleaned up.

## Remaining Full Scope

Actual confirmation-drop proxy testing, OS-process death at that revoke boundary,
negative receipt/cancellation UI matrix, pairing-only runtime query testing,
legacy/peer version negotiation, strict receipt-envelope fingerprint accounting
and local wipe partial-failure recovery remain open. Exact request-ID matching
is appropriate to a single terminal self-revoke per device but is not a general
remote mutation receipt ledger. Cross-process concurrent Android storage writes
are still outside current locking guarantees.

Git crash reconciliation, execution cwd/real scheduler quota, prompt/approval/
diff/PTY and live event/ACK parity, physical camera/LAN, supported API matrix,
release and execution-controller gates remain required. All 36 blueprint items
remain unchecked. No workers, commit, push or full-goal completion is claimed.
