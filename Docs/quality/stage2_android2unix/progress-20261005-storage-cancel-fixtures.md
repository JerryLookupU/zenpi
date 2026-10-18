# Storage Cancellation And Terminal Fixture Repairs

The previous goal turn made concrete progress: catalog pagination and
keyboard/focus behavior were repaired and verified on portrait/wide layouts;
the current-production APK passed camera, network synchronization and Doze
on API 34. This turn repairs real connected-test failures rather than treating
that narrow workflow as full Blueprint acceptance. All checklist rows remain
unfinished and the original goal stays active.

## Reproduced Failure And Diagnosis

The current API 35 storage/cancellation baseline ran all five named tests in
`ForegroundStoreTest` and `PairingCancelTest`; all five failed. Its original
output remains `android-api35-storage-cancel-baseline-20261005.instrumentation.txt`.

Both foreground-store tests failed only at the `finish_workspace` restored
state equality assertion. The encrypted bytes, host and cached projection
were unchanged. `WorkspaceCommand` is a normal class whose restored instance
is reconstructed from its JSON envelope, so whole `SavedHostState` equality
compared two distinct command object identities. The test now compares the
complete state excluding the command instance, then the exact pending command
envelope separately. The byte-identical encrypted-record assertion and every
admission count/cancellation/key-removal assertion are still required.

The three pairing-recovery tests reached their scanner-admission probe, then
expected OFFLINE but observed CONNECTING. They used `beginScan()` as a
readiness poll and immediately called `scanResult(null)` before checking the
reconciled state. That explicit new user action legitimately invokes
`resumeAfterScan()` for a saved host in the foreground. The test was asserting
the recovery invariant after initiating another operation.

The fixture now waits for the held metadata read to complete on the main
owner, using its existing reflective fixture boundary. It checks saved/missing
host, OFFLINE/unknown-outcome state, signing-key preservation and explicit
cancelled-recovery behavior before starting another scan. Scanner admission
and scan completion are still exercised afterward in every branch. Storage
ownership, original waits/deadlines, interruption matrix and production
reconnection behavior were not changed. The repaired five-test run passes
and is retained as `android-api35-storage-cancel-repaired-20261005.instrumentation.txt`.

## Terminal Test Follow-Up

The earlier full desktop-suite failure showed `TerminalControlScreenTest`
calling a synchronizing semantics lookup from inside `runOnIdle`. That
lookup now stays on the instrumentation thread; the exact zero-scroll
assertion is unchanged. A new six-test run then passed the five storage tests
and reached a later terminal assertion. It failed because a duplicate selected
project hides the Open desktop terminal control, rather than rendering a
disabled control. That failed run remains
`android-api35-storage-terminal-current-20261005.instrumentation.txt`.

The terminal fixture now separately requires the duplicate-project control to
be absent and all other blocked-scope controls to be disabled. It also checks
that none of those states adds an open callback. All existing byte-exact
input/key/interrupt, dimensions, draft reset/preservation, horizontal scroll,
malformed terminal, read-only, offline and exit/reopen assertions remain.
No production code was changed in this turn.

## Final Connected Evidence

`android-api35-storage-terminal-final-20261005.json` passes six exact named
JUnit start/success pairs in the three classes. Its verifier checks current
source/build provenance, both installed APK hashes, retained output checksum,
actual emulator API 35 and five storage/reconciliation phases. This is focused
encrypted-storage and terminal-UI evidence, not a live network-terminal or
whole connected-suite pass.

Production app SHA-256 remains
`46ceae6cc632eb864fb85be5960d5720f677f94d05bfc4d135856d08c625c553`.
Final instrumentation APK SHA-256 is
`a520840270ae4bd972cddfa8e53f958e20858d9d424c5a4b9781863ba89bad22`.
Final build receipt SHA-256 is
`79eb012c09a8cd261557d022e8284b61f2ebe25b3281629bb46c76456d52ad41`.
Earlier camera receipts bind their earlier test APKs; the unchanged production
APK does not make them a final-test-artifact whole-suite receipt.

The Stage 2 Python suite passes 182 tests. Strict Blueprint validation and
`git diff --check` pass. A serialized Gradle unit invocation succeeds with its
debug/release unit results up-to-date; this turn changed only instrumentation
fixtures. Full Rust and full desktop gates remain unaccepted.

## Next Acceptance Work

The failed full desktop run's first `HostPairingTest` assumes there is no
terminal before the Android Open command. Inspection of production
`TuiState::render_workspace` shows the visible Execution pane automatically
owns a real PTY via `ensure_shell()`. Its failed initial assertion prevented
the test's shared process credential from being retained, so downstream
shared-host failures cannot independently prove those workflows broken.
This diagnosis is not a pass: the full host fixture still needs an actual
repair and rerun while preserving explicit terminal-open/input/exit/reopen,
prompt, approval, replay and authority tests. Real-phone QR/direct LAN,
current-artifact API lifecycle coverage, release installation/upgrade and
per-item Master acceptance also remain.
