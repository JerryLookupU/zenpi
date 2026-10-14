# Scanner And Dialog Evidence

Provisional work for S2-031/S2-036/S2-038. The original Android QR/LAN/Rust
desktop synchronization goal is still active; no authoritative item is accepted.
The previous API matrix turn made verified progress. This continuation improves
the actual scanner's failure path and repairs the dialog focus scope, then audits
the captured evidence rather than promoting a blank screenshot.

## Product Changes

MainActivity now handles JourneyApps 4.3.0 MISSING_CAMERA_PERMISSION and TIMEOUT
result flags. These names and result semantics were verified against the installed
dependency bytecode with javap after official-source browsing failed. The existing
QR-only capture retains permission handling in the library, disables barcode
image persistence explicitly and adds a bounded 120-second capture timeout.
Missing camera hardware or capture Activity returns Camera unavailable.

PairingState separates scanError from connection/write outcome state. Camera
permission denial, timeout and missing camera do not replace host, transcript,
pending envelope or encrypted metadata. Cancellation is a no-op. Scanner launch
waits for metadata loading and rejects connecting/unresolved write/revoke states;
the UI hides Scan QR for unresolved writes and revocation, and returned results
are fenced by those states. Valid scan results use the existing strict URI parser
and enter confirmation only; safety code is never filled or sent automatically.

Diff refresh and IME Done now use LocalFocusManager/keyboard from the dialog's
composition rather than the parent screen. Focus is cleared before hide. A
Compose test verifies the path field is unfocused after refresh.

## Executed Evidence Before Sandbox Change

Gradle :app:test :app:assembleDebug :app:assembleDebugAndroidTest succeeded,
with 51 debug and 51 release unit tests. ScannerActivityTest drove MainActivity,
opened the real scanner, denied the actual system camera permission dialog,
returned to the visible denial state and checked that permission stayed denied.
ScannerResultTest has five cache/result tests; PairingScreenTest has five and
DiffScreenTest has two. The actual 1+12 test run exited 0 in
scanner-api35-resultfenced-20261002.json.

However, visual inspection of that receipt's screenshot found all-black pixels.
Its original passed field describes its original assertions, not a nonblank
screen or full acceptance. The receipt and screenshot remain unchanged and are
explicitly rejected by the strengthened validator. Earlier failed receipts also
remain: scanner-api35-20261002 failed five fixtures due to a missing event cursor;
scanner-api35-cachefixture-20261002 failed one fixture due to comparing deserialized
WorkspaceCommand object identity rather than its original envelope. Test fixture
repairs supply the cursor and compare host/projection/envelope structurally.
An initial instrumentation compile failure for missing Compose delegate imports
was fixed before the passing run. These were not hidden successful reruns.

The separately started headless-api35-scanner-dialog-20261002 run lost its tool
handle when the environment changed, but subsequent file inspection found its
completed, passing receipt. It records two provider requests, actual process
death (PID 4105), original encrypted prompt recovery, concurrent reads, diff,
durable ACK and normal listener expiry. Saved command checksum and independent
provider/journal/ACK validation pass. Both screenshots match their hashes;
visual inspection of its diff PNG confirms an unfocused path, no keyboard and
the full review.txt patch with close/refresh buttons visible.

Both runs bind app APK SHA-256
6641c59b6ea518203e365c790ba79638b5f01ff8be573f109db3c47ca7184503
and test APK SHA-256
ed06fc527f19455fb42fc02e6ba3bd026c60b82000597b2bd175bbb8eb88c665.
These are newer than the previous API31/34/35 matrix APKs; that old matrix
remains historical evidence for its exact artifacts, not this source revision.

## Evidence Repair And Current Validation

The scanner Activity test now waits for idle and retries screen capture within
a bounded two seconds, sampling 32x32 pixels and rejecting black/near-black
captures. At least ten percent of sampled pixels must have a channel above 32.
It emits scanner_permission_denial_nonblank_verified only after saving a
nonblank capture. ScreenEvidence is a heuristic blank guard, not OCR or proof
that every UI element is correctly drawn. Human screenshot review remains needed.

The scanner harness requires exact 1/12 test counts and the nonblank marker,
refuses to overwrite existing evidence, and exposes validate_receipt to check
saved checksums/counts/marker. The previous passing receipt is now refused with
nonblank production permission denial capture was not proved. Four positive/
negative harness tests are included in all 44 passing Stage 2 Python tests.

Two direct Kotlin/JUnit ScreenEvidence tests passed using the existing Kotlin
1.9.24 compiler and JUnit 4.13.2 cached jars, output confined to /private/tmp.
They reject black/near-black/insufficient pixels and accept colored/light samples.
An initial direct compiler invocation lacked the annotations jar; the corrected
classpath compiled and JUnit reported OK (2 tests). This is not an Android build.

After the environment became workspace-write/network-restricted, Gradle's wrapper
failed writing its external cache lock and adb failed installing its smartsocket
listener with Operation not permitted. ps is also prohibited. No permission
bypass, approval escalation or replacement emulator/host was attempted. Thus the
updated nonblank instrumentation is not built or rerun, and the previously
started emulator PID 43616 cannot be independently confirmed stopped here.
The completed headless receipt proves host/listener completion, not emulator
cleanup. Its old tool handle is missing, which is recorded rather than inferred
as a still-live or successfully cleaned emulator.

Strict Blueprint validation, controller validation/status and git diff --check
pass. Controller tests pass 15 fixtures; their mock worker/status messages do
not indicate canonical worker launches. Canonical status has no claims, live
transports, requests or queued handoffs. All 36 Blueprint items remain unfinished.
No Rust source edit, nested agent, commit or push was made in this continuation.

## Remaining Work

Rebuild and run the strengthened scanner fixture with a new receipt path once
normal Android build/device access is available, visually inspect a nonblank
denial screenshot and confirm owned emulator cleanup. Run updated APK lifecycle
and desktop gates separately; do not attach the old matrix hashes to new APKs.
Permission re-enable, scanner rotation/recreation, successful physical camera
scanning and actual phone LAN are still open. Live event push, PTY/bulk projection,
per-worktree execution scheduling, fault/migration/security and signed release
acceptance remain part of the original objective, not waived by these tests.
