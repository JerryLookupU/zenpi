# Full Desktop Suite Repair Evidence

The preceding goal turn made actual progress: six named API 35 encrypted
storage, pairing-cancellation and terminal-UI tests passed on attributed
APKs. This turn moved back to the full desktop integration surface. The
original Blueprint goal remains active; no checklist row is accepted.

## Production Host Fixture And Evidence Gate

The desktop Execution pane can already own a real PTY before Android sends
Open. `HostPairingTest` no longer assumes that production terminal is null.
If it is running, the test closes it via the authorized Android terminal
input UI, requires the same terminal ID to become exited, and requires the
input control to disappear. It then exercises the original explicit Open,
file-producing input, exit, reopen and second file-producing input. Both new
Open commands must produce new terminal IDs; none of the original assertions
or phases was removed.

The host receipt verifier accepts only the original five-operation sequence
or the exact six-operation sequence with one initial `exit\r` input. Each
operation still needs its uniquely bound reserved/succeeded pair, same
device/project/session/subtab/revision, fingerprint, request and terminal ID.
The optional initial terminal must differ from both explicitly opened
terminals. Exit input byte lengths and both exact workspace file effects
remain mandatory. Negative tests cover foreign identity/scope, wrong bytes,
reused terminal IDs, missing receipts and extra initial closes. The focused
Python module passes 19 tests and the Stage 2 suite passes 182 tests.

## Full Runs And Repairs

All four full attempts below ran the same 93 named tests, retained their
original receipts and finished. None is a pass:

- `desktop-api35-initial-terminal-repaired-20261005.json`: nine failures.
  Real terminal Open/input/exit/reopen, prompt/steering/cancellation and both
  approvals completed. Restored-cache equality then exposed the intentional
  omission of live tool-call arguments from encrypted display cache.
- `desktop-api35-cache-scope-repaired-20261005.json`: four failures. The
  production host fixture now passes and retains the shared device namespace.
- `desktop-api35-offline-boundary-repaired-20261005.json`: two failures.
  WorkManager and deep-idle recovery pass; Wi-Fi restore still lacked an
  actual association and the second directory-tab click was offscreen.
- `desktop-api35-network-restoration-repaired-20261005.json`: two failures.
  Real network-loss/recovery now passes. Remaining failures are the second
  directory-tab visibility assertion in `WorkspaceModelTest` and a later
  worker submission callback assertion in `WorkerScreenTest`.

Cache assertions now compare the exact bounded display projection and
explicit tool-call omission, rather than asserting that live tool arguments
survive offline. All four cached subtab capabilities, including concurrency,
must be false. Doze now requires the exact `IDLE` state, preserves the stored
display projection and compares the resumed live transcript against the
captured pre-idle live transcript. No production cache or authority policy
was weakened.

Prompt/approval fixture assertions were separated at actual ownership
boundaries. A duplicate project hides steering and approval targets; a
session change retires an open confirmation instead of retaining a disabled
stale dialog. Tests require those targets absent and no callback, then reopen
an actual current-scope confirmation before continuing. Worker labels are
queried within their dialog to avoid matching both the underlying page and
confirmation. The narrow approval summary test scrolls each inspected field
into view without changing its content or authority assertions.

Installed WorkManager 2.9.1 bytecode was inspected from its local AAR. The
first worker receives the old attempt value, while WorkerWrapper increments
the persisted WorkSpec attempt count when entering RUNNING. Completed
WorkInfo therefore reports one attempt after immediate permanent failure,
not zero. Missing/mismatched identity tests require FAILED, exactly one
attempt, unchanged encrypted bytes and no signing-key creation/replacement.
The actual worker log shows immediate FAILURE, not retry. Production retry
policy remains unchanged. Local disassembly is retained at
`/private/tmp/zenpi-work-runtime-workerwrapper-20261005.txt`.

The network fixture restores its initially associated AndroidWifi hotspot
after toggling Wi-Fi. This association command is enabled only when the
device reports `ro.kernel.qemu=1` and was already associated with AndroidWifi;
the known shell UID refusal alone permits the emulator debug `su 0` variant.
Physical devices get only their original Wi-Fi/data state restored. Recovery
still must arrive through real ConnectivityManager callbacks, exact OFFLINE
cache fencing and real foreground authentication. In the final run the
network-loss/recovery phase and WorkManager-read phase pass.

## Current Attribution And Remaining Work

Final production app SHA-256 remains
`46ceae6cc632eb864fb85be5960d5720f677f94d05bfc4d135856d08c625c553`.
Final instrumentation APK SHA-256 is
`b208b439dec57fc859cf62fa0af0289a3a9a98844be98d0c1cf9018b2048b851`.
Build receipt SHA-256 is
`aad74bbf1a6aafeab10d53aa18e23c03a7e342f5ccaf20af7bf9c1542b8ff515`.
The full attempts bind their exact earlier/final artifacts; none was run
while sources were changing. This turn changes fixtures and evidence
verification only, not production behavior.

The latest full run emits real host terminal, shared prompt/approval, replay,
encrypted-cache, network loss/recovery, storage cancellation, directory
ordering/offline-cache/stress, WorkManager and exact deep-idle phases. Those
observations are useful partial evidence, not whole-workflow acceptance:
the harness correctly leaves `passed=false`, blocks process-death follow-up
gates and does not credit full release or physical LAN support.

The second directory tab remains outside the vertical viewport even after
its nearest horizontal scroll ancestor is scrolled. Its retained pre-click
PNG confirms the row is above the visible area; the displayed assertion now
refuses to pretend that synthetic offscreen input was a user action. The
worker fixture also needs its actual later submission click investigated.
These two failures, downstream process-death/revocation workflows, full Rust
gates, current-artifact API lifecycle coverage, physical QR/direct LAN,
release installation/upgrade and per-item acceptance remain unfinished.

Strict Blueprint validation and `git diff --check` pass. The serialized
Gradle unit invocation succeeds with existing unit tasks up-to-date; only
instrumentation fixtures changed. All dedicated validation processes are
terminal; the API 35 GUI emulator is retained for the operator.
