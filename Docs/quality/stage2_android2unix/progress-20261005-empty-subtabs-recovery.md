# Empty Subtab Recovery And Current QR Workflow

The previous goal turn made actual progress: the local API 35 GUI emulator
booted, accepted the current APKs and ran the production camera-permission
denial test. This continuation preserves the dirty worktree and leaves the
Stage 2 goal active. No Blueprint checkbox was changed.

## Production Repair

`PairingScreen` previously passed an empty secondary-tab list with selected
index zero to Material 3 `SecondaryScrollableTabRow`. The existing terminal
history connected test reproduced `IndexOutOfBoundsException` in TabRow's
measurement. The before-repair output is retained locally at
`/private/tmp/zenpi-stage2-terminal-history-before-20261005.log`.

The screen now omits the secondary-tab section when its list is empty.
Existing codec, owner, capability, session and command fences are unchanged.
The extended subtab test requires the primary workspace to remain available,
secondary tabs and mutation controls to be absent, no concurrency callback,
and valid-scope recovery with the original stable-ID command assertion.

The first focused run passed the repaired terminal history test and the
expanded invalid-scope test, but failed the move-tab callback assertion.
Adding main-thread assertion synchronization alone did not solve that
failure. The command icon was outside the visible scroll area. The test now
scrolls it into view and requires displayed/enabled state before clicking;
it still checks the exact operation, stable ID and arguments. Both failed
five-test runs remain in their original instrumentation files:

- `android-api35-empty-subtabs-20261005.instrumentation.txt`
- `android-api35-empty-subtabs-synchronized-20261005.instrumentation.txt`

## Current Connected Evidence

`android-api35-empty-subtabs-visible-controls-20261005.json` passes all five
named tests in `TerminalHistoryScreenTest` and `SubtabScreenTest`. The receipt
binds the current source/build inventory, exact JUnit start/success pairs,
retained output hash and device-installed APK hashes. This is focused UI
regression acceptance, not full connected-suite acceptance.

Current app SHA-256:
`128965999cab812c6da897dddc585ca6bd1d084530dd58ed83e55276cbc93cda`.
Current instrumentation APK SHA-256:
`d6a2ba401c03a72cf7c8a9355f8b25275054cd4390f8d84d86daf33cc81635c2`.
Build receipt SHA-256:
`ef91e83c008d9dac97ff6ddc235f8f80ae7b8328b0fedd95691eaf50276b2743`.

The same APKs then passed the full emulator-camera workflow on API 34:
`emulator-camera-api34-empty-subtabs-20261005.json`.
Independent retained-evidence verification checks current Android and Rust
build provenance, helper hashes, screenshot/output hashes, exact JUnit test
identity and all 14 ordered phases. The production desktop TUI QR passes
through the emulator imagefile camera and production scanner to pinned TLS.
Explicit Read/Control grants precede the desktop live update, background
cache fencing, foreground replay and Android-originated prompt/answer.
There are three bounded provider requests and six appended durable turns in
the same session; the original journal prefix is byte-preserved. The replay
cursor advances from 4 to 6. Actual deep idle, unchanged cached authority and
foreground reauthentication also pass. The final PNG was visually inspected
and contains readable Android prompt and desktop answer text.

Transport is `emulator_host_alias` (`10.0.2.2`) with no ADB reverse mapping.
This does not prove direct physical LAN or physical camera scanning. Their
receipt flags and whole-item acceptance remain false. Earlier API 31/34/35
matrix receipts bind their earlier artifacts, not this repaired APK.

## Other Gates And Remaining Work

After the production repair, actual Gradle debug and release unit variants
each collected 610 tests: 590 passed, 20 skipped, zero failures/errors.
The Stage 2 Python suite passes 182 tests. Strict Blueprint validation and
`git diff --check` pass; all 36 checklist rows remain unaccepted.

The original full Rust run failed four escaped-pipe deadline tests before
their Python fixture entered. A serialized focused recheck passed all four
with their original deadlines and cleanup assertions unchanged. Another
default full Rust attempt passed those cases but failed
`auth::callback::tests::oversized_uri_and_slow_partial_requests_do_not_consume_the_flow`
at the single-pump connection-count assertion; its focused recheck passes.
The full gate is not accepted and no Rust test was skipped or weakened.
Current logs are retained locally as
`/private/tmp/zenpi-stage2-cargo-test-recheck-20261005.log` and
`/private/tmp/zenpi-stage2-pipe-recheck-20261005.log`.

The previously failed 93-test desktop suite remains failed, not superseded
by these five focused tests or the camera workflow. Remaining acceptance
includes repairing/rerunning that broader suite, the full Rust gates,
current-artifact API lifecycle coverage, physical-phone QR/direct LAN,
release install/upgrade and per-item Master receipts. The API 35 GUI emulator
is retained for the operator; the workflow harness stopped only its owned
API 34 emulator and bounded desktop/provider fixtures.
