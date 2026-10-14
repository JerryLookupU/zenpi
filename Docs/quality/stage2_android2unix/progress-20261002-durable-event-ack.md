# Durable Transcript ACK Progress

Partial S2-014/S2-020/S2-033/S2-038/S2-053 progress. The intervening RTK request
verified the installed local tool but did not advance the Android goal; current
sources, test processes and existing receipts were re-inspected before resuming.
The full Android/desktop objective remains active. All 36 authoritative rows
remain unfinished. No commit/push, nested agent or controller worker launch.

## Runtime Contract

The host now accepts transcript cursor ACK through the same authorized owner
adapter used by desktop and non-TUI host modes. The generic mailbox drain also
uses mutable owner dispatch, rather than advertising ACK and then rejecting it
through its old read-only dispatcher. Production custom drains still share the
same ACK implementation. This creates no additional Agent/session writer.

An ACK requires current project Read permission, exact project/session identity
and a cursor no later than the durable transcript end. The registry lock spans
authorization and checkpoint CAS. The host stores a device/project/session
high-water mark in the existing bounded private workspace checkpoint. Duplicate
or older cursors return the existing mark without rewriting the checkpoint or
bumping workspace/subtab revisions. Failed persistence does not advance the
in-memory mark. Future cursors, foreign owners, malformed envelopes, busy
owners, capacity and storage uncertainty refuse. No journal data is deleted.

The WorkspaceList boolean event_ack_available advertises checkpoint support.
Android skips ACK against older hosts without that field. Foreground refresh
and WorkManager opt in only for a previously committed encrypted display and
cursor, not the page being received. Explicit and unresolved workspace commands
do not carry ACK. The receipt codec strictly checks device/project/session,
cursor scope, field sets and monotonic unsigned-u64 sequence. A greater returned
mark is allowed for a duplicate from another connection of the same device.
The host records the client's durability claim, not proof of an untrusted
client's filesystem state.

## Tests And Evidence

The real-owner Rust adapter fixture now checks ACK persistence, duplicate/older
byte stability, checkpoint restoration, generic mailbox dispatch, future and
foreign-session refusal, busy and revoked owner refusal, and unchanged
workspace revision. Invalid/duplicate/over-capacity checkpoint records fail
restore; legacy checkpoints without ACK still restore. Exactly 128 valid marks
restore, but a 129th new identity is refused without rewriting bytes. An
external checkpoint modification makes advancement fail CAS; a duplicate still
returns the previously durable mark and leaves the external bytes untouched.

After the late fixture and mailbox edits, 19 Rust remote/workspace/tool/input/
extension/approval suites passed 220 tests with five existing ignored/helper
cases. An earlier ten-suite run passed 71 before the last added fixture cases;
it is not used as evidence for those later cases. One attempted broad command
named nonexistent inputs/tool_context test targets and failed before executing;
the corrected invocation used stage1_input_host/stage1_tool_batch and passed.
Rust locked check/build passed with 20 existing warnings. Format/diff checks
passed. Android Debug and Release result XML each has 48 tests, zero failures,
errors or skips; the latest Gradle invocation was up-to-date, not a fresh rerun.
The original ACK implementation invocation had run those 48 tests freshly.

The headless harness now independently validates the saved ACK against all
durable turn records and the single admission session. Twelve Python tests
include six ACK-verifier positive/negative cases: missing/duplicate/malformed
records, wrong device/project/session, lagging/future or noninteger sequence,
ambiguous/missing session, empty transcript and extra fields. All 31 tools
Stage 2 tests and 15 controller fixture tests passed. The controller fixtures
use temporary repositories/mocked transport; their printed acceptance/startup
messages do not represent canonical worker launches. Strict Blueprint and
controller validation/status passed with no live claims, transports, requests
or handoffs.

The original headless-android-durable-ack-20261002.json remains immutable and
its saved provider/journal/checkpoint evidence passes the strengthened verifier.
The original desktop-android-durable-ack-20261002.json likewise remains a
passed full desktop workflow regression receipt, not independent desktop ACK
checkpoint evidence. Those original runs bind Rust 3fd31df350852b9947b03c4e9c915f789ce54ea23ad5f21145a09a40a4507be7.

The new headless-android-ack-boundaries-20261002.json passed with the mailbox
correction and strengthened verifier. API-36 Android connected directly to
192.168.50.14 with an empty adb reverse table. Four concurrent authenticated
reads, two real HTTP provider prompts, actual Android process death (PID 8661),
encrypted original-envelope recovery, exact retry with no provider replay and
listener expiry passed. The host checkpoint ACK equals all six durable turns.
The saved command evidence hash and both verifiers were independently checked
again after the run. Its screenshot was visually inspected; this is emulator
UI evidence, not a physical camera/phone LAN claim.

New headless run artifacts:

- Rust: fb3d43388d91521883ec1843a91c86c801910a7de737a824e6978c852c3422a3
- App: bb15067f8a54bb1eaa2b3fa40ca80aec2b7732ef64189a3b130e0407246dd7ff
- Test APK: 80c1a750633e9643c88f949a600ca29e2c7478954089c6966d3c3099e4fa8ed9
- Headless harness: dff7e88685cba731c8c63520282fd70f33d2359365d0d631e1982be8acdd0409
- Saved commands: 3a5304a3129b31da2acb7c9e2f35d8692b55f53d52c0b13f01c08cb44ef780cc

The matching desktop-android-ack-boundaries-20261002.json passed with the same
Rust/App/Test APK hashes and the unchanged desktop harness
a1a1d509c4b35e2fc93a551ee3c86308095e897351afb72245e7493f13695ddf.
It independently decoded full-screen terminal QR pixels and completed 24 initial
Android tests, 17 real provider requests, shared desktop updates, directory
selection stress/tool roots, worktree create/execute/dirty refusal/clean removal,
steer/cancel/approval audit, WorkManager, network callback recovery, actual
Android process death (PID 9254) and original pending retry. A second actual
process death after durable revocation (PID 9328) resolved the receipt and wiped
credentials in a new process, followed by four independent cache/revoke process
cases. Listener shutdown and empty adb reverse table passed. This harness does
not independently inspect an ACK checkpoint; ACK persistence is proven by the
headless saved checkpoint and the owner-adapter Rust tests, not inferred from
the desktop workflow pass.

Both fixture hosts/providers and harnesses are terminal. The owned API-36
emulator was stopped through its exact serial, its process/session exited 0,
and adb reports no remaining device. No unrelated process was stopped. Final
strict Blueprint validation and controller status still show 36 unfinished
rows, zero live claims/transports/requests and no pending handoff queue.

## Remaining Scope

ACK retention/cleanup across closed projects and many sessions, slow-client
backpressure/fault policy and live provider/tool delta push remain unfinished.
ACK does not trim the fixed replay window or journal. Independent worktree
session owners and executable concurrency, attachment/resource parity, diff/
PTY/bulk projections, full protocol/fault/adversarial coverage, API 31/34/35
lifecycle evidence, physical scanner/phone LAN, signed release and complete
privacy/security gates remain required. No whole-item or full-goal acceptance.
