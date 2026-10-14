# Registry Contention Progress

Partial S2-013/S2-014/S2-019/S2-052/S2-053/S2-054 progress. The prior goal turn
implemented Android command/process recovery and corrected connected refresh.
This turn addresses an observed concurrency boundary in host authorization.
All 36 authoritative rows remain unfinished; full goal remains active. No
commit/push, nested agents or controller worker launch; dirty state preserved.

## Runtime Contract

The private device registry previously used immediate nonblocking exclusive
flock, with most callers mapping any lock I/O failure to Unauthorized. A valid
device could therefore appear revoked during an overlapping registry read.
Locking now retries only WouldBlock for at most 100 ms, then returns AuthError
Busy. All other I/O/schema/private-file/proof/revocation failures remain
fail-closed. Once acquired, authorization reloads the registry and checks the
original connection revision. No cached grant, stale-revision fallback or
unauthorized automatic write retry is added.

Prompt, workspace, subtab, approval, steering and cancellation authorization
propagate Busy before reservation/side effects. Snapshot/replay and capability
listing refuse Busy rather than publish an empty project list during a denied
lock attempt. Authentication and revoke-status preserve Busy, consume their
nonce, and require a fresh connection proof. A self-revoke Busy failure proves
no registry write, while post-publication storage uncertainty remains unknown.

The listener rechecks current authorization before each outgoing frame. If it
cannot do so after dispatch, it does not disclose the frame. A write returns
unknown_outcome with the applicable code, never failed/no-effect, because the
operation may already have committed. A later exact authorized request can
resolve the existing receipt. Read-only frame refusal remains failed. An
additional post-frame check may close the connection without altering already
delivered receipts. Capability-only prompt flags remain conservatively false
if their separate check is unavailable; they never grant on error.

## Behavioral Evidence

New Rust tests hold the actual registry lock across authorization, challenge,
self-revoke and owner-adapter operations. They prove bounded Busy, unchanged
registry/journal/revision, no prompt submission/reservation, consumed challenge,
fresh-proof recovery, and revoked-key refusal. A short-lock fixture changes
revocation under the shared lock and proves the waiting reader reloads current
state. Eight threads execute 400 authorization and 400 current-state reads
without invented Unauthorized or registry writes.

Production host-pair TLS test now holds the lock during authentication and
verifies failed/Busy then successful fresh authentication. A separate real
listener/owner-mailbox TLS test commits WorkspaceSelect, obtains the registry
lock before returning the result, and verifies unknown_outcome/Busy delivery.
After release, a new authenticated connection retries the original envelope
and receives the original succeeded receipt with revision still exactly one.
This uses an explicitly injected mailbox boundary, not a process-kill claim.

Nine focused remote suites passed 64 tests. The later added delivery-boundary
test passed in the four-test remote_tui_pairing suite; an initial test compile
failed due to a missing mutable host binding, corrected before passing.
The broader 19 remote/core/tool/input/extension/approval suites passed 247
tests with five ignored existing/helper cases. The delivery-boundary test was
added after that broad invocation compiled its test binary and is accounted
separately by the later four-test suite, not silently included in 247.

API-36 Android non-TUI direct-interface harness passed:
headless-android-registry-contention-20261002.json. Four authenticated TLS reads
run concurrently with foreground reads and return the same session/transcript.
Two real HTTP provider prompts, actual Android process death, encrypted pending
resolution, exact retry without provider replay and listener expiry passed.
The persisted command evidence independently revalidated. This does not inject
a long-held lock on Android; that boundary is covered by Rust/TLS tests.

The full production desktop QR/worktree direct-interface harness passed:
desktop-android-registry-contention-20261002.json. It independently decodes
terminal QR pixels, runs 24 initial Android tests and 17 real provider requests,
verifies directory/worktree tool scope, dirty refusal and clean removal,
steer/cancel/approval audit, WorkManager, actual process death/pending recovery,
revocation/death recovery and four independent cache/revoke process cases.
No adb reverse. Physical camera/phone LAN and live event push remain false.

Both accepted Android runs bind these immutable artifacts:

- Rust: e00015d3c7a0be0c4686bce3111e4ab98f8ee8ca16399daf1b60bbf64bccb431
- App: dafdf2f1fc981f793c6e1c829f39a9e118403c1961e2f160fb943dc64d08dfaf
- Test APK: 96120db6279f576f753297fb18ac9684ecce76d7a4c3288ae44dc96e0ffa2567
- Headless harness: 28a5ed18e1d25b3fb93c4a0e529453172f794a44f8a3fbe2b2c165c8aa62b189
- Desktop harness: a1a1d509c4b35e2fc93a551ee3c86308095e897351afb72245e7493f13695ddf

Gradle build/instrumentation compilation passed; unchanged debug/release unit
tasks were up-to-date (45 each), not claimed freshly rerun. Python headless six,
desktop fourteen and Blueprint five tests passed. Strict Blueprint/controller
validation/status passed with no active claims/transports/requests/handoffs.
Format/diff checks passed. Fixture hosts/providers stopped. Owned emulator used
no-snapshot-save and shut down with exit 0; adb reported no remaining device.

## Remaining Scope

Long-held lock fairness/resource policy, full fault/adversarial matrix,
independent worktree session owners and executable concurrency, attachment/
resource parity, PTY/diff/bulk, live push/ACK, API 31/34/35 lifecycle validation,
physical scanner/LAN and release/privacy/security gates remain unfinished.
This correction does not prove full synchronization or whole-item acceptance.
