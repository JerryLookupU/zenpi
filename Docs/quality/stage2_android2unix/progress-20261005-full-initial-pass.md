# Full Initial Connected Suite Pass

The previous goal turn was progress: full 93-test attempts reached real
desktop terminal, prompt/steering, approval, cache, network restoration,
WorkManager and Doze behavior while retaining two failures. This turn fixes
the remaining visible-click fixture boundaries and runs the complete initial
suite successfully. The broader desktop workflow is still failed and the
original Blueprint goal stays active. No checklist checkbox was changed.

## Two-Axis Selection And Task Submission

The secondary TabRow now has the stable `secondary-tabs` semantics tag.
This is test identification only; rendering, authority and routing are
unchanged. The directory fixture scrolls that row into the outer vertical
viewport before scrolling the actual stable-ID target horizontally. It
requires displayed/enabled targets before actual clicks, including every
alternating stress selection and the final cleanup selection.

The worker fixture likewise scrolls the later task submission icon into view,
requires displayed/enabled state and checks the exact submitted prompt on
the main owner. No command callback was directly invoked and no permission,
pending-write, offline or scope assertion was relaxed.

`desktop-api35-visible-subtab-repaired-20261005.json` retains the first attempt
in this turn: 93 tests, one failure. Both directory prompts and tool/provider
responses were observed, but the final cleanup selection still used the old
horizontal-only scroll. The final two-axis version passes that selection,
directory close/recovery, subsequent workspace mutations and the worker test.

## Attributed 93-Test Pass

`desktop-api35-two-axis-tabs-20261005.json` contains a genuine `OK (93 tests)`
for the entire initial suite, with zero skipped or failed named tests. Its
later process-death follow-up fails, so the source receipt remains
`passed=false` and was not rewritten as a success.

`android-api35-full-initial-suite-20261005.json` is a deliberately scoped
derived receipt for those exact 93 tests. Its verifier requires each named
JUnit start/success pair against the current source inventory, hashes the
source receipt and initial output, checks both installed APK hashes, checks
the production Rust artifact and harness hash, and binds current Android
and Rust build provenance. It explicitly marks full desktop workflow,
post-process-death replay, physical camera/direct LAN and whole-item gates
false. This evidence does not replace the failed source workflow receipt.

App SHA-256:
`780f374c3967704173981550e72c34e67136ddc6c975689b68cebd35244706cb`.
Instrumentation APK SHA-256:
`5fabad50aa2fa69590d4c0b31f999fc3463e29ec1a5acad86c0ac2ed115c3fc6`.
Build receipt SHA-256:
`e413d4bec29396e4150ec29c97362836b815b06c0dcf110789bbf59d144206f5`.

Actual Gradle debug/release unit variants were rerun after the semantics-tag
change. Each collected 610 tests: 590 passed, 20 skipped, zero failures/errors.
The subsequent edit affected only the instrumentation selection fixture.
The Python suite passes 182 tests. Strict Blueprint validation, current
APK provenance verification and `git diff --check` pass.

## Process-Death Follow-Up Is Not Accepted

After the initial suite passes, the harness actually starts a new owned
Android process, saves an unresolved workspace-select request after its real
host commit, proves its live PID and force-stops it. The desktop then appends
another provider answer. A different Android process loads the exact pending
request and cached projection, verifies that foreground/network/cancel
transitions cannot automatically flush it, explicitly retries the same
envelope and reaches CONNECTED with the expected answer in the same session.

The follow-up then fails at
`NetworkProcessTest.newProcessAuthenticatesAndReplaysFromItsEncryptedCursor`:
`replayedEventCount > 0` is false. Revocation/process cleanup follow-ups are
therefore not credited. The source receipt retains all three instrumentation
observations, including the failed follow-up and bounded live-process death.

Inspection identifies a concrete policy mismatch to investigate next, not an
accepted fix: `TranscriptRestorePolicy.replay` refuses incremental replay
when a cached turn has `toolCallsOmitted`. The completed approval workflows
produce such turns, and encrypted display cache intentionally excludes their
argument previews. The follow-up's assertion assumes all caches are replayable
without a refreshed snapshot. Existing production replay/snapshot policy and
this follow-up assertion were left unchanged in this turn. Correct acceptance
must cover both safe argument rehydration and genuine cursor replay rather
than simply removing the replay assertion or enabling unsafe cache authority.

Full Rust gates, the process-death/revocation workflow, current-artifact API
lifecycle/camera matrix, real-phone QR/direct LAN, release installation/
upgrade and per-item Master acceptance remain unfinished. All validator
processes are terminal. The dedicated API 35 GUI emulator is retained for
the operator and the application is reopened after validation.
