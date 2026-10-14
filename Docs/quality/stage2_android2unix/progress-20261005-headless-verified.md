# Headless Android Runtime Verification

The preceding goal turn made progress through actual APK builds, connected
tests and production repairs. This continuation used the existing dirty
canonical checkout and the dedicated ZenpiStage2Api35 / emulator-5584 only.
The whole Stage 2 goal remains active and incomplete. No Blueprint mark changed.

## TLS Connection Budget

The previous concurrent-read failure is reproduced by a new real TLS test,
four_live_tls_connections_refuse_a_fifth_and_release_capacity. Four established
connections complete Hello/Welcome and remain live; a fifth is refused before
its TLS request, then a released slot admits a replacement. Shutdown releases
all slots. The production cap remains four and no retry or fallback was added.

HeadlessHostTest incorrectly requested four reads in addition to its foreground
subscription. It now requests three concurrent authenticated reads, preserving
the subscription, checking all three projections and reporting the exact read
count. The Python verifier rejects absent, duplicate, summary-only and wrong
count markers. This is a four-total-connection fixture, not evidence of four
reads plus a fifth subscription.

## Model UI And Journal Verification

The first connection-budget run passed concurrency, then failed selecting low.
The fixture attempted to click a background model-entry button while the
previous modal remained open. It now closes the modal before reopening it and
checks that low is an actual host choice and the reasoning selector is enabled.

The model-dialog run completed real Android process death and recovery and made
exactly two provider calls, but failed the independent evidence verifier. The
verifier incorrectly used the in-memory SessionRecord field sequence for raw
JSONL. SessionStore's disk envelope uses seq (src/session.rs encode_record).
The verifier and synthetic fixtures now use seq, retaining exact adjacent
reservation/selection/receipt revision checks and wrong-type/refusal coverage.
Alias-only sequence records are refused, not accepted as interchangeable data.

The new journal-schema run actually passed the full harness, exit zero:
headless-api35-journal-schema-20261005.json and its checksum-bound commands,
instrumentation and PNG artifacts. This proves real pinned TLS, live provider
preview, background read/write fences, foreground reattach, three concurrent
reads, read-only diff, paged host files and provider attachment, paged resource
selection/expansion, model/reasoning selection in actual provider requests,
distinct-process saved-request resolution, durable ACK and exact retry without
provider replay. The host exited at its original 90-second expiry; a subsequent
TCP probe confirmed the listener closed. Exactly two provider requests occurred.

Transport is adb_reverse. Camera, terminal QR decode, direct host interface and
physical-phone LAN fields remain false. No desktop bidirectional, API matrix or
whole-item acceptance is inferred from this headless result.

The model-settings PNG in the journal-schema run was visually inspected and
shows the underlying page instead of the modal. It is not accepted as a model
dialog visual receipt. The fixture now checks visible Host model / Reasoning:
low text and awaits Compose idle before capture. The first visible-settings run
exposed an intermittent disabled selector after the asynchronous refresh and
remains failed. A bounded five-second semantic wait for the enabled selector
was added without removing its enabled assertion or changing production fences.
Its next actual run and visual result are recorded below when terminal.

Both headless-api35-settled-settings-20261005.json and
headless-api35-window-idle-20261005.json subsequently passed all runtime and
independent journal/provider gates, each with two provider requests. The
settled-settings PNG caught a dialog window fade that Compose idle does not
cover. The window-idle run produced a readable, opaque model-settings dialog
with the selected model and low reasoning, but its file-picker PNG still
showed the preceding page. Those files are preserved, not relabeled. All six
fixture captures now share a Compose-idle plus bounded accessibility-idle
helper; no production animation, network policy or command behavior changed.

Final actual run: headless-api35-capture-idle-20261005.json, exit zero. All
headless runtime gates passed again, with exactly two provider requests and
listener expiry/closed-port verification. All six PNGs were visually inspected:
nonblank, readable, correct diff, second file page with fixture note.txt,
second resource page with the chosen template, low reasoning/model selection,
and distinct running/committed transcript states, without transitional window
overlap. Seven retained artifact hashes and independent prompt/ACK evidence
were rechecked against saved files; the exact concurrent-read and distinct
process-recovery markers are present in the saved instrumentation output.

Final APK attribution: app
2267dfddf3d2c2e00c3876b17cb55a5041fc5478cc03f0145e4aa1f6409986a5;
test c92b07d4ba6aa895eebb4ca863c0d5372ce743a465cf1c873a1fac397c0e847b.
Input digest d0b5961a77b2bddd9c94d9ebb6345e56ab443ca9598f9960a74c4de3208d7561;
build receipt fd0d0ea34e3125c18a12e4148bda3009589900f83e729e0d209368a7bf203119.
Production app and host code were not changed in this continuation; repairs
are scoped to connected fixtures, evidence verification and startup fixtures.

Final checks passed: 174 Stage 2 Python tests; strict Blueprint structural
validation (all 36 items still unfinished); APK --verify-only; rustfmt --check
for the two touched Rust tests; git diff --check. The full Rust suite and fresh
Android unit variants were not rerun in this continuation. Earlier unit results
remain historical, not newly executed gates. Reverse mappings were empty;
the test app was force-stopped and only the owned API 35 emulator was shut down.

Failure retention was also repaired: first-phase instrumentation output is now
saved even when the harness exits before process death. Invites are redacted;
old failed receipts are unchanged.

## Production Rust Startup

Capturing bounded private fixture stderr identified the attached-workspace
production test failure: no provider URL configured. The fixture's isolated HOME
had no provider configuration. It now uses the existing bounded loopback
ProviderFixture and explicit non-authenticated local config. Production startup
requirements are unchanged and no real operator configuration is copied.

Actual command:

```text
cargo test --locked --test remote_host_e2e --test remote_listener -- --include-ignored --test-threads=1
```

Exit zero: all five host E2E tests (including all three formerly ignored
production cases) and all five listener tests passed. The debug host build
provenance was refreshed after Cargo completed, before Android validation.

## Remaining Acceptance

The complete QR/camera and direct LAN desktop workflow is not yet accepted.
The previous direct-interface failed receipt still lacks partial instrumentation
output; its error-result retention needs repair. Full Rust gates, API 31/34/35
lifecycle runs, physical LAN/camera, signed release install/upgrade and
checksum-bound per-item Master receipts remain unfinished.

The installed emulator explicitly supports imagefile camera input according to
its local -help-camera-back. That enables a future real scanner-path emulator
test, but such a test is not a physical-camera receipt. LAN routing policy must
remain fail-closed; no default-route bypass was added.
