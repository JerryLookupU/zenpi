# Device Project Access Change Workflow

Provisional S2-034/S2-037/S2-038/S2-052/S2-053/S2-054 acceptance-preparation
progress at 2026-10-03T21:17:04+0800. Previous turn implemented typed unavailable
project handling. This turn extends the existing production desktop/session
instrumentation workflow to exercise it against real local grant commands.
No nested agents, worker launch, checklist promotion, device launch, commit/push
or cleanup.

## Prepared Connected Gate

SessionHostTest keeps the existing rename/new/resume/local desktop switch and
cancel/reattach phases, then backgrounds the production ViewModel. The harness
replaces that phone's project grant with fixture-only session-access-hidden;
an independent authenticated Android read must observe the original project
missing. Foreground attach must enter Error with the typed unavailable flag,
retain pairing/encrypted display/cursor, drop control/owner authority and leave
no pending write. State must stay unchanged while no refresh is requested.

The local grant then restores the original project. A separate read verifies
that restoration but does not update the held ViewModel. Only the production
Refresh host button may reacquire the original current session and controls,
with unchanged transcript/cursor/workspace revision and no mutation admission.
The session suite still contains eight named tests, now with nine ordered
workflow phases. A 150-second instrumentation budget remains below the existing
240-second desktop invitation bound. This is not a background socket promise.

The harness requires three device records with identical identity/public key,
name, capabilities and non-revoked state; exact project removal/restoration;
and consecutive integer registry revisions. Owner checkpoint and active journal
must be byte-identical during the ACL-only change. Saved command evidence now
retains public registry records and before/after owner hashes. Direct-interface
runs recheck no adb reverse after the workflow. Existing source/APK/host and
harness/verifier attribution gates remain.

New Python negative fixtures refuse altered identity, grants, revisions and
missing records. Instrumentation count and ordered phases are source/fixture
tested. They cannot establish that the actual workflow passed on a device.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-access-device-workflow-final-20261003: exit 0; main/unit/
  instrumented Kotlin compile; 361 JVM tests pass. Instrumentation changes
  compile only. Production Kotlin/Rust implementation is unchanged this turn.
- Session harness verifier: eight Python tests pass; Stage 2 Python discovery:
  94 pass; lifecycle verifier: 17 pass; instrumentation verifier: five pass;
  Blueprint verifier unit tests: five pass. Expected invalid-LAN argparse
  diagnostics are negative fixtures.
- cargo fmt --all -- --check, git diff --check and strict Blueprint validator
  pass. All 36 authoritative checklist rows remain unfinished.
- Debug and release host verify-only pass. Android verify-only refuses missing
  APK provenance. Running the session harness with an unavailable fixture serial
  fails at require_current_build before any adb or binary invocation, as its
  mocked admission test also requires. No receipt or device success published;
  no unchanged blocked Gradle/socket attempt repeated.

```text
main.jar a4ada458b9844220aeb5dc7c7d8379bcae5a17d7e72f778c5db57ab8d858d76f
unit.jar 69bad05f0867d51ed0fdc744a13a93815de348a4b37e2001cc4b16c79cbf055f
instrumented.jar 0517a18ec22b5ef3f4cd0ba2dbefce587517a775244f04693f778bd78ec03622
SessionHostTest.kt 25b4a03b905a234802c78b09de196cf9212ea98a90782c19823b162194caffaa
stage2_session_android_smoke.py ef4ff0e1153178cc6e99d0883be23dfbe8db58620abed579ca1fceeaff767332
test_stage2_session_android_smoke.py cec2ac88b9579cd662be4dc13f1ff6bda4f2e2e1f55dd7e404556510535df247
```

Goal remains active. Current-source APK and execution of this production
desktop/Android workflow, physical camera QR/direct LAN, API 31/34/35 connected
lifecycle, full parity and signed install/upgrade remain required. Terminal QR
image decoding and emulator direct-interface TLS cannot replace physical-phone
camera/LAN evidence.
