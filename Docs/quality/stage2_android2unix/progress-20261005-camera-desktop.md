# Emulator Camera And Desktop Continuation

Previous goal turn: progress. The actual headless harness passed repeatedly,
including its final correctly rendered six-screen capture. The whole goal stays
active; no Blueprint item was accepted or rewritten.

## Desktop Regression Evidence

The desktop harness now retains InstrumentationFailure.result even when the
invocation never returns a CompletedProcess to its caller. Bounded partial
stdout/stderr, exit code and timeout state are redacted and retained alongside
completed earlier invocations. Two regression tests cover missing/duplicate
observations, prior results and credential redaction; desktop tool tests 19 pass.

An actual API 35 desktop/workspace/worktree/Doze run used adb_reverse and reached
all 93 tests, with 20 device failures. These include a stale terminal-null
expectation, lifecycle/storage/pairing callbacks and UI interaction failures.
It also correctly refused attribution because the canonical test APK/source
inventory was changed by the new camera test while this run was active. The
failed desktop-api35-reverse-current-20261005.json preserves 183387 characters
of instrumentation and is not accepted. This was an execution-order mistake,
not a reason to bypass provenance: future APK/source edits must wait until
connected validation terminalizes, just as Gradle invocations are serialized.

The headless passes from the previous turn remain historical attributed
evidence. They are not relabeled as passes of the expanded camera-test APK.

## Real Emulator Camera Path

Added EmulatorQrHostTest and stage2_emulator_qr_smoke.py. The harness reconstructs
pixels from the production desktop PTY QR, independently decodes the original
PGM, preserves its cropped binary QR pixels in an explicitly positioned PNG, and
starts only a dedicated Stage 2 AVD with imagefile camera input. No invitation
URI/token is supplied to the instrumented app. The actual MainActivity Scan QR
button launches PairingCaptureActivity and its production decoder factory;
camera frames must reach the confirmation screen before any key is created.
The emulator-only test automates SAS entry, performs pinned registration,
checks the ungranted projection, waits for an explicit host Read grant and
renders the real provider transcript, then uses production local cleanup.

This mode explicitly records imagefile camera and adb_reverse. Physical camera,
physical phone LAN, direct LAN and whole-item acceptance remain false. Physical
manual-camera acceptance is not changed and still requires the operator.

Three Python tests pass for CRC-valid PNG encoding, exact QR pixel preservation
and quiet zone, invalid/rectangular/nonbinary image refusal, and public argument
shape with emulator-only SAS. Existing pairings/keys are refused, never cleared
as prerequisites; an already running requested emulator is never replaced.

The first actual camera attempt booted, opened the production camera Activity
and verified its decoder, then failed because Compose hierarchy queries cannot
run while the native camera Activity hides the Compose screen. That failed
receipt and output are preserved. The bounded camera wait now observes the
production ViewModel's real scanner completion before querying the returned
Compose confirmation screen; it does not inject a scanner result. The next
actual run is recorded below after terminal observation.

## Camera Framing Findings

The native-wait, settings-landscape, wm-rotation and automation-rotation attempts
all remain failed receipts. Real native screenshots showed the imagefile input
being clipped in portrait; shell rotation was reset by the instrumented test
environment. Emulator-only UiAutomation rotation reached an actual landscape
camera (Camera Display Orientation: 0), but the centered image still clipped at
the top. A timed screenshot of the live camera proved this, rather than treating
the failed decoding as network or permission trouble. A larger 1280x960 input
and controlled pixel-position probes showed that the imagefile camera rotates
the bitmap axes. Moving the unchanged QR to (340,480), with a complete quiet
zone, exposed it in the camera viewport. The PNG encoder tests were updated to
verify those exact pixels/coordinates, CRCs and invalid image refusal.

emulator-camera-api35-rotated-viewport-20261005.json is the first actual camera
decode: production CaptureActivity/decoder -> scanner result -> confirmation
with the exact independently decoded endpoint and fingerprint, while no key or
pairing existed. It then failed on an ambiguous Confirm host selector: heading
and button both match the text. The selector now additionally requires a click
action; no production UI, confirmation requirement or decoder was weakened.
All intermediate failed attempts are preserved and not accepted as a complete
pairing pass. Camera input lives only in fixture-private temporary storage;
receipt output stores its hash, not the one-use QR pixels/token.

The harness now binds its relevant helper/source hashes, checks the QR pin
against the production identity's certificate, refuses existing emulator ports
and terminates only its owned live process. The next actual confirmation run
uses the refreshed attributed test APK and is recorded below when terminal.

## Actual Camera Pairing Pass

emulator-camera-api35-confirmation-20261005.json passed, exit zero. Its saved
instrumentation has one exact named JUnit start/success pair and all five
ordered phases: production camera requested, camera confirmation verified,
ungranted projection verified, authenticated desktop transcript verified,
production local cleanup verified. The camera received production TUI QR
pixels, not a URI injected into the app; the emulator-only SAS was then entered
through the real confirmation control. Pin, device key and explicit Read grant
were checked against the production host. The 640x320 transcript screenshot was
visually inspected: readable/nonblank production UI with the exact provider
answer and no overlapping content. Saved screenshot/instrumentation checksums,
all harness input hashes, JUnit identity and ordered phases were reverified.

Final app hash remains
2267dfddf3d2c2e00c3876b17cb55a5041fc5478cc03f0145e4aa1f6409986a5;
test APK e7687ef1dc7e3bff1184ea943c8a0e32fd9bbb7016dac31e079386c456e6b0dc.
Input digest 9707491528558b9679c632afad9edac2ff66d3c595bae45aa67b97a432c8dbe2;
receipt ba785e378d6aacb358f3d31a80fb737992db320dbd47c629329f6f6eadb02d08.
APK --verify-only passed; 179 Stage 2 Python tests passed; strict structural
validation still reports all 36 Blueprint items unfinished; git diff --check
passed. Android unit variants and full Rust gates were not rerun in this turn.
The harness removed its reverse mapping, force-stopped its app, waited for its
owned emulator process to exit and cleaned up its desktop; adb devices now has
no connected emulator. RateVC AVDs were not touched.

This is a camera-frame-to-TLS simulator pass, not a physical-camera/manual-SAS
or direct LAN pass. The direct LAN and bidirectional desktop/full lifecycle
regressions remain unfinished. The physical test also uses the old native-camera
Compose hierarchy wait; that unexecuted fixture must be repaired before a
physical operator run, without automating its manual SAS confirmation.
