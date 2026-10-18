# Android Lifecycle Evidence

Scope: S2-038/S2-053/S2-055 emulator validation, not whole-item acceptance.
The active objective still requires complete Android/desktop synchronization.
The previous goal turn made verified diff progress. This turn installs separate
API 31/34/35 test images and validates the current product rather than promoting
API-36 receipts as proof of the required API matrix. No nested agent, worker
launch, commit/push or unrelated file/process cleanup.

## Current Verified Matrix

The retained `api-matrix-current-20261006.json` now verifies six successful
desktop/headless receipts across API 31, 34 and 35 with identical artifacts:

- Rust debug host: `e94ce4b0c77e182932cbe425489c2a6e148f8d6ec8424cf6621a3af67c5ec1e5`
- Android app: `363f4ad92d946bff1033b54c238d5be4abd317335ce505aff3542a1598c334de`
- Instrumentation APK: `128362dc54402e9f2342ac8a1eead342838d62c6f8c3f14c6752fa902d3639dc`

Each desktop lane completes the exact 93-test initial suite and subsequent
process-death/revocation checks, including worktree execution, bounded diff
pagination and stopped-monitor fencing. Each headless lane completes stream,
background, read concurrency, resource/file and pending-request recovery
checks. Transport is `emulator_host_alias`, without ADB reverse. The matrix
does not prove physical-phone LAN, camera, signing or whole-item acceptance.

The current API 35 image-file camera receipt
`emulator-qr-api35-hostalias-current-20261006.json` separately verifies production
scanner/pinned TLS, bidirectional synchronization and Doze recovery with these
artifacts. API 31 and API 34 camera receipts below retain their recorded older
APK identities; they must not be described as a current same-artifact camera
matrix. See [the matrix progress record](progress-20261005-api-matrix-current.md).

The following sections are historical evidence, not current-state authority.

## Historical Connected Evidence (2026-10-05)

This section supersedes historical execution-status statements below, but
does not accept the complete API matrix or any Blueprint item.

The newest API 35 full worktree/control/Doze run
`desktop-api35-worktree-hostalias-reconciled-20261005.json` passes all 93 named
initial tests and subsequent real process-death/pending retry/revocation gates.
It verifies 17 provider calls, Git worktree execution and dirty-removal refusal,
large UTF-8 diff pagination/full hash/change refusal and stopped-network-monitor
fencing. Transport is emulator host alias 10.0.2.2, no ADB reverse, no physical
LAN/camera credit. NetworkMonitor now reconciles complete inventory while
foreground-owned and cancels that generation on stop; unchanged snapshots do
not reconnect. Current debug/release unit reports each collect 611 tests,
591 pass, 20 skip, no failure/error. See
[the full network report](progress-20261005-emulator-full-worktree-matrix.md).
Newest app: `871ddd9ffe49c2d55d8c899eb4658f3547b0628785825f0aadb7694d9747065c`;
test APK: `7eec8f0c690d4998b655d7e8896f279815bf1d4c004f50916a2d8aa4c776c412`;
debug host: `e94ce4b0c77e182932cbe425489c2a6e148f8d6ec8424cf6621a3af67c5ec1e5`.
Older receipts below retain their original artifact attribution.

`desktop-api35-process-rehydration-20261005.json` passes the complete current
93-test initial control/Doze suite and its subsequent process-death/revocation
workflow. Tool-bearing encrypted display caches intentionally omit tool
arguments: authenticated snapshot recovery restores their exact turn identity,
role/content and tool metadata before accepting the resumed projection. The
receipt credits metadata rehydration, not incremental cursor replay. It also
verifies the original pending workspace envelope's explicit retry and duplicate
idempotency, real process deaths, revocation receipt resolution and credential
wipe. The production replay policy and authority fences were not relaxed.

`desktop-api35-plain-process-replay-20261005.json` separately passes 39 named
initial tests and the same real process-death/revocation follow-ups with a
complete cache. It credits genuine event-cursor replay, not tool rehydration.
Both desktop runs use ADB reverse and cannot prove camera or physical LAN.
The previously failed `desktop-api35-two-axis-tabs-20261005.json` remains failed.

`emulator-camera-api34-process-rehydration-20261005.json` passes the production
scanner/QR/pinned-TLS bidirectional workflow with controlled Doze. Retained
verification checks the named test, all 14 phases, three provider calls, six
ordered appended turns, preserved seed prefix, same-session cursor 4-to-6
recovery and output/helper/screenshot hashes. It uses the emulator host alias
with no ADB reverse, not a physical phone or the Mac's LAN interface.

All three receipts bind the app APK
`780f374c3967704173981550e72c34e67136ddc6c975689b68cebd35244706cb`
and instrumentation APK
`bbe5bd1e610bbd5135ad4cf8bbb974ce24fd165f37080a6c812df62e66f4ed22`.
Their then-current host artifact is
`c93f6d6385f31c092b3abaf34a6e5f959d041ad9a1e671aacdf69add23da63bd`.
Subsequent source edits require fresh host attribution before new device runs;
these retained receipts do not automatically certify a new source inventory.

The subsequent headless dialog-button fixture repair produces instrumentation
APK `425eff06d7957f2090c51610dd9afdc64c9e4615328fd1883b7b9a329a65ee2a`
with the same application APK. The actual API 35 non-TUI workflow passes in
`headless-api35-release-fat-dialog-20261005.json`, using the attributed fat-LTO
release host. It verifies live stream/background fences/reattach, three parallel
authenticated reads, model/reasoning selection, file attachment and template
provider payloads, process-death exact retry and durable ACK. Saved request/
journal/provider evidence and all six screenshots are reverified. This uses
ADB reverse and does not make the older desktop/camera receipts a current
mixed-artifact matrix. See [the full report](progress-20261005-recovery-release-headless.md).

Current-source desktop/non-TUI API 31/34/35 receipts covering the complete
matrix, worktree/diff workflows, physical-camera/direct-LAN acceptance and
signed install/upgrade remain open. The dedicated API 35 GUI emulator is
retained for the operator; camera harnesses clean up only their owned AVD.

## Matrix Contract

The separate physical-camera/direct-LAN harness now requires Activity recreation
after the first authenticated transcript read. It must create a new Activity
while retaining the ViewModel, saved pin/device identity and same session,
automatically reconnect without scan/Refresh, and never regress the event
cursor. The ordered physical_qr_activity_recreation_resume_verified marker and
activity_recreation_resume_verified boolean are mandatory in current receipts.
This does not replace process-death/API-matrix gates. New assertions compile
only; no current physical-device execution is claimed.

The physical v7 contract additionally requires phone-initiated self-revocation
through the production UI after the prompt roundtrip. The phone observes the
original pending request ID, confirmed no-host/no-key/no-cache state and exact
physical_qr_self_revoke_verified phase. The host harness independently checks
the same device's persisted revoked=true, unchanged ACL and next registry
revision with that request ID. Missing/mismatched evidence and v6 receipts
refuse. This gate is prepared, not executed on a physical device.

The current desktop control+Doze suite is 93 tests: 39 base, 53 workspace
control and one Doze. It additionally requires the exact
revocation_identity_removal_cancellation_fenced_verified phase and boolean.
DeviceRevokeTest exercises cancellation at the three identity-removal checks
and while waiting for the app's identity lock, retaining real signing identity
and exact encrypted revocation intent before matching cleanup succeeds.
Source compilation/validator fixtures do not establish connected execution.
Historical suite counts and receipts below remain historical, not current
acceptance evidence.

As of 2026-10-03 the desktop control+Doze matrix requires the explicit 72-test
suite (20 base, 51 control and one Doze), including file-picker, terminal, diff, ordinary tool/message paging,
resource catalog, model selection and worktree-task/page UI boundary classes. Receipts must record
instrumentation_classes and expected_instrumentation_tests matching that exact
suite. Old 44-test or earlier output is not sufficient even with a valid output
hash. Current Python tests compare suite counts to the source test methods and
reject reduced/mismatched identities. No new connected execution is claimed.

The current desktop matrix additionally requires
android_worktree_diff_pages_verified. Its production ViewModel workflow reads
a tracked UTF-8 patch larger than 64 KiB through more than eight pages, verifies
previous/next identity and full host patch SHA-256/byte count, and refuses a
continuation after a controlled desktop edit before refreshing. Ordered
worktree_diff_pages_requested/worktree_diff_pages_verified/
worktree_diff_changed_verified phases and exactly one matching content digest
and byte-count status are required. The host harness checks unchanged owner
revision, tab checkpoint and session journal during diff reads. These new
connected gates are prepared and source-tested, not device-executed.

The desktop matrix also requires android_desktop_terminal_open_input_verified.
The production Android ViewModel opens the desktop's PTY and sends a command;
the harness verifies ordered reservation/completion records, checkpoint request
fingerprints, identical terminal/session/subtab and the exact host file bytes.
It does not accept the Android marker alone. This new current-source workflow
is compiled and verifier-tested, not yet executed on a connected device.

android_desktop_terminal_exit_reopen_verified is also required. The same
production UI sends exit, observes disabled old-instance control, explicitly
opens a distinct running instance and sends another file-producing command.
Five ordered open/input receipts must share session/subtab but match old/new
instance groups, and both exact host files are verified. Source compilation
does not satisfy this connected gate.

The current verifier also requires a named AndroidJUnitRunner start and success
pair for every expected class/method, with consistent current/numtests indexes.
Missing, foreign, duplicate, skipped, failed or unfinished tests cannot be
replaced by a matching summary count. Saved completed-test identities must
match the parsed output. Desktop/session harnesses bind the shared verifier
source hash so its changes during validation invalidate the run.

Current source additionally requires
android_host_file_picker_provider_attachment_verified for all three headless
receipts. The fixture navigates the production picker and sends its quoted
reference; saved provider payload must contain exactly the known file bytes,
and journal metadata must retain only path/size/digest. A second prompt must
not carry the attachment. files_screenshot is required and hash-checked.
Current source also requires
android_host_resource_pages_template_provider_verified. The production composer
opens the loaded catalog, visits its second page, returns to the first with the
same snapshot, visits the tail again and selects a template into existing draft
arguments. Filler templates alternate missing and empty descriptions; both
pages must show empty descriptions, never their first-body-line fallback.
Selection retains arguments without sending. The saved second provider request must contain the
exact host-expanded body/literal arguments; the journal retains only the raw
command and exact source provenance/hash, not expanded body. The normal pending
request then crosses actual process death and exact retry with still two provider
calls. resources_screenshot is required and independently hash-checked. Older
plain second-prompt evidence cannot satisfy the expanded verifier.
These gates are source/validator-tested, not yet executed with new APKs. Historical
receipts below do not satisfy the expanded current matrix contract.

Each API must have a fresh production desktop and non-TUI host receipt, guarded
by the actual runtime ro.build.version.sdk. All six receipts must bind the same
Rust binary, application APK and instrumentation APK. An explicit validator
rejects missing/false gates, duplicate/wrong APIs, mixed artifacts, reverse
forwarding, wrong provider counts and incomplete/tampered desktop output.
Its nine positive/negative unit tests and all 40 tools Stage 2 tests passed.
The verifier checks all four desktop instrumentation hashes, killed/recovered
process identities and matching recovery markers, plus headless saved command
checksums, original request identity, provider/journal/ACK evidence and saved
PNG checksums. These consistency checks do not replace the actual executions.
This is emulator direct-private-interface TLS, not camera or physical-phone LAN.

Desktop runs cover pinned TLS/one-use registration, actual owner/provider/tool
operations, worktree/diff, foreground cache/replay, default-network loss,
WorkManager read/no unknown-write flush, actual process death and exact pending
retry, revoke/death receipt resolution and independent cache/revoke processes.
A controlled deep-idle test turns off the display, unplugs the simulated
battery, verifies dumpsys deviceidle get deep is IDLE, verifies cached state
unchanged and no pending mutation, exits idle, wakes the display and verifies
fresh foreground authentication to the same host/session/transcript.
It does not promise a socket or WorkManager execution during idle, simulate
OEM battery policies or prove physical long-duration background operation.

Non-TUI runs cover composer/provider dispatch, diff UI/read-only file
preservation, four concurrent authenticated reads, actual Android process death,
encrypted original-envelope recovery, exact retry without provider replay,
durable cursor ACK matching all transcript turns and listener expiry.

## Local Fixture Setup

Installed official Google APIs ARM64 system images through the existing local
SDK manager and already accepted SDK licenses:

| API | Image Revision | AVD |
| --- | --- | --- |
| 31 | 11 | ZenpiStage2Api31 |
| 34 | 14 | ZenpiStage2Api34 |
| 35 | 9 | ZenpiStage2Api35 |

Existing RateVC AVDs are untouched. New AVDs run one at a time, no snapshots,
software rendering, 2560 MiB RAM, 320x640/density 160 for narrow-screen checks.
Screen timeout is disabled on these owned fixtures so unrelated sleep cannot
invalidate a Compose screenshot; deep idle is entered explicitly by the test.
The installed images/AVD definitions remain reusable, but live emulator processes
must be stopped after validation.

## Initial Failure

desktop-api35-lifecycle-20261002.json is a failed receipt, not accepted evidence.
Cold API-35 instrumentation took 75 seconds for the first host test and then
continued the workspace suite. The old 120-second host invite/listener and
initial instrumentation budget expired before WorkManager/Doze validation;
logcat showed ECONNREFUSED and null screenshots after the display's default
timeout. The fixture process was inspected as terminal and force-stopped before
retry, never restarted solely on an observation timeout. New bounded validation
seconds (120..300) preserve a bounded listener but allow 240 seconds for the
initial suite and 300 seconds for the complete host workflow. No product timing,
authorization or lifecycle fallback was relaxed.

The failure exception initially included the expired fixture invite in its
argv. A privacy correction redacted that line while preserving failed outcome,
timestamps and artifact hashes; the exception writer now redacts invites before
receipt publication. This is a recorded privacy correction, not immutable
evidence relabeling or a hidden successful rerun.

## Evidence Status

API 35 passed desktop-api35-lifecycle-bounded-20261002.json: 25 initial tests
including actual deep-idle/foreground resume, 17 provider requests, worktree
diff and network/WorkManager checks, actual process death (PID 5340), pending
retry, revoke/death recovery (PID 5402) and four cache/revoke processes. Its
saved instrumentation hash was independently verified. The separate
headless-api35-lifecycle-20261002.json passed with two provider calls, diff,
four concurrent reads, real process death (PID 5793), encrypted exact retry,
durable ACK and listener expiry. The API-35 emulator exited 0 before starting
API 34 on the same fixture serial. desktop-api34-lifecycle-20261002.json also
passed the same 25-test/17-provider/Doze/network/worktree/process/revoke gates
with identical artifacts. Its cold image was allowed to finish boot after
initial adb offline observations; no duplicate emulator was started. The
non-TUI API-34 run also passed with two provider calls and actual process death
(PID 6222). Both API-34 harnesses exited 0, then its emulator exited 0 before
starting the API-31 fixture.

API 31 passed desktop-api31-lifecycle-20261002.json with the same 25 initial
tests, 17 provider requests, Doze/network/WorkManager/worktree/diff gates,
actual process death (PID 6083), pending retry, revoke/death recovery (PID 6151,
recovered PID 6216) and four independent cache/revoke processes. Its separate
headless-api31-lifecycle-20261002.json passed with two provider calls, process
death (PID 6554), exact encrypted request recovery and durable ACK. Both
harnesses exited 0. API-31 emulator PID 27768 was then stopped through its
specific adb serial, and its original process handle returned exit 0. Images
and AVD definitions remain; no owned fixture emulator is left running.

The aggregate api31-34-35-lifecycle-matrix-20261002.json was generated only
after the verifier read all six successful receipts, recomputed desktop output,
headless command and nine PNG checksums, and rechecked the three saved
provider/journal/ACK chains. All six bind these unchanged artifacts (also
rehashed directly from disk after the last live run):

| Artifact | SHA-256 |
| --- | --- |
| target/debug/zenpi | f59f8cb78b9464bb6409ff1e5350cc90b3ef122fa1cf05795109a9012ca2d1b0 |
| app-debug.apk | d95d962091898f853d15e9887c45a72ca4f9e5f979dba4261a8636f98af4a4a4 |
| app-debug-androidTest.apk | 4a10dac0f2e1a9c35b276b25b641846e17b0c478002c655ea2ab804687e27da0 |

No Rust or application source/artifact changed during this matrix continuation.
The first accidental API-31 invocation combined mutually exclusive extension
and worktree fixture flags; argument validation rejected it before any host or
instrumentation launch. The actual API-31 run used workspace plus worktree
flags, exactly as the API-34/35 receipts; extension coverage is a separate lane.
API-31 software-renderer stderr reported missing ColorBuffers. This is retained
as a fixture diagnostic, not suppressed or called a product failure: the
instrumentation, screenshot extraction, checksum checks and harness exits
passed. Visual inspection still finds the diff dialog keyboard covering part
of the patch after refresh; the document does not claim that ergonomic defect
is repaired.

## Revalidation

From the repository root, recheck existing receipts without launching a host:

```sh
python3 tools/validate_stage2_android_lifecycle.py \
  --desktop \
  Docs/quality/stage2_android2unix/desktop-api31-lifecycle-20261002.json \
  Docs/quality/stage2_android2unix/desktop-api34-lifecycle-20261002.json \
  Docs/quality/stage2_android2unix/desktop-api35-lifecycle-bounded-20261002.json \
  --headless \
  Docs/quality/stage2_android2unix/headless-api31-lifecycle-20261002.json \
  Docs/quality/stage2_android2unix/headless-api34-lifecycle-20261002.json \
  Docs/quality/stage2_android2unix/headless-api35-lifecycle-20261002.json
python3 -m unittest discover -s tools -p 'test*stage2*.py'
```

`--receipt` optionally writes a new aggregate file exclusively; an existing
receipt is never overwritten. The aggregate records physical-phone LAN,
camera scanning and whole-item acceptance as false. It establishes the
bounded emulator matrix only, not every S2-053/S2-055 gate.

## Remaining Acceptance

FGS and notification permissions/services are not implemented: the production
manifest explicitly removes WorkManager's foreground service and declares no
foreground-service or notification permission. This unsupported mode remains
explicit rather than claiming an absent notification/FGS workflow passed.
Release signing, migration/upgrade and complete security/privacy gates remain
unfinished. All 36 authoritative checklist rows stay unfinished.
Real camera scanning and phone LAN, full socket fault/replay-gap cases, live
provider event push, PTY/bulk projection and executable per-worktree scheduling
remain outside these lifecycle receipts and require their own implementation
and acceptance. Strict Blueprint validation and controller validation/status
passed with zero controller live transports, requests or queued handoffs; no
worker was launched for this continuation.
