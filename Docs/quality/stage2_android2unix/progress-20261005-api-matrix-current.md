# Current Artifact API Matrix

Previous goal turn: progress. It passed complete API 35 desktop and non-TUI
host-alias workflows and fixed a real missed-network-observation issue. This
continuation preserves the original Blueprint scope and dirty checkout; no
item marks, child agents, commit/push or unrelated process cleanup.

## API 34 Fixture Finding

A dedicated API 34 AVD is launched on emulator-5586 without snapshots, resized
to 320x640 at density 160. The existing API 35 GUI on emulator-5584 remains.
The first full worktree/control/Doze run completes but fails. Pinned production
pairing and initial desktop controls pass; its network-loss recovery fails and
later host-dependent tests see Network unavailable. The failed receipt
`desktop-api34-worktree-hostalias-reconciled-20261005.json` remains unchanged.

Actual emulator inspection shows Wi-Fi enabled but not associated after the
test's restore. The shared Python prerequisite helper reads stderr and correctly
handles this image's known shell-UID association refusal through its debug
emulator su command. The instrumentation fixture instead read only stdout;
the stderr-only SecurityException was silently lost, so it mistakenly treated
failed association as success. No application route/security failure is
inferred from that broken prerequisite.

The fixture now uses UiAutomation.executeShellCommandRwe on API 34+, closes
stdin, and consumes stdout plus stderr. Installed SDK api-versions.xml confirms
that method begins at API 34; API 31 retains the older stdout API. Only the
existing explicit ro.kernel.qemu and originally-associated AndroidWifi gate
allows the exact known UID-2000 denial fallback. Production route binding,
TLS, permissions and network restoration assertions remain unchanged.
The tool web documentation endpoint was unavailable, so API availability was
checked against local SDK metadata and javap rather than guessed.

Fresh APK packaging passes with unchanged application
`871ddd9ffe49c2d55d8c899eb4658f3547b0628785825f0aadb7694d9747065c`
and instrumentation
`18c96fe26add6486d8d45a1d2f19d499493c6351683acc2324544d98198b69a7`.
Debug host remains
`e94ce4b0c77e182932cbe425489c2a6e148f8d6ec8424cf6621a3af67c5ec1e5`.
The next full API 34 run is observed separately. Earlier API 35 receipts keep
their original test-APK identity and cannot certify the new same-artifact matrix.

Remaining: complete same-artifact API 31/34/35 desktop/non-TUI matrix, fresh
camera workflows, physical phone/manual QR/SAS/direct LAN, signed Android
install/upgrade, fat-LTO release repeat and per-item Master acceptance.

The immediate stderr-fixed attempt also fails, but at the initial prerequisite:
the asynchronously requested emulator association has not yet published its
matching network when the first pairing starts. No device registry is created,
so later host-dependent failures are not credited. That failed receipt remains
`desktop-api34-worktree-hostalias-stderr-20261005.json`. After this process is
observed terminal, actual Wi-Fi status and ConnectivityManager LinkProperties
confirm the AndroidWifi association, assigned 10.0.2.16/24 and nondefault
10.0.2.0/24 Wi-Fi route. A fresh complete run uses that observed ready network,
the same APKs and unchanged production route checks. No test or route is skipped.

## Actual API 34 Pass

`desktop-api34-worktree-hostalias-associated-20261005.json` actually passes,
exit zero: all 93 named initial tests, 17 provider calls and complete worktree,
large diff pagination/hash/change refusal, network loss/resume/stopped monitor,
WorkManager/Doze, actual process death, exact pending retry/duplicate idempotency
and revocation/death/credential cleanup. Independent retained verification
checks current artifact/source provenance, four output hashes, helper hashes,
all applicable desktop gates and PNG identity. The screenshot is readable.

`headless-api34-hostalias-stderr-20261005.json` also actually passes, exit zero:
running stream, background read/write fencing and reattach, three concurrent
authenticated reads, model/reasoning selection, actual file/template provider
payloads, two provider calls, saved-request real process-death recovery and
durable ACK. Saved provider/journal/checkpoint/request evidence and all six PNG
hashes are reverified, with the resource page visually inspected. Both use
host alias 10.0.2.2, no ADB reverse and no physical LAN/camera credit.

The API 34 emulator is stopped through only its owned serial, and its original
process handle returns exit zero. API 35 GUI remains. The next dedicated API
31 lane uses the same source and artifacts; it is not inferred successful.

## Actual API 31 Results

The dedicated API 31 desktop lane reaches the end of all 93 instrumentation
tests and all host-side phases, but its receipt is correctly failed because
three Compose assertions are API-level-specific: the stable-ID subtab query is
empty, and the two maximum-tab narrow-screen targets are not displayed. The
failures are in `SubtabScreenTest.kt:28` and `TabCapacityScreenTest.kt:43,57`;
they are not converted into a product pass or hidden by the harness. The
receipt remains `desktop-api31-worktree-hostalias-stderr-20261005.json` with
`passed:false`.

The separate API 31 non-TUI headless lane passes with two provider requests,
stream/background fencing and reattach, concurrent authenticated reads,
resource/file/template payloads, process-death recovery and durable ACK. Its
receipt is `headless-api31-worktree-hostalias-20261005.json`; it uses the
emulator host alias `10.0.2.2`, no ADB reverse and no physical LAN/camera
credit. The owned API 31 emulator was then stopped by serial.

This leaves physical phone/manual QR/SAS/direct-LAN validation, signed Android
install/upgrade, fat-LTO release repeat and per-item Master acceptance open. No
Blueprint checkbox is marked by this progress note.

## Current API 31 QR Camera Retry

The first current-artifact API 31 image-file camera attempt timed out in the
scanner at `EmulatorQrHostTest.kt:58`; it was retained as
`emulator-qr-api31-hostalias-current-20261006.json`. A clean retry with the
same current app, instrumentation APK and debug host then passed the complete
production terminal QR to pinned TLS flow, bidirectional Android/desktop
prompt synchronization, foreground replay and controlled Doze recovery. The
authoritative retry receipt is
`emulator-qr-api31-hostalias-retry-20261006.json`, with one completed
`EmulatorQrHostTest` and no ADB reverse. It advertises the emulator host alias
`10.0.2.2`; it is emulator camera evidence, not physical-camera or physical
phone LAN evidence.

The API 31 desktop matrix was then rerun after making the tab tests explicitly
scroll their vertical and horizontal scroll ancestors before asserting target
visibility. This adds the stable `workspace-tabs` test tag and avoids the
API-31 Compose test-framework nested-scroll ambiguity without changing command
authority or user-visible routing. The fresh receipt
`desktop-api31-worktree-hostalias-scrollfix-retry-20261006.json` passes all 93
instrumentation tests and the complete worktree/network/Doze/process-death/
revocation host workflow with current APKs.

The same current artifact desktop rerun now also passes on API 34 in
`desktop-api34-worktree-hostalias-scrollfix-retry-20261006.json`; its network
readiness is gated on the observed `wlan0` route before listener startup. The
current artifact headless API 34 companion
`headless-api34-worktree-hostalias-scrollfix-20261006.json` passes its stream,
background, concurrent-read, resource/file, process-death and ACK checks.

The current artifact API 35 desktop rerun
`desktop-api35-worktree-hostalias-scrollfix-retry2-20261006.json` and headless
companion `headless-api35-worktree-hostalias-scrollfix-20261006.json` also pass.
The retained matrix receipt `api-matrix-current-20261006.json` independently
verifies identical Rust/app/instrumentation artifacts across API 31, 34 and 35,
using the emulator host alias with no ADB reverse. It keeps physical
phone/LAN and physical-camera claims false.

The fat-LTO Rust release was rebuilt fresh and its production host-pair smoke
passes in `host-release-smoke-20261006.json`. A direct private-interface retry
advertising this Mac's `192.168.50.14` was intentionally retained as a failed
emulator receipt (`desktop-api35-direct-lan-emulator-20261006.json`): the
Android refuses socket admission because the emulator has no matching
non-default Wi-Fi/Ethernet route to `192.168.50.14`. This is a route-policy
refusal before TCP connect, not a measurement proving the Mac interface is
unreachable at the IP layer. It is not credited as direct LAN or physical-peer
evidence; the real-phone step cannot be silently replaced by `10.0.2.2`.

The Android release variant was also rebuilt fresh from the current source
with the repository's deliberate no-keystore policy. Gradle
`:app:assembleRelease` passes and emits the unsigned inspection artifact
`android/app/build/outputs/apk/release/app-release-unsigned.apk` (SHA-256
`9609d9cac4a99d133bd73a26533678e7c0bd1d055628f9816aef46b254b10507`). It is
not installable release evidence; an external operator keystore is still
required for signing and upgrade validation.

The current artifact API 35 emulator image-file camera workflow was rerun after
the API 31 pass. `emulator-qr-api35-hostalias-current-20261006.json` passes the
production terminal QR to pinned TLS flow, bidirectional prompt synchronization,
foreground replay and controlled Doze recovery over `10.0.2.2` with no ADB
reverse. The API 35 emulator was briefly relaunched on `emulator-5584` for an
app-entry check and then stopped; no live emulator is implied by this note. This remains
emulator-camera evidence, not physical-camera or physical-phone LAN evidence.

## 2026-10-07 Restricted Runner Recheck

No Android device is currently visible through ADB. Repeated `adb start-server`
attempts fail before device discovery because the managed runner refuses the
local smartsocket listener with `Operation not permitted`; this is an execution
permission boundary, not an Android app or QR failure. No new connected receipt
is published from this run.

The local test surface was still exercised without external model credentials:
`rtk cargo fmt --check`, `rtk cargo check --locked --offline`, all Python tools
(`285` tests), and the focused Rust protocol/security/parity suites (`55 passed,
1 ignored`). Listener tests that require OS sockets retain their existing
permission-denied failures rather than being counted as network passes. The
macOS RSS benchmark fallback was repaired and its four tests pass; it uses
`getrusage(RUSAGE_CHILDREN)` when the platform `time(1)` sysctl probe is denied.

The latest recheck still sees no ADB device and the same managed-runner
smartsocket refusal. This confirms the connected Android evidence remains the
previously retained API 31/34/35 matrix; no new device result is inferred from
source-only checks. The local no-key path remains usable: controlled provider
fixtures supply responses, while real provider credentials remain optional and
are never embedded in QR or Android artifacts.
