# Android Client Development

For the current operator flow, see
[Android/Rust LAN user guide](stage2_android2unix_user_guide.md). It distinguishes
source-implemented behavior from outstanding device/release acceptance.
Since ZE-514 pairing grants Read/Control/Approval on the host's mounted project
directly (scanning the QR shown on the owner's screen is the desktop
confirmation); the grant CLIs and the six-digit safety code are removed, and
later sections that mention them are historical. The existing separate
capability checks and offline authority removal are unchanged.
Paired host display now shows the full device ID in four monospace lines with
an explicit copy icon, so the local operator can match it to --list-devices
before granting an ACL. The ID is public correlation, not a signing key or
permission. IPv6 saved-host display brackets the literal address. A 320dp
offline ID/copy Compose case is source-compiled, not device-executed.

The native Kotlin/Compose client lives in `android/`. Minimum Android API is 31;
compile/target API is 35. Current code implements pairing, snapshots and transcript replay,
not the full Stage 2 workspace/session synchronization deliverable.

Resource command insertion now revalidates resource name/kind/command and
description rather than trusting invocable alone. Decode, current catalog
selection and insertion share this metadata predicate; duplicated/oversized
entry lists cannot authorize insertion. A modified resource cannot prepend
shell text or a mismatched slash command to the draft; NUL draft text is also
refused while normal multiline arguments remain valid. Current JVM/Rust-owner
resource interop covers valid/mutated insertion, not actual Android or provider
execution. Prepared Compose assertion remains compiled-only.
Resource catalog selection now also checks canonical u64 generation, snapshot,
32-entry page/offset/continuation consistency and bounded consecutive retained
history with unique resource identities. Invalid catalogs cannot authorize
fresh reload, insertion or paging; exact saved reload retries and fresh catalog
refresh remain available. Valid current Rust generation/paging/reload receipt
interop remains covered; device/LAN acceptance is still separate.

Retained file listings now revalidate file name/path agreement, canonical u64
sizes, directory metadata and the exact 10 MiB attachment boundary before
selection or reference insertion. Paged listings require a valid snapshot,
aligned bounded offsets and complete 64-entry forward pages. Navigation and
attachment admission also validate the at-most-seven preceding pages as one
consecutive same-scope chain with unique paths. Corrupt metadata cannot enable
directory navigation, paging or attachment insertion; a fresh browse remains
available for recovery. Current Rust-owner/Kotlin file interop and ordinary JVM
tests cover valid pages and malformed retained state. Compose assertions are
source-compiled only, not physical Android/LAN acceptance.

Retained terminal projections now revalidate canonical subtab/instance IDs,
u64 revision, bounded control-free screen text and optional 1..512 geometry.
Geometry or exited lifecycle without an instance is refused. Decoder,
current-terminal selection and Compose display share that predicate, so bad
retained text or dimensions cannot preserve history, enable fresh input/resize
or render a misleading terminal viewport. Valid legacy screen-only display,
exited-instance history and geometry/output updates remain supported. An exact
saved pending write is still retryable to resolve its receipt, and opening a
new terminal remains a separate host-authorized operation. Ordinary JVM and
current real-PTY Rust/Kotlin lanes cover admission; Compose cases are compiled
only, not connected device/LAN evidence.

The shared current-subtab resolver now also validates retained revision and
every tab's canonical ID, bounded control-free name, supported kind, main-tab
position and 1..64 concurrency. Decode-display results reuse the same shape
predicate. Invalid inactive metadata cannot authorize tab CRUD, quota changes,
worker submission or scoped file/diff/terminal reads simply because the active
ID still matches. Exact pending-envelope retries remain separate. Current
Rust/Git worktree interop and JVM cross-surface cases cover refusals; valid
read-only tabs and boundary values remain supported. Existing Compose tests
are extended but still source-compiled only, not device acceptance.

Worker full-message responses retain opaque
nonblank turn IDs up to 128 UTF-8 bytes without controls, including Unicode and
punctuation, matching worker history rather than imposing an ASCII-only rule.
Selected live/history message requests reject malformed turn identities before
encoding; task/subtab/role/digest/continuation binding remains unchanged. JVM
tests and mutated current Rust worker results cover this decode contract, not
new production worker execution or connected-device/LAN behavior.

Worker history continuations now share the same retained-page validator as
navigation. Invalid digest/count/offset/turn metadata cannot construct a read
or bypass admission through direct decoding; repeated adjacent turn IDs and
empty nonzero histories are refused. Worker and conversation history pages
also revalidate tool summaries: assistant role, at-most-eight unique calls,
bounded IDs/names/previews and truncated-count agreement. Corrupt summaries
cannot authorize navigation or full tool-argument selection. Valid real tool
loop history and bounded summaries remain supported. JVM and current Rust-tool/
Kotlin interop cover these guards; prepared Compose cases are compiled only.

Live transcript tool-argument admission now reuses the retained tool-summary
predicate too. Initial button/ViewModel admission, ordinary message retention,
request/decode and live/worker page selection reject malformed tool metadata,
even when the selected call ID matches. An invalid summary cannot preserve an
already-open argument page or construct a fresh/continuation read. Normal
read-only message paging, full eight-call truncated summaries and cache display
remain separate from write authority. JVM cases and the actual Rust-tool/Kotlin
lane cover refusals; Compose regressions are compiled only, not device-run.

Retained model status now revalidates canonical u64 selection/budget values,
safe model labels, exact capability keys and unique supported reasoning choices.
Catalog pages require a bounded snapshot/offset, full 16-entry forward pages,
valid lifecycle labels and strict UTF-8 ordering. Current scope, merge,
selection and continuation reuse that validator; navigation and fresh dispatch
also check the complete seven-page ordered history. Malformed state cannot
enable model/reasoning writes or paging, while fresh inspection and exact saved
retry remain available. Legacy status without a catalog is still supported.
Current Rust-owner/Kotlin model interop and JVM cases cover admission; existing
Compose cases are compiled only, not device/LAN acceptance.

Retained approval confirmation now revalidates bounded identifiers, known
effect/origin, digest, complete worker/subtab scope and preview text/withheld
consistency. Current admission also validates the full at-most-16 pending list
and its unique scoped identities. Invalid summaries or previews cannot enable
Allow/Deny construction, confirmation or fresh dispatch simply by matching a
request ID. Distinct worker targets and exact persisted pending retries remain
supported. JVM and actual Rust approval/Kotlin interop cover refusals; Compose
confirmation cases are compiled only, not connected-device evidence.

Session catalog scope now also validates snapshot/paging and retained entry
metadata before continuation or resume admission. Current flags must match
session identity; IDs, unsigned creation time, counts, optional names and search
snippets must be valid. Navigation/fresh resume require a bounded consecutive
retained chain with unique session IDs. Malformed catalogs can refresh/search
but cannot resume; exact saved pending retry remains distinct. Current JVM and
Rust-owner/Kotlin session checks cover these predicates; Compose assertions
are source-compiled, not connected-device execution.

Retained worker task-list pages now validate digest form, bounded offsets,
entry-count/total/has-more consistency, metadata-only scoped unique tasks and
the complete at-most-seven-page preceding chain before navigation or page-only
task read/cancellation selection. Duplicate IDs across retained pages are
refused. An exact saved cancel remains retryable; fresh summary cancellation
still does not depend on valid submission tabs. A failed list can refresh
without continuing malformed history. JVM and current Rust-worker/Kotlin lanes
cover these predicates; prepared Compose execution remains outstanding.
The same retained metadata predicate now checks nonempty bounded task IDs,
canonical subtab IDs, known states/failure codes and runtime-active consistency.
Preview text or truncation flags cannot enter a metadata-only page. Empty
pages are valid only for the zero-task first page. Decoder results must satisfy
the same predicate before publication. Valid failed and active unknown-outcome
metadata remain readable; this validation does not infer backend liveness.

Git diff reads now require the complete current subtab list for initial,
legacy and paged request/decode/current-selection checks. Duplicate inactive
IDs and lists exceeding 32 tabs cannot preserve old patch navigation or
authorize late publication. Healthy read-only diff remains available without
prompt/Control authority. JVM and actual Rust Git/Kotlin pages cover refusals;
extended existing Compose assertions are source-compiled, not device evidence.

Session replacement, history resume and rename now share a current-main-subtab
guard across command construction, fresh dispatch and Compose controls. It
requires a unique current project/session, 1..32 unique tabs, a valid active ID
and exactly one main tab in the first position, selected as active. Duplicate,
foreign, empty, oversized or misclassified tab projections cannot authorize a
new write. A saved pending intent still permits only its exact envelope retry.
Ordinary JVM and current Rust-owner session interop cover these refusals;
extended Compose cases are source-compiled, not connected-device evidence.

Committed transcript messages now preserve an explicit per-message truncation
indicator. Rust redacts the complete message before projecting a 12 KiB UTF-8
prefix; it does not change the host journal. Android retains at most 64 newest
turns within a 96 KiB JSON-encoded display budget so repeated large replay pages
can commit their matching cursor/cache instead of growing past storage limits.
The message flag survives offline cache restore. Legacy bounded host messages
are still readable. If a single newest legacy turn exceeds the encoded display
budget, Android retains its identity and a Unicode-scalar-safe content prefix,
marks that turn truncated and keeps the full-message action reachable instead
of committing an empty display with an advanced cursor. The same behavior
applies to snapshot and replay; valid tool invocation summaries are retained.
A connected truncated-message action now opens a read-only
paged viewer: each explicit Next fetches up to 8192 bytes from the current
durable turn, bound to the prior redacted-content digest and offset. Only the
current page is retained; it never changes cache/cursor/ACK. Host-side original
and redacted messages are limited to 4 MiB for this operation. Background,
network change, reset, selection and new writes close it. Paging UI and client
are source-compiled, not fresh APK/device/LAN verified.
Verified projection replacement also closes a page when its turn disappears,
changes role/content/truncation, or changes project/session. It cancels the
pending page generation so a late response cannot reopen it. Appending an
unrelated message or refreshing status keeps an unchanged page available.
Full-message continuation now validates the prior page before constructing a
network request: its digest, UTF-8 byte offsets, total/has-more/text consistency
and role must be valid. Worker continuations additionally bind the prior page
to the current succeeded task's stable subtab ID; ordinary continuations forbid
worker subtab metadata. Decoding reuses these admission checks, so callers
cannot bypass them by directly decoding a continuation. Existing valid
ordinary/worker/history/tool-argument paging remains covered by JVM tests.

Prompt text can reference a host-workspace file with `@path` or `@"file name"`.
TUI and non-terminal hosts use the same bounded parser/resolver inside the
captured selected subtab. These are files on the desktop, not Android uploads.
Invalid/missing/oversized references fail the task before calling the provider;
an admission receipt is not a successful file read. Steering remains text-only.
The composer attachment action opens a transient host file picker. It lists
only immediate non-hidden children of the selected subtab, supports directory
and parent navigation, and inserts a quoted reference without sending it.
Oversized files are visible but cannot be selected. Lists retain at most 64
entries and show explicit truncation. Background, route/selection changes,
reset and writes cancel the read and clear the picker; files are not cached.
This flow is source/JVM tested and Compose source-compiled, not device accepted.
File read admission now validates the complete current subtab list, not merely
the active ID. Duplicate inactive IDs and lists over 32 tabs cannot authorize
request/decode, navigation or attachment insertion. After the asynchronous
read, publication checks the returned file page against the current complete
selection before reopening the picker. Unrelated model/resource read publication
is unchanged. JVM and actual Rust-file/Kotlin interop cover selection refusal;
the existing Compose case is extended but not connected-device executed.
Picker selection is disabled if a prompt becomes active or live prompt
authority disappears while the dialog is open, so it cannot silently insert a
file into text-only steering. Remote prompts also refuse while the desktop
composer has local staged attachments rather than consuming another user's
prepared references. Current files_list encrypted transport tests cover real
rustls/P256 over memory pipes; they do not prove Android socket/LAN execution.

## Build And Test

The cached-source checker now generates actual framed exchanges from the current
Rust HostAdapter before compiling/running Android JVM tests. RustAndroidWireTest
feeds them through Android's real response reader and replay/subscription
reducers with fragmented reads: initial desktop display, a redacted local update,
bounded subscription, stale-cursor snapshot replacement and malformed/truncated
refusals. A checked-in public fixture supplies the same tests to normal Gradle
unit runs. This verifies cross-language wire behavior, not TLS/Android sockets,
APK packaging, physical camera/LAN or full write parity. Details and fixture
regeneration are in fuzz/remote_protocol/README.md.
Snapshot and replay terminals also require the transcript event cursor not to
exceed the durable journal sequence. Every durable turn has a journal record;
private/non-transcript records consume journal sequence but no event sequence,
so the counters need not be equal. The same snapshot decoder rejects this
contradiction during cache v1..v4 restoration and encoding. A refused page cannot
replace the last committed display/cursor. This validates a cross-counter bound,
not equality, transcript completeness or transport success.
The same checker now runs an explicit Android-to-Rust session-rename lane:
Android builds a framed request from the current owner's real projection, Rust
dispatches it and Android validates its actual receipts. It checks one journal
mutation, exact duplicate receipt, preservation of a newer desktop name,
revision/request-ID conflicts and Control ACL loss. This lane requires compiled
JVM classes and runs separately from ordinary unit-test counts; normal Cargo
marks it ignored and normal Gradle assumption-skips its phase helper. No
device, TLS/LAN, provider or complete write-parity acceptance follows from it.
An additional explicit proof lane runs Android's actual invitation parser,
PairingHandshake and DeviceProofCodec with a software P256 fixture key, then
Rust verifies the emitted hello/pair and authentication/revoke signatures.
It covers exact Unicode proof bytes, one-use invite/challenge, altered signed
fields and revoked identity. No private key is exported. The offline pairing
receipt is synthetic: this lane is not Android Keystore, TLS or camera evidence.
The checker also runs a current Kotlin-to-Rust/Git worktree lane. Kotlin builds
directory/worktree add, rename, reorder, concurrency 1/64, select and close
commands from Rust's actual listing/snapshot/subtab view. Rust executes them
through HostAdapter and Kotlin validates fragmented real receipts plus refreshed
views. Exact retries must not advance revisions twice; stale revisions and
changed payloads under the same request ID must be refused. A dirty worktree
close must preserve the file, tab, selection and both revisions; a clean close
removes the checkout but preserves its Git branch. Directory-tab close leaves
source files intact. This helper runs separately from ordinary JVM counts and
is assumption-skipped by Gradle without the Rust fixture. It is source-level
write interoperability, not APK, device, camera, LAN or full scheduler parity.
An explicit session lane now has Kotlin build new-session and catalog-bound
resume commands from current Rust views. Rust dispatches the actual framed
commands, replaces the live owner's journal/input port, restarts the pool from
its checkpoint and repeats the original request. Kotlin validates actual
fragmented receipts and refreshed projections/catalogs. The lane checks busy
owner refusal before mutation, stale catalog rejection, request/revision
conflicts, exact retries across restart, old-journal preservation and refusal
to reuse an old catalog after session replacement. It is a current-source
owner/session transaction fixture, not provider, APK, camera or LAN acceptance.
Pairing now captures one canonical public key/device ID before opening its
connection. Before LAN binding, after binding and after TCP connect, it checks
the captured key, invitation wall/monotonic expiry and cancellation. Keystore
reads are bracketed by freshness/cancellation checks so a blocked read cannot
admit the next connection stage after expiry or cancellation. A key deleted
during binding is refused before TCP connect and the owned socket is closed;
the prepared LanSocketRoutingTest assertion is compiled, not device-executed.
The actual handshake rechecks that identity at admission, after
welcome/challenge, after signing before token transmission, and after receiving
the registration receipt. Each handshake identity read is bracketed by
cancellation checks, including the final receipt check: cancellation observed
inside the read refuses the next hello/sign/token/publication stage. The JVM
regression exercises all five identity-read boundaries and preserves the exact
cancellation exception; post-send cancellation does not imply host rollback.
A missing/replaced key cannot publish pairing success;
the ViewModel also rechecks the receipt device ID inside its storage lock before
saving paired metadata. Cancellation remains cancellation. These fences never
generate a replacement key, resend the token or infer host-side rollback if the
host already registered the old key. A failed post-send check retains unknown
pairing outcome for operator reconciliation. JVM race tests and prepared real
Keystore deletion/replacement assertions cover the boundaries; device assertions
are compiled only, not connected acceptance.
Authenticated connections now recheck the saved device identity after TLS and
welcome/challenge, before every request, on every validated response and around
subscription publication. Key deletion/replacement after signing therefore
cannot confirm a later write or publish a new page under stale local authority.
AuthenticationProof.current now checks cancellation both before and after each
potentially blocking identity read. Cancellation during that read cannot admit
the next signature, request or publication stage after the read returns; the
original CancellationException is not reclassified as authentication failure.
HostClient also rechecks after TCP connect before TLS wrapping. JVM blocked-read
and pre/post-sign regressions cover the fence; a named LanSocketRoutingTest
checks authenticated key deletion inside bind refuses connect and closes the
socket, but remains compiled-only until device execution is available.
Foreground subscription/final-read/confirmed-write storage boundaries and the
background ResumeWorker also recheck before cache commit. Failures retain the
last committed display and leave an unconfirmed write unresolved. Foreground
storage admission now brackets identity reads with both coroutine and generation
checks; ResumeWorker uses the same post-read cancellation fence before and inside
commitResume. An identity read that returns after cancellation/replacement cannot
admit AtomicFile startWrite or signing-key removal. AuthenticationProofTest
generation regression and extended ResumeStoreTest first/final admission
assertions cover this boundary; store device assertions remain compiled-only.
They never
create a key or infer host rollback. JVM tests cover receipt-time loss and
multi-page cursor retention; a real Keystore/cache assertion is prepared but
requires connected execution. These checks are not atomic with external key
changes, and an already transmitted operation may still have executed on host.

The API lifecycle matrix uses separate Google APIs ARM64 emulator images for
API 31/34/35, rather than treating API 36 as evidence for older versions.
See `Docs/quality/stage2_android2unix/android-lifecycle.md` for current evidence
and limits. The desktop harness supports `--expected-api N --with-doze
--validation-seconds 240`; expected API is checked against the running device
before any pairing. The bounded cold-image test budget is not a product timeout
override. The headless harness also accepts `--expected-api N`.
`tools/validate_stage2_android_lifecycle.py` validates six same-artifact
desktop/headless receipts, complete API coverage and actual saved provider/ACK
evidence. Forced emulator idle proves cache/offline/foreground reauthentication,
not background socket persistence, OEM behavior or WorkManager timing in Doze.

Live authorized projects can expose `diff_available`. The diff icon opens a
bounded read-only patch viewer for the current stable-ID subtab. A relative path
can be refreshed; binary, no-change, truncation, loading and error states are
explicit. The phone executes no Git or shell command. Each result is checked
against project/session/subtab/revision/path, and the view is transient rather
than a cached authority. Selection/revision changes, command admission, network
loss, backgrounding and key/host reset cancel the read and clear its view. Large
diff paging/application and PTY projection remain unfinished.

Use Java 17 and a local Android SDK with platform 35. Set `ANDROID_HOME` or
provide untracked `android/local.properties` with `sdk.dir`. The checked-in
Gradle wrapper pins 8.11.1 and verifies its distribution SHA-256. AGP 8.7.3,
Kotlin 1.9.24 and Compose compiler 1.5.14 are explicitly versioned.

```sh
cd android
./gradlew :app:test :app:assembleDebug :app:assembleDebugAndroidTest
```

Device harnesses now require fresh source-bound APK provenance. From the repo
root, build both debug and device-test APKs with the guarded entry point:

```sh
python3 tools/stage2_android_build.py
python3 tools/stage2_android_build.py --verify-only
```

The builder invokes the pinned Gradle wrapper offline, disables build-cache
reuse and reruns both APK tasks. Only exit-zero completion with unchanged source
inventory publishes `android/app/build/stage2-device-build.json`. It records
source/config/wrapper/corpus and both APK SHA-256 values; local SDK properties,
signing files and generated output are excluded. Failure or a mid-build edit
removes old provenance and never promotes an existing APK. The pair, desktop,
headless and scanner harnesses refuse stale/missing provenance before contacting
adb or installing anything, and recheck it before writing successful evidence.
This is development-source attribution, not a signed attestation, dependency
lock, hermetic reproducibility proof or device acceptance. Cached JVM checks
do not create this record. Existing October 2 APKs are historical artifacts,
not evidence for current source.

Completed Gradle attempts also retain private stdout/stderr logs and the
diagnostic-only `android/app/build/stage2-device-build-attempt.json` index,
including hashes, byte counts, source-inventory digest and exit status. A
timeout records captured partial output and a null exit code. The CLI names
this index on failure; the logs must be reviewed/redacted before sharing.
The diagnostic index always has `passed: false`, is never consumed by device
harnesses and cannot substitute for the successful APK provenance record.

Host/device workflows also require the Rust host binary to match current
sources, not just a binary hash captured at test start. Build from the repo
root with:

```sh
python3 tools/stage2_host_build.py
python3 tools/stage2_host_build.py --verify-only
# For a release-profile host:
python3 tools/stage2_host_build.py --profile release
```

The builder uses offline locked Cargo for the zenpi binary, inspects Cargo's
structured compiler-artifact report and binds the expected executable to
Cargo manifest/lock, src, vendor, optional build/toolchain/Cargo config and
builder source hashes. Source changes, artifact replacement, failed build or
concurrent edits invalidate the receipt. It publishes atomically under target
only after success and records controlled environment names and output hashes.
Pair/desktop/headless/session device harnesses verify this before adb and
again before successful evidence, and retain host_build_provenance beside APK
provenance. Scanner permission tests do not launch a Rust host and need only
APK provenance. This is local attribution, not a signed attestation or
hermetic/reproducible release proof; user Cargo configuration and dependencies
still require release review.

The release artifact inspector now validates packaged backup rule contents as
well as the signer, alignment and merged manifest. It resolves the manifest's
compiled backup/extraction resource IDs, decodes the actual APK XML and checks
complete exclusions for all nine data domains. The current format permits only
unqualified XML resources; unexpected configurations fail closed. The v2
inspection receipt remains single-artifact evidence, not source attribution,
runtime device transfer, install/upgrade or release reproducibility.

The debug APK is `android/app/build/outputs/apk/debug/app-debug.apk`. Release
signing, reproducible release artifacts and upgrade tests are unfinished;
no production signing credential is included.

To exercise an already-running emulator against the real Rust binary:

```sh
python3 tools/stage2_android_pair_smoke.py --serial emulator-5580
python3 tools/stage2_android_pair_smoke.py --serial emulator-5580 --with-host-state
# Direct Wi-Fi/Ethernet LAN path; do not use adb reverse in this mode.
python3 tools/stage2_android_pair_smoke.py --serial emulator-5580 --lan-ip 192.168.50.14 --with-host-state
```

The harness installs the app/test APKs, launches `target/debug/zenpi host pair`
with a temporary identity, and runs Android instrumentation. Without
`--lan-ip` it forwards only the host port using `adb reverse`; with `--lan-ip`
it binds the Rust listener to the supplied local interface and performs no
reverse forwarding. It verifies wrong certificate refusal, valid
one-use registration, duplicate refusal, non-exportable Keystore private key,
encrypted metadata save/load/wipe and Compose confirmation/accepted states.
The selected emulator must exist; the harness waits for boot but does not create
or stop emulators. The temporary host and any reverse mapping are removed on
exit. In direct-LAN mode no reverse mapping is created or removed.
Tokens are not persisted in receipts. `adb reverse` is emulator interoperability
evidence, never proof of LAN reachability or camera scanning.

The optional host-state mode first drives the real production headless binary
against a controlled local HTTP provider (not Echo). It attaches that durable
session to the host, verifies the Android device initially sees no projects,
grants one project through the local CLI, reads the provider transcript,
reconnects with the same snapshot, renders it in Compose, and revokes the device
through the host CLI before verifying reconnect refusal. This is bounded
snapshot interoperability, not live event synchronization or simultaneous TUI
ownership. The receipt keeps those broader claims false.

The non-TUI command runner has a separate Android acceptance harness:

```sh
python3 tools/stage2_headless_android_smoke.py --serial emulator-5580 --lan-ip 192.168.50.14 --receipt Docs/quality/stage2_android2unix/headless-android-local.json
```

Use the host's actual RFC1918 interface in place of the example address, or
omit `--lan-ip` for adb reverse. Build the app and instrumentation APKs first.
The harness pairs a fixture device, grants Control through the local CLI,
submits through the production Compose composer and observes the real HTTP
provider answer. Before sending the first message, Android opens the production
host file picker, navigates to documents, selects fixture note.txt and verifies
only a quoted reference was inserted, with no pending write or cursor change.
The harness independently verifies the exact Responses input_file data URL,
filename, journal path/size/SHA256 reference, no raw/base64 bytes in the journal,
and no attachment carried into the second prompt. A file-picker PNG is retained
and checksum-verified; the lifecycle matrix requires this new gate and rejects
older receipts lacking it. These new checks require a fresh device run.
The first provider response is held after TextDone preview
until Android proves the running text is displayed without a committed
assistant turn, the encrypted cache strips the preview and maps running to
unknown outcome, and foreground/background/foreground reattach restores the
same request and cursor. An ordered gate releases completion only after those
instrumentation phases; its event order and running-preview screenshot are
recorded separately. These new checks are source-compiled, not device-verified
until fresh APKs run this harness; older receipts do not prove them.
The background interval also invokes retained task cancellation, steering,
prompt, workspace, retry, command refresh and revoke callbacks. It also invokes
pure refresh, selected-project refresh and diff reads. They must not
change the ViewModel or encrypted cache bytes. Shared workspace admission
requires foreground/network availability and permits only an exact saved
pending envelope on explicit retry. Fresh Error-phase writes are limited to
known steer/cancel/approval controls; offline cached authority is not live
write permission. Pairing confirmation and revoke/status require foreground.
Foreground ViewModel refresh/diff entry points refuse background/reset states
before changing display or launching work. Background opportunity reads remain
the separate bounded ResumeWorker path, not retained UI callbacks.
API matrix validation requires ordered write-fence and read-fence phases and a
hash-checked running-stream screenshot, not just receipt booleans.

Production ViewModel connections and ResumeWorker now bind each unconnected
socket to a matching explicit Wi-Fi/Ethernet route before connecting. Unicast
nondefault routes, including routes through gateways to another LAN subnet,
are considered even without INTERNET/VALIDATED capability. Longest-prefix
match wins; equally specific different networks are refused. A most-specific
nonforwarding route is refused rather than falling back to a broader LAN or
cellular default. VPN/restricted networks are not explicit LAN candidates.
Without an explicit match binding is refused; an unrestricted Wi-Fi/Ethernet
default alone does not authorize fallback. Loopback adb-reverse fixtures remain
unbound. No process/global route changes.
Desktop automatic QR address selection also checks the underlying IPv4 address
for IPv4-mapped IPv6 route hints: unspecified, loopback, link-local, multicast
and broadcast hints cannot become LAN candidates. The configured address family
remains mandatory; this selector regression is not proof of phone reachability.
NetworkMonitor observes default and unrestricted Wi-Fi/Ethernet link changes;
structured local address/route stamps fence sockets even when cellular remains
default. This is source-verified, not a dual-network physical-phone receipt.
Observation tokens are process-wide monotonic identities, not raw network
handles or per-start counters. Every monitor start forces a fresh observation,
so the same network after foreground/Activity restart cannot collide with a
ViewModel's old token. Duplicate callbacks within that cycle remain suppressed;
offline observations remain null. Device restart/network recovery still needs
fresh APK execution evidence.
A second prompt is committed on the host while its original
envelope remains pending in the encrypted Android cache; the harness proves
the app is alive, force-stops that exact fixture process, then starts a distinct
instrumentation process. The offline UI must retain the unresolved command
without automatic sending. Its explicit retry recovers the original receipt
and replays the missing transcript, with exactly two provider requests and two
durable prompt admissions in the host evidence. Four independent authenticated
TLS reads also run concurrently with the foreground client and must return the
same original session/transcript. Registry contention is distinct Busy, never
evidence of revocation; reads may resume with normal bounded backoff while saved
writes retain their existing explicit-resolution policy. The fixture listener expires
and closes normally. Receipts include immutable artifact/evidence hashes;
failures are retained and are not promoted to passing receipts. The same Rust,
app and instrumentation artifacts now pass separate API-31/34/35 desktop and
non-TUI emulator runs; the checksummed aggregate is
`Docs/quality/stage2_android2unix/api31-34-35-lifecycle-matrix-20261002.json`.
This establishes bounded emulator command/recovery/lifecycle behavior, not
physical camera scanning, actual phone LAN connectivity, every platform
policy, full API support or complete parity.

## Runtime Boundaries

Message drafts now bind host, project/session, active subtab/revision and the
current running prompt request ID. A new-message/steering target change clears
unsent text rather than retargeting it to another request; same-target event or
preview updates keep it. Worktree task drafts likewise reset on subtab revision
changes. This includes quoted file references inserted by the picker, so a
draft from a previous selected directory is not silently reused after a tab
transition. Already persisted unresolved commands retain their exact envelope
and are not rewritten. Prompt construction requires the controlled project's
session to equal the selected projection session before producing an envelope.
Compose source tests cover these transitions; device acceptance remains open.

Replay-gap snapshot replacement requires the snapshot, its envelope and the
terminal response to agree on project/session, event cursor, journal sequence
and phase. A contradictory completion cannot replace the transcript or publish
its accompanying transient workspace controls. An optional legacy event cursor
must be entirely absent or present with its scope and exactly the two cursor
fields; partial/null/extended cursors are refused. Valid branch replacement at
the same event cursor still works when the journal advances. This is framed
client/source validation, not connected network-fault acceptance.

The host-resource dialog now exposes a reload icon when a fresh workspace
listing advertises resources_reload_control. It sends a typed host reload
bound to catalogue generation/digest, current project/session and owner/revision.
The Android device never reads local resource files or runs extensions. The
existing encrypted pending-write mechanism preserves exact-request recovery;
new reloads are disabled offline, while loading or while another request is
unresolved. No capability is restored from cache. Reopening the catalogue after
confirmation reads current host metadata. Rust owner/receipt tests and JVM
request/receipt/fencing tests are available; the Compose test is source-compiled
but remains unexecuted on devices. Resource path/configuration management and
full interactive PTY/settings parity remain unfinished.

Ordinary transcript snapshots and replay now include the same bounded public
tool-call summaries as worker history. The Android transcript shows tool names,
redacted argument previews and separate argument/list truncation indicators.
The shared decoder requires assistant role, unique call IDs, exact summary
fields, at most eight calls and 512 UTF-8 bytes per preview. Display-tail byte
accounting includes the JSON-encoded summaries. Offline cache v4 deliberately
omits these previews; it retains the existing transcript cursor and text.
JVM decoding/cache tests pass. The new narrow-screen Compose test is compiled,
not device-executed; this is not full S2-036 acceptance.

An authorized workspace may expose `desktop_terminal` for the desktop's
already-open shell screen. The Android screen projection is read-only, monospace, connected-
only and fenced by current subtab ID/revision. Its exact scope/flag/text parser
refuses foreign identity, control authority and ANSI/control bytes; text is
bounded to 16384 UTF-8 bytes. A truncation indicator is visible. The view never
creates a shell and is omitted from offline cache/cursor records. A separate
terminal_control capability enables current hosts' input/resize writes.
The Rust TUI scopes the PTY to its project/session/root and sanitizes screen
output. Real domain PTY tests pass; new Compose/APK/TLS device tests remain
unexecuted under current restrictions. Control-capable current hosts now expose
explicit input/special-key and resize controls for the same existing PTY,
instance-bound through the durable write workflow. Full terminal emulation and
complete terminal emulation remains unfinished; current device acceptance is missing.

Current terminal projections optionally include actual kernel PTY size.
Android initializes Rows/Columns from it and resynchronizes those fields on
host geometry changes, rather than always presenting 24/80. Output-only
updates preserve size drafts; geometry changes do not discard the input draft.
Legacy hosts lacking size retain the existing default. Exact bounded size
decoding requires a terminal instance and does not grant Control or enter the
offline cache. Domain PTY/TUI and JVM regressions cover this contract; updated
Compose interaction is compiled, not device-executed acceptance.

Desktop PTY synchronization now bounds its reader queue to 256 KiB plus one
8192-byte pending chunk and processes at most 64 KiB per UI pump. Slow reads
backpressure the shell without dropping/reordering its output; queue waits are
cancelled at shutdown. Incomplete CSI/OSC retention is bounded to 4096 bytes;
oversized controls are ignored through their terminator, not leaked as text.
The retained screen and phone projection remain separately bounded. Real PTY
fixtures exercise output larger than the queue and full-queue shutdown; this
does not prove connected phone performance or full terminal emulation.

Terminal input/resize construction additionally uses the shared scoped terminal
resolver: a terminal preview with foreign project/session cannot supply a write
even when its instance and tab IDs match. Draft input, Enter mode and size fields
are reset on active-tab/revision or workspace-owner changes as well as terminal
instance/session/host changes. Ordinary output-text updates preserve the draft.
This prevents silently carrying unsent terminal input into a replaced scope;
the host's own owner/revision/instance checks remain authoritative.

Current terminal instances also expose Read-only retained-history pages. Android
binds scope/instance/subtab revision, digest and UTF-8 byte offsets, with explicit
previous/next/refresh and at most seven prior pages. Output changes refuse old
continuation rather than mixing snapshots. Pages are transient and cleared by
selection/instance, authority, network/background and pending-write fences.
The host sanitizes up to 4000 retained parser lines before paging; this is not
unbounded raw output storage. Shell hard wraps/cursor rewrites can obscure secret
markers, so terminal history remains a trusted-device, best-effort-redaction view.

Continuation admission now validates the preceding page itself: digest, bounded
offsets/total, UTF-8 text byte length, nonempty progress, hasMore and sanitized
control characters must agree. Back/forward navigation validates all retained
pages and the seven-page bound, not only terminal identity and contiguity.
Malformed chains disable navigation before network reads but leave explicit
Refresh available for a fresh current-terminal document. Unit and prepared
Compose regression checks do not establish new device/LAN behavior.

Terminal history navigation now requires the displayed page to match the current
project/session, terminal instance and active subtab/revision, including the
subtab projection's project. Previous-page chains must be contiguous and keep
the same digest/total/scope; only seven preceding pages are retained. Foreign
or broken history disables navigation, while explicit refresh remains available
only if the current projection still has a valid terminal. A late stale success
closes the history panel. Diff opening closes all prior read panels/jobs through
the shared close path, avoiding stacked session/terminal/task dialogs. These
UI/source checks are not current connected-device or terminal-fidelity evidence.

The live prompt status can include a transient `stream_text` preview from the
host's production provider callback while the original remote-owned prompt is
still running. It is displayed separately as Assistant (in progress), never
inserted into committed turns or used to advance event cursors/ACK. The host
retains at most 4096 UTF-8 bytes per preview and publishes redacted complete
lines/TextDone only. At overflow, remote prompt statuses retain the previous
publishable prefix and carry `stream_truncated: true`; Android displays an
explicit truncation indicator. An oversized first chunk produces an empty
preview plus the indicator, not an unsafe partial line. Preview slots cover
the host's 64 project lanes without evicting another active task. Further
chunks for that provider response are ignored until a new response or task
cleanup; full committed transcript delivery remains separate. Desktop-local
preview still withholds overflow because its wire field has no truncation
metadata. Offline cache encoding drops preview text/truncation and restores
unfinished work as unknown outcome. This now permits observing partial output
before the locked Agent finishes, through existing foreground listing polling.
The desktop's locally submitted discussion-model jobs now expose a separate
nullable `desktop_stream_text` per authorized project. It renders as Desktop
assistant (in progress), carries no command identity or extra control authority,
and is also omitted from cached projections. Shell/resource/arch jobs are not
misrepresented as model previews. This remains foreground polling, not
long-lived server push, reasoning/tool-argument projection or complete
local-desktop/Android stream parity. Updated Compose/APK tests remain pending
under the current Android build/device restrictions.

`PairingInvite` accepts only strict `zenpi://pair` invitations, exact query
fields and literal IPs, bounded token/fingerprint/expiry and no duplicate keys.
The UI displays the pin and requires manual entry of the desktop safety code.
the draft is transient and bound to the exact invitation, not just its host
fingerprint/expiry. A replacement invitation clears it even if the host refreshed
within the same second. The UI and ViewModel share a fresh-invitation confirmation
gate (six ASCII digits, current phase, no pending write/revoke and no reset).
Invalid/expired confirmation does not start registration or create device keys.
It never pre-fills that code from the QR. Deep links also go through the same
parser and confirmation, and do not auto-connect. Invitations are held only in
memory. Cold foreground restore completes metadata admission before starting
saved-host reconnect. An invitation received while that read is pending goes
through confirmation first; an unresolved saved write/revoke still fences it.
Reconnect cannot leave scanner admission permanently waiting on metadata.
The scanner harness now requires all four cold-restore phases and the updated
21-test state/UI suite; these additions are source-compiled, not device-proven
until fresh APK execution. Older scanner receipts do not prove this behavior.
Successful registration stores host address/pin/device ID in
AES-GCM-encrypted, no-backup metadata using a Keystore AES key. Subsequent reads
atomically cache bounded displayed turns and their matching event cursor in
that same encrypted record; the cursor never commits without its display.
On a subsequent authenticated pure read, foreground/WorkManager callers can
confirm the previous persisted cursor through ACK when the host advertises
support. The host checkpoints a device/project/session high-water mark without
changing workspace revisions or trimming the journal. Just-received pages are
never acknowledged before local cache commit; unresolved commands suppress ACK.
Duplicate ACK and connection loss do not authorize command replay. This is
durable transcript confirmation, not live provider-delta or server-push ACK.
No invitation token is persisted. The UI restores saved host metadata and an
offline transcript after process death and
starts authenticated foreground synchronization without rescanning. While the
Activity is started, committed transcript events are pulled about every two
seconds, with floored-jitter exponential retry ceilings of 1/2/4/8/16/32/60 seconds
after errors (uniformly sampled from 250 ms through the ceiling) and immediate stop on explicit
unauthorized. A read-only refresh of an already connected projection retains
Connected while its owned fetch runs, so confirmed writes do not flicker all
controls off during the following read. First attach and unconfirmed commands
still use Connecting; failures enter Error and cached offline authority stays
disabled. onStop cancels the owned network operation and marks the retained
projection offline; onStart resumes. Each fetch is bounded, so no persistent
background socket or FGS is claimed. Cache corruption fails closed; old
metadata-only records still attach with a fresh snapshot. While the Activity is
started, a default-network callback fences and cancels work on route loss or
replacement, preserves offline display/cursor, and reattaches reads when a
route returns. It checks route presence, not Internet validation: an offline
LAN can host Zenpi. Late loss for an older route does not invalidate its
replacement. onStop unregisters the callback and fences queued delivery.
Unresolved writes are retained and never automatically replayed on recovery.
Connected projections expose a transient synchronization status: Catching up
when a validated replay page still has a tail, and Snapshot replaced when a
validated snapshot_required replacement has completed. Ordinary replay clears
the replacement observation. Offline/error/pending/reset/scanner states do not
claim live recovery, and neither label/flag is serialized into display cache.
These observations are not grants, journal effects or evidence of whole-run
network acceptance. The prepared Compose state regression verifies the labels
and offline removal; the scanner harness requires its unique marker.
Foreground attach and transient readers now load only an existing signing
identity and verify its device ID against the captured paired host before
opening a socket. Missing/unreadable/deleted or mismatched keys are typed
authentication failures: the read loop stops and old live authority/transient
views are cleared, while durable display and unresolved request identities stay.
They do not silently create another key, retry it as a transport outage or clear
saved writes. Explicit QR confirmation is the only identity-creation entry.
HostClient also captures the verified device ID once per connection, then
checks the current key immediately before and after challenge/revoke-status
signing. Key deletion/replacement during TLS or signing and local signing
failure become typed authentication failures before the proof is written.
Receipt validation, self-revoke ownership and ACK use the captured ID rather
than rereading a potentially changed Keystore alias. Cancellation remains
cancellation; these checks never retry a signature or create a new key.
The prepared DeviceIdentityAdmissionTest uses task-local Keystore aliases to
check deleted and mismatched signing keys through the foreground ViewModel,
with unchanged encrypted cache and no replacement key. The scanner state suite
requires its unique identity-admission marker as well as its named completion;
it is still compiled-only until current APK/device execution is available.
A verified desktop session switch now clears the old projection/cursor inside
the existing foreground read generation rather than starting a new refresh
task. Its fresh snapshot waits through the same failure backoff; transport
failures after the switch continue that budget until a projection is persisted
and published. Background, route replacement and a newer generation cancel the
wait. This recovery does not resend mutations or revive old controls.
onStop schedules one unique WorkManager read opportunity after 30 seconds;
onStart cancels it before foreground attach. It performs a bounded read
(20-second outer deadline, existing 15-second transport deadline), with at most
three attempts and exponential backoff. It does not convert a parent/WorkManager
timeout cancellation into another scheduled
retry: worker-owned operation expiry and transport timeouts remain retryable
within the existing bounded budget, unlike parent cancellation.
After a saved pairing is admitted, a missing, unreadable or mismatched signing
identity is a terminal authentication failure for that opportunity, before any
socket is opened. It is not reported as successful skipped recovery and never
generates a replacement key. Prepared WorkManager assertions check FAILED with
zero retries and unchanged encrypted display for missing/mismatched keys; these
device assertions are compiled-only until current APK execution is available.
Observed cancellation before cache commit stops
that commit; the synchronous commit is still guarded by the saved-record CAS.
The worker neither keeps a socket alive nor
starts an FGS. Its socket binder requires the matching Wi-Fi/Ethernet LAN route
rather than Internet-validated `CONNECTED`, since an offline LAN can be reachable.
LAN binding also compares the matching LinkProperties source-address set,
interface and MTU before and after binding. An unchanged destination route
cannot hide DHCP source replacement or link reconfiguration before TCP connect;
address enumeration order and duplicates do not cause a false route change.
Each source address now includes its prefix, flags and scope in the shared
LanAddressState observation. A flag/scope transition with an unchanged IP and
route invalidates both the pre-connect binding check and the NetworkMonitor
inventory revision, while repeated identical observations remain stable.
This guards both pairing and authenticated resume because they share the
binder; actual Android/LAN link-change recovery still requires device evidence.
Missing metadata or an unresolved write skips network work; it never creates a
replacement signing key or sends a saved write. Explicit unauthorized stops
retries. Cache commit compares the exact encrypted-record digest captured
before the read, so foreground updates, re-pair, wipe or a new pending write
invalidate a stale background commit. Same-project/session cache writes now
explicitly refuse regressed journal or transcript cursors instead of silently
skipping the write and allowing the caller to publish the rejected projection.
The policy runs before foreground save, request completion with a display and
background resume commit. A foreground stale-read failure reloads the current
durable display before another read; it does not ACK or publish the rejected
cursor. Display replacement may shorten visible turns while retaining a
nondecreasing transport cursor. The full scheduling,
Doze, cancellation and API lifecycle matrix remains unfinished.

A validated authenticated workspace listing that no longer includes the
requested project now raises SelectedProjectUnavailableException, distinct from
an invalid/malformed listing or generic transport failure. Foreground authority
handling strips old control/transient state and stops that read loop; background
resume treats it as terminal and does not retry or commit a replacement. Offline
messages, cursor, paired identity and unresolved write/revoke are retained.
This does not claim the device was revoked: the project may have closed or the
ACL may have narrowed. The next foreground/manual refresh requests a fresh
authorized selection rather than repeatedly requesting the unavailable project.
Successful projection publication clears the in-memory unavailable-selection
flag. This flag is not stored as a durable grant or added to the wire protocol.

A background read can now recover one verified same-project session-switch
race. HostClient's opt-in read recovery uses the existing authorized listing
probe for ACK/resume/inspect/subtab-list owner mismatches, without subscribing
or retrying any write. ResumeReadPolicy drops the old projection/cursor and
performs at most one new authenticated snapshot read. A foreign/unconfirmed
scope change, actual authority refusal, cancellation or second race escapes
without another immediate retry or cache publication. The original encrypted
record stamp still guards the final commit, so foreground changes or newly
pending writes invalidate this background result. The same outer deadline
covers both reads; no background socket persistence is introduced.

Workspace listings are no longer published early as a projects-only update:
the selected project/session, transcript/cursor, owner/revision and secondary
tabs are validated as one projection before publication. A failed later read
cannot combine new capabilities with the previous owner or cursor. A fresh
listing without workspace owner/revision drops both old authority fields;
partial metadata is refused. Initial reads and subscription pages also reject
workspace revision rollback within the same host owner, even across project or
session changes and when transcript or secondary-tab revisions are newer.
Only a different workspace owner starts a new workspace revision scope; journal,
event and model-selection revisions remain project/session scoped. Foreground
subscription and final-read publication recheck the current live projection
before saving or displaying a reply. Event monotonicity is checked independently
of an optional journal sequence. Offline caches still contain no workspace
authority, so they are not a persistent workspace-revision watermark.

Once a projection has a transcript event cursor, a same-project/session
replacement cannot omit that cursor, even when its journal sequence advances.
Foreground publication, encrypted cache writes, request completion with a
display and background resume all refuse the replacement before committing it.
Legacy snapshots without an event cursor remain valid for an initial attach or
a different project/session; they cannot erase a known resume position within
the current scope. JVM tests mutate a current Rust adapter snapshot; the
encrypted-store regression is compiled but still requires connected execution.

Before a new workspace request becomes pending, dispatch rechecks its current
owner/revision, target project/session and operation capability even while
Connected. Non-workspace operations must target the selected session. Listed
workspace select/close targets may differ from the displayed project but must
still have their current session and permission. Subtab operations check stable
target/revision and individual capabilities; worker/terminal writes also bind
the active tab, and terminal writes bind the current terminal instance.
Steer/cancel/approval recheck their live target in Connected and Error alike.
Session rename additionally checks the captured journal revision. A captured
stale operation is refused locally before saving or sending. This is an early
client fence, not a replacement for host-side ACL/owner/CAS validation. An
already unresolved saved command may only be retried as its identical envelope,
without retargeting it to the current display or regenerating its request ID.

PromptControlPolicy now shares current scope and scanner/reset/pending guards
between message/worker composers, cancellation/steering controls and ViewModel
preflight. PromptStatusCodec.active requires exactly one target request ID in
the unique selected project, with matching project/session and accepted/running
state. Construction and fresh dispatch cannot borrow a foreign or duplicate
status. Worker summary cancellation uses the same lifecycle scope admission;
worker task construction requires one real active tab. Broken active-tab data
does not crash the Compose task composer. Saved exact retry remains separate.
JVM and prepared Compose regressions cover this boundary; they do not prove
current physical QR/LAN cancellation or connected lifecycle behavior.

Retained active prompt metadata is also checked before steering/cancellation:
bounded request identity, known state/failure code, failure only on failed
state and bounded NUL-free live preview with consistent truncation. Decode and
active-target selection share this predicate. Invalid live metadata cannot
authorize construction or fresh dispatch; valid accepted/running previews,
including exact 4096-byte and empty truncated preview, remain usable. Exact
pending envelopes remain reconcilable without granting a new target or ID.
Current Rust running-owner/Kotlin preflight mutations cover the negative cases;
they are not connected APK or direct-LAN evidence.

The source checker explicitly runs AndroidRustPromptTest through an ignored
remote_headless_commands lane. Kotlin generates a message from a real owner
snapshot, then steering and cancellation from its running preview. Rust dispatches all
through HeadlessHost to the same Agent, refuses Read-only/other-device/foreign
session controls. Steering is enqueued once, then applied at the original
Agent's provider boundary; its text appears exactly once in the second
controlled backend context. Exactly two backend calls occur, not an extra job.
Exact prompt/steering/cancellation receipts are compared after stop and host restart, with no
journal rewrite during receipt lookup. A contended unknown_outcome response
is not a receipt: current Kotlin rejects its null result and retains uncertainty.
This is an in-process controlled-backend fixture, not network/TLS, physical
Keystore, live provider or tool-effect acceptance.

The recovery lane now constructs a distinct new Agent from disk journal and
checkpoint rather than reusing the earlier Agent. Its backend refuses any
execution; restored receipt lookup makes zero backend calls and recovers the
same Applied steering record. Revoking the device then rejects all three exact
old requests, even though their receipts exist, with no journal mutation. This
proves fresh in-process owner recovery, not an OS crash or physical reconnect.

The controlled backend's first completion requests two production write_file
tool calls. The original Agent uses Always approval. A separate Approval-only
device's Kotlin client allows one and denies the other from real pending diff
previews. Control alone cannot approve. The allowed fixture file changes and
has one success tool result; denied file stays absent and has one error result.
Kotlin validates path, byte count and resulting diff. Persistent audit binds
device/request/project/session/digest, remember=false and host_answer source;
only the allowed call has an approval_consumed event. Exact approval receipts
also survive fresh-Agent recovery and fail after approver revocation.
Receipt lookup, fresh-Agent recovery and post-revoke retries preserve that file
and journal without backend execution. This proves a real host filesystem tool
effect under explicit all-builtins/Always approval, not an Android-local write,
live provider or physical-LAN acceptance.

The lane also cancels the original prompt while a third real write_file waits
for approval. Kotlin's captured but unsent Allow then fails after cancellation,
pending approvals disappear and the third file never exists. Fresh recovery
rejects the old-owner request; even a request rebound to current owner/revision
cannot approve the cancelled target. No approval_consumed record exists for
that call. This checks cancellation before approval, not every simultaneous
Allow/cancel ordering or physical reconnect race.

A separate remote_approval_wakeup regression holds the waiter's cancellation
predicate while a digest-fenced Allow is accepted, then sets cancellation
before releasing the waiter. It returns Cancelled, clears the accepted decision
and pending projection, refuses late persistence/re-answer, and notifies the
bound session. This is a deterministic coordinator interleaving, not proof of
filesystem effects or all concurrent remote-command orderings.

Primary workspace commands resolve the addressed project ID exactly once
before checking workspaceControl. A duplicate ID with another session or no
capability cannot be ignored to construct a new command. Selecting a different
unique authorized project remains valid and uses that project's session, not
the currently displayed session. Primary tabs refuse duplicate targets and
pause during scanner ownership or local reset; open/close use the same live
mutation guard as their confirmation dialogs. Subtab select/rename/move/close
and concurrency construction require a unique addressed tab. A non-null move
destination must also exist exactly once, both at construction and dispatch.
These checks do not rewrite an unresolved saved command: its exact envelope
still passes the dedicated retry path under the existing lifecycle fences.

SubtabProjectionCodec.current now shares the selected project/session, tab
capacity, unique IDs and existing active-tab check across secondary command
construction, fresh dispatch and Compose controls. A stale/missing/duplicate
active scope leaves display tabs read-only and hides indexed editing/concurrency
controls instead of calling single or indexing a missing target. The concurrency
stepper uses the same scanner/reset/pending-write guard as workspace mutations.
Invalid-scope rejection does not turn an exact saved retry into a new command.

The worktree task composer and workerPrompt construction now use that same
complete active-tab scope rather than validating only the active ID. Duplicate
inactive tabs, foreign tab projects and missing active targets suppress fresh
task submission. Task cancellation still targets its explicit running request,
not necessarily the currently selected tab; a valid switch does not remove
that task's cancellation control. Current headless subtab projections exercise
composer construction and malformed-scope refusal without dispatching an extra
task. This is not physical LAN or current APK execution evidence.

Worker cancellation builds its own request-targeted envelope instead of
constructing a dummy worker prompt first. Current project/session, worker
authority, owner/revision and an explicit cancellable task remain required;
missing or invalid submission tabs do not prevent stopping an already known
runtime. Summary lookup resolves the task ID before checking cancellability;
page lookup does the same before scope/state filtering. Mixed duplicate IDs
therefore refuse rather than ignoring a terminal or foreign duplicate. If a
summary contains that ID, target construction must match its unique current
record; a scoped page may still supply an ID outside the bounded summary.

An explicit Kotlin/Rust worker-cancel lane now constructs that request from
actual WorktreeHost live statuses with no submission tab projection. The host
uses two controlled running lanes on distinct tabs: foreign-device and stale
revision refuse, the Kotlin frame cancels only its target, and the sibling
remains running. Exact receipts are equal before completion and after a fresh
Agent/pool loads the disk journal/checkpoint; recovery cannot invoke another
cancel or prepare worker owners. Kotlin validates correlated refusal/accepted
frames and the terminal/running pair. The fixture advertises worker_control
explicitly because low-level owned listing does not advertise controller
capabilities. This exercises production cancellation/runner/receipt code, not
an external provider, Android Keystore, production listener or physical LAN.

The worker-cancel lane now replaces its custom runner closure with
spawn_observed_prompts and real independent Agent instances. Controlled
backends produce newline-complete live previews and hold at the cancellation
boundary. The target journal has one real user turn and no assistant answer;
the sibling continues and produces one final assistant answer after release.
The master journal receives no task turns. This strengthens actual Agent and
session execution evidence while retaining the controlled-provider limitation.

The same lane now uses current Kotlin workerPrompt and FrameCodec to generate
both initial task submissions from production SubtabList snapshots, with an
explicit fixture-selected displayed tab and stable fixture request IDs. Rust
decodes those frames and admits/retries each exact envelope once through
WorktreeHost. Fresh disk Agent/pool returns their original accepted receipts
without preparing workers or invoking backend again. This closes the generated
submit -> actual Agent preview -> targeted cancel/sibling completion -> receipt
recovery chain, but does not prove connected Android UI tab selection or LAN.

That recovered worker lane also sends modified submit/cancel payloads with
their original request IDs and checks request_id_conflict, then revokes its
device and checks unauthorized on all three previously accepted exact requests.
Current Kotlin parses these correlated refusals. No new worker/backend start,
second cancellation or journal change is allowed; durable receipt lookup
therefore does not substitute for current registry authorization. This is
in-process disk recovery/security evidence, not a physical revoke/reconnect.

Catalog-backed writes now use the same scope admission at construction as at
fresh dispatch. Resource reload requires a uniquely selected project/session
matching its catalog before reload capability filtering. Model/reasoning
selection requires ModelStatusCodec.selectionCurrent; reasoning also checks
current live choices. A newer live selection revision or mixed duplicate project
cannot produce a new command from an old displayed page. A manually entered
model name remains legal for host registry validation, and a separately read
status remains usable when no live model status was supplied. Saved pending
envelopes still use exact restore/retry rather than new command construction.

Pinned TLS is followed by one shared strict welcome/challenge parser for both
QR pairing and saved-device authentication. Connection negotiation cannot carry
request IDs, routing, owner/revision or result metadata, including null extras.
Invalid/order-mismatched/truncated negotiation stops before sending a token or
authentication/revocation signature. These framed-memory negative tests do not
replace current-APK Keystore/TLS/LAN acceptance.

The paged worktree-task dialog now exposes cancellation for eligible tasks
outside the 16-entry summary. Current summary status overrides older page
evidence. A page is eligible only while open, Connected, successfully loaded
and matching the selected project/session; owner/revision replacement closes
it and fences late reads. Loading/error/offline pages cannot supply a new cancel
target. Unknown outcomes need the host's runtime_active evidence. The command
still uses the current owner/revision and original task ID, persists intent
before sending and resolves only through a validated receipt. Page data is not
copied into the main projection or offline cache.

Task-list continuation must start at the preceding page's next offset and
cannot proceed past a terminal page. The client rejects duplicate task IDs
across the current page and up to seven retained navigation pages even if a
response repeats the same digest. Retained pages must form a consecutive chain
with one project/session, digest and total count. Invalid continuation enters
the existing list-changed error state and clears the active page; reloading
starts from offset zero. This is a bounded client consistency check, not a
claim that the client recomputes the full host list digest.

Completed tasks on later list pages now expose full output/history reads too.
A transient WorkerReadScope keeps only the validated task identity and current
owner/revision while its reader is open. Existing read codecs get a temporary
read-only projection containing that target; the main projection, durable
cache, cursor/ACK and write authority are untouched. Continuation, history
message/tool-argument reads and dismissal reuse that scope, with current summary
evidence taking precedence. Host owner/revision/session changes, conflicting
task identity, background/network loss, reset or new writes close the readers.
No read scope is restored from offline cache. Current-device and real-host
later-page output/history acceptance remain pending.

Completed worker reads also validate the summary and transient scope's complete
display metadata before constructing history/detail requests. Invalid subtab,
foreign scope, unknown state, failure code on success, contradictory active
runtime, oversized/NUL preview and completed-task stream evidence refuse the
read. Duplicate task IDs are rejected before filtering by state. Valid bounded
output previews, truncation and explicitly withheld output still permit reads;
metadata-only task pages retain their stricter no-preview rule. The current
Rust task/Kotlin lane checks mutated real metadata before request construction,
not production LAN or device execution.

Fresh worker cancellation uses the same complete display-metadata predicate
for summary target selection and command construction. Invalid subtab,
failure/output evidence on a running task, oversized/NUL stream or truncation
without a preview cannot authorize a new cancellation. Healthy accepted/running
tasks with bounded previews and unknown-outcome tasks with explicit runtime
evidence remain cancellable without requiring valid new-task submission tabs.
An already persisted exact cancellation keeps its original request identity
for explicit reconciliation even after metadata becomes unusable; no new
request is substituted and scanner/reset/foreground/network fences remain.

The scanner uses JourneyApps ZXing Embedded 4.3.0, restricted to QR, with camera
permission managed by its capture activity. No gallery asset, token or scanned
image is stored; barcode image capture is explicitly disabled. The scanner is
bounded to 120 seconds. Permission denial, timeout and absent camera produce
separate transient errors without replacing the paired host/cache or authorizing
control. A cancelled result is a no-op. The scanner cannot launch before saved
metadata loads, while connecting or while a write/revoke outcome is unresolved;
returned results are also fenced by those pending outcomes. A valid result still
requires human safety-code confirmation. No scanner error includes scanned text.

`tools/stage2_scanner_android_smoke.py` exercises the production Activity and
actual Android permission dialog, then result-state/Compose/diff tests on an
owned emulator. The first assertion-passing capture was black and is not accepted
as complete screenshot evidence. A new sampled-pixel guard and required
`scanner_permission_denial_nonblank_verified` marker now reject it; the guard
has direct Kotlin/JUnit tests, but updated connected tests remain pending after
the environment became sandboxed. See
`Docs/quality/stage2_android2unix/progress-20261002-scanner-evidence.md`.
Physical camera scan, permission re-enable/rotation/recreation and the complete
scanner lifecycle/permission matrix remain required.

The ViewModel now records an outstanding camera Activity result separately from
whether that result is still admitted. A scan disables another launch and
foreground refresh/write admission until its callback returns. Admission also
increments the connection generation, cancels the foreground subscription and
all detail-page reads, and changes a saved host to an offline display-only
projection before the camera launch. It does not wait for Activity.onStop, so
an already-owned read cannot publish live authority in that launch interval.
The saved transcript/cursor and encrypted metadata are retained; a foreground
cancel/error return requires a fresh authenticated read before controls return.
Cancel, local
reset or a new deep link invalidates the old result, but cannot reopen launch
admission while that Activity is still outstanding. Its late URI/error/cancel
callback only releases the outstanding slot; it cannot replace the new host
confirmation or recreate wiped credentials. Permission denial, timeout and
camera-unavailable release the slot normally. An admitted return resumes the
saved host's foreground read when no unresolved operation blocks it; an
invalidated late result cannot undo an explicit stop. This state is transient, not
stored in paired metadata; after process death an unowned result is ignored.
JVM admission tests and a prepared ViewModel/device regression cover these
rules. The scanner harness requires 24 exact state-suite identities and the
scanner_live_read_pause_verified marker; the previous suite cannot prove the
new live-read pause. Actual scanner rotation/process-death/camera execution
remains pending.

`PairingClient` uses TLS 1.3 and ALPN `zenpi/1`. Its trust manager checks the
leaf certificate expiry and exact QR-pinned certificate DER SHA-256 before the
token is transmitted. It does not globally disable TLS validation. Network work
runs on Dispatchers.IO; cancellation and a 15-second absolute timer close the
owned socket. Ambiguous failures are shown without automatic token retry.
No provider, shell, git or tool executes on Android.

Subscription response consumption budgets the actual received JSON body
lengths, including terminal control pages across the request, rather than a
compact reserialization of parsed JSON. A subscription body taking aggregate
traffic above 196608 bytes is refused at its prefix before allocation/read or
page publication. Ordinary replay event/snapshot accounting also uses received
bytes. Padded/escaped JSON cannot hide resource use behind normalization. JVM
framed-stream tests cover the exact boundary and preservation of the last
committed projection; this is not connected Android/TLS/LAN acceptance.
Transient-only subscription batches explicitly refuse phase/journal authority
fields (including null), and require the retained project/session to match the
requested subscription. Busy display updates preserve the transcript and cursor;
malformed authority cannot publish their accompanying controls or completion.
SubscriptionTest and current Rust-owner Busy frames consumed by RustAndroidWireTest
cover this boundary; physical transport and current APK execution remain pending.

Explicit pin/chain/certificate-validity failures are typed host-identity
refusals. TLS cause wrappers are inspected by type, with cycle protection,
not by diagnostic text. They strip stale in-memory control/transient surfaces
through the same authority-loss policy and stop the foreground automatic read
loop; ResumeWorker treats them as permanent for that scheduled opportunity.
Durable pairing/display/cursors and unresolved request identities remain, so
the user can explicitly reauthenticate, inspect the desktop or re-pair. Ordinary
TLS I/O failures/timeouts remain transient and do not impersonate pin refusal.
JVM trust/policy tests do not establish Android TLS-provider/device behavior.

Explicit ALPN mismatch and invalid complete welcome/challenge negotiation are
also typed refusals, separately from certificate identity failure. Both clients
use the same ALPN admission; reauthentication strips stale control authority
and stops automatic foreground/background retries when negotiation is refused.
The cause classifier is type-based and cycle-safe, never diagnostic-text based.
EOF, partial-frame EOF, timeout and socket I/O during negotiation stay transient.
Stored pin, display/cursor and unresolved write/revoke identities remain intact;
no protocol fallback or automatic invitation retry is authorized. Framed JVM
tests verify these classifications and prebuilt decision fencing, not a fresh
Android TLS/ALPN or connected LAN workflow.

An invalid device-authentication receipt now produces a typed
HostAuthenticationException rather than an ordinary wire error. Missing or
contradictory receipt fields, false authentication, foreign device identity,
unexpected authority and invalid device revision cannot leave old live
decision controls active after a reconnect failure. A local signing-device /
saved-host identity mismatch uses the same refusal before socket creation.
Authority-loss handling removes stale controls/transient views and stops
automatic foreground reads; ResumeWorker does not retry this scheduled
opportunity. Durable pairing/display/cursors and unknown exact request identities
remain. Valid read-only receipts, including legacy receipts without revision,
are unchanged. Generic I/O, truncation and diagnostic strings are not promoted
to this classification. Framed receipt and dispatch JVM tests do not establish
connected Android device/Keystore acceptance.

Authentication classification also applies before semantic receipt validation:
invalid frame decoding, unexpected event/message, wrong response ID/channel,
forbidden routing fields and success/error-code contradictions are typed
unconfirmed authentication failures when the outstanding request is authenticate.
Non-failed accepted/running/unknown outcomes reach authentication validation
before generic operation-completion handling. A complete hostile reply cannot
leave old decision authority active merely by failing an earlier reader check.
Typed host refusal codes still retain their normal handling, and transport EOF,
partial body/prefix EOF, timeout and socket errors remain transient. Normal
workspace/replay/subscription reader behavior is unchanged. Framed JVM tests
cover this path; real Android TLS/Keystore/reconnect acceptance is still required.

`FrameCodec` handles u32 network-order frames, rejects excess length before
allocation. A post-dispatch typed write may return null-result unknown_outcome
with a delivery authorization/availability code. ResponseReader preserves this
specific case after correlation/routing validation; HostClient throws a coded
WorkspaceUnknownOutcomeException without confirming the mutation. Authority
codes fence old controls while the exact pending envelope remains unresolved;
Busy is not revocation. Coded unknown steering responses stop receipt polling.
Unknown self-revocation likewise cannot authorize local key wipe. Read replies,
object-valued unknown results and successful replies with error codes do not
use this exception. JVM wire negatives and the current committed desktop
workspace/Kotlin lane cover the contract, not a real LAN delivery race.

`FrameCodec` rejects excess length before
allocation, malformed UTF-8/JSON, duplicate and unknown fields, unsupported
versions, invalid channels/outcomes and bounded payload complexity. Common
raw-wire fixtures and schema-contract.json now exercise all 33 operation names
and read/write classification, six outcome states and twelve error codes in
both codecs. The contract-driven tests check owner/revision presence, maximum
u64 revision, wrong channels and unknown enums. They do not validate operation
arguments or authenticate a grant. The same reviewed contract now generates
RemoteSchema.kt for FrameCodec's version, enum admission and write classification.
It also supplies payload field sets and channel rules for all thirteen message
types. Both codecs validate common minimal fixtures, every omitted required
payload field, unknown fields and wrong channels; Android additionally compares
the generated maps with the contract. Fixture device proof values are public
structural data, not admitted credentials.
python3 tools/generate_stage2_android_schema.py regenerates it; --check fails
closed on stale output without modifying source. Gradle preBuild and the cached
source check require this verification, and APK attribution includes the
generator, contract and generated source. Python 3 is required by this build
gate. The guarded APK builder independently checks the generated source before
starting Gradle and after successful process completion, and current-build
verification rechecks it before device harnesses can install. Matching hashes
alone cannot promote an edited/stale generated source to valid APK authority.
Preflight refusal removes the builder's old success record without regenerating
source, starting Gradle or fabricating a process-attempt log.
Operation-specific argument/result types are not generated, and broader
Rust/Kotlin parity properties remain required before accepting S2-030 as a
complete shared-schema deliverable.

After invitation acceptance, Android shows device registered and host control
unavailable. With an explicitly attached host session and a local project Read
grant, Refresh host displays authorized primary tabs and bounded transcript.
Host snapshot parsing rejects session/cursor mismatches, missing selections,
duplicate identities, invalid types/roles/phases and cursor overflow. Refresh
does not mutate host selection or retry a write. With an explicit local
Read/Control grant, live primary tabs select the host workspace; open/close
dialogs send owner/revision-fenced commands. Unknown outcomes retain the exact
encrypted pending request and offer explicit same-ID retry instead of automatic retry.
Offline cache restore drops workspace-control authority until a live attach.
Live attach also fetches the authorized secondary-tab projection, rejecting
owner/revision/session mismatches, unknown fields, leaked root fields, invalid
IDs/selections and capacity/concurrency violations. Secondary tab selection,
confirmed rename and adjacent reorder use stable IDs and both host revisions.
Main stays first. An explicit directory-tab capability adds a dialog for an
existing project subdirectory and confirmed close. These operations only attach
or remove an InPlace tab record; close does not delete the folder or files.
The same pending-command mechanism covers these writes;
cache v2 retains the redacted secondary display, stable IDs, order, selected
tab and displayed concurrency alongside the transcript/cursor. It omits all
secondary capabilities and workspace owner/revision; restored tabs are disabled
until a fresh live attach. The cache shares the live subtab view validator,
rejecting malformed identities, foreign project bindings, duplicates, invalid
selection/kind/count/concurrency, leaked paths and unknown nested fields. A
strict v1 record remains readable and upgrades on its next write without
inventing secondary tabs. AES-GCM storage/AAD and the immutable pending-command
record remain unchanged. Git add/delete and concurrency stepper are still
unavailable, not inferred from read/control permission.
Pending writes are saved before sending and restore offline across process
death. Cancel and rescan preserve unresolved requests; Forget explicitly wipes
them. A confirmed result clears the request atomically with its display/cursor.
Private signing keys remain in Android Keystore;
Forget device locally wipes local signing/metadata keys but does not claim host
revoke. Revoke device on host is a separate confirmed action. It authenticates
the current key, sends the current device-ID/revision-fenced self-revoke and
only removes local metadata/signing keys after a matching successful host
receipt. Signing-key removal additionally checks the revoked device ID against
the alias's current public identity while holding the same in-process lock as
key loading/creation/removal. A delayed receipt cannot delete an app-created
replacement under that alias: missing or mismatched keys retain metadata and
the pending revocation marker for explicit reconciliation. Both direct revoke
completion and saved-receipt reconciliation pass the same coroutine/generation
admission into identity removal. It checks under the identity lock before and
after reading the current public identity, then again after opening Keystore
and immediately before deleteEntry. Cancellation while waiting for that lock or
during those reads therefore retains the signing key and pending metadata.
DeviceRevokeTest covers all three admissions and a cancelled blocked-lock
operation before a successful matching cleanup; its exact named phase and
boolean are required by the current desktop lifecycle verifier. The source is
compiled only, not connected evidence. Local Forget remains a separate explicit
unconditional reset. External Keystore changes are not an atomic transaction
with these app-local checks. Prepared real-key tests cover deletion/replacement,
unchanged encrypted metadata and matching-key removal; connected execution is
still required.
Pending workspace commands disable revocation. Network/response/storage
failure retains local credentials and reports unconfirmed revocation; it is
never automatically retried. A UUID intent is encrypted atomically before the
request can be sent, bound to the saved host/device. Foreground/network restore,
cancel, rescan, cache writes, new workspace writes and WorkManager resume cannot
clear or bypass an unresolved intent. It survives process recreation; only a
matching live host confirmation completes key/cache removal, or explicit local
Forget abandons the local state without claiming host success. Incoming QR URIs
wait for initial metadata loading, so a cold-start URI cannot bypass a saved
pending workspace/revoke marker. Check revocation status explicitly sends a
connection-challenge signature bound to the saved device/original request/query
IDs. It retrieves only that receipt even after normal device authentication is
revoked. A matching committed receipt completes local key/cache removal;
unmatched receipt or network/proof failure retains the marker and credentials.
The query grants no project access and never re-sends revocation. Older host
records/local-operator revocations without that receipt still require desktop
inspection. This path remains explicit, not foreground/background auto-retry.
Fresh authentication fails for revoked
keys and a new pairing must use a new key.
S2-013/037 complete ACL/remote-revoke UX, complete host runtime adaptation,
workspace/worktree controls, transcript/approval/diff/PTY, lifecycle/resume,
LAN camera tests and API 31/34/35 coverage remain unfinished. These pending
features must not be replaced by the current read-only transcript behavior.

## Shared Desktop Owner

With an existing configured workspace and session, the explicit desktop entry is:

```sh
target/debug/zenpi host pair --desktop --workspace /absolute/workspace --session /absolute/session.jsonl
```

It opens a full-screen QR over the production desktop owner; R refreshes the
invite and Esc cancels it and returns to the desktop. Local device/project Read
grants are still required. Registered clients keep reading the same live owner
until desktop exit, which stops the listener. Foreground synchronization uses
ordered transcript replay from an independent event cursor after the initial
snapshot. This is pull synchronization, not live provider-delta server push
or live provider delta push. Desktop mode now separately advertises prompt_control:
Android's Message/Send control submits to that desktop's existing runner and
project Agent. It persists the exact pending envelope before transmission;
accepted admission is not provider success, and an ambiguous outcome permits
only explicit same-envelope retry. Cached display drops prompt authority.
Main-tab idle prompts, request-targeted cancellation, active remote-prompt
steering and bounded tool approval are implemented. Full command parity,
local-turn steering policy and per-worktree concurrency remain required.
Selected directory/worktree roots now route through
the desktop's request-scoped tool context, rather than silently using main.
Android directory execution is exercised by reading distinct host files in two
selected subtabs. Live worktree authority now exposes Add Git worktree and
confirmed Remove Git worktree actions, using fresh-branch names rather than
client-selected destination paths. Both retain the same owner/revision and
encrypted pending-request mechanism. Cached views never retain worktree
authority. The host uses the local bounded Git safety helpers and reserves
request identity durably before Git; uncertain Git/checkpoint outcomes never
auto-reexecute. Dirty tracked/staged/untracked/ignored files keep the worktree
and tab; successful ordinary removal retains the branch. The host now retains
typed pre-Git intents and offers local `--list-worktree-intents` and
`--reconcile-worktree REQUEST_ID --device DEVICE_ID` actions with the original
workspace/session. Stop the desktop host first. They inspect observed completed
Git effects and settle the original receipt without replay/deletion. Restart
the host and explicitly retry the saved Android envelope to resolve it. Divergent
state/no-effect failure/legacy intent repair, independent worktree sessions and
concurrency, and full Git acceptance remain required.
The desktop harness flag `--with-worktree-workflow` (with workspace control,
separate from extension fixtures) drives production Android create/read/remove
actions and independently inspects host Git state, selected execution roots,
provider outputs and branch retention.
Loaded extension tools and hooks also reinitialize against that selected root,
with fresh original-workspace restoration authority prepared before execution.
Failed preparation leaves the original root/lease untouched, and cancellation
reaps initialization subprocesses. The optional desktop harness flag
`--with-workspace-extensions` (with `--with-workspace-control`) exercises two
Android-selected directory reads through a real extension subprocess and checks
its lifecycle/root logs independently. The passed receipt is
`Docs/quality/stage2_android2unix/desktop-android-extension-workspace-20261002.json`.
This does not make native extensions a sandbox or accept all extension parity.

While the selected project's own remote prompt is accepted/running, the composer
switches from Message to Steer and sends the exact active request ID with its
text. Pending writes, revoked/offline authority and cached projections fence
the action. A steering acknowledgement requires a durable host input-queue
receipt, not just an in-memory ticket. Brief receipt polling reuses the exact
encrypted envelope; it never creates another input identity. A unresolved
receipt remains available for explicit exact retry.

An accepted/running status exposes a Stop control only with live prompt authority.
It sends a new owner/revision-fenced command targeting the original request ID.
The desktop cancels only that device's still-active remote prompt, not a later
local job or another device's turn. Cancellation admission is not a terminal
cancelled claim: the worker's journaled outcome supplies the terminal status.
Prompt/cancel admission resolves after the authorized workspace/subtab read,
without waiting for an execution-locked transcript; foreground replay continues
separately. Unknown outcomes retain the encrypted envelope for explicit retry.
Validated write receipts now clear that envelope durably before subsequent
projection reads. If those reads fail, the confirmed operation is not resent;
display remains without mutation authority until a fresh read succeeds.
The shared read-response consumer refuses contradictory non-failed outcomes
with non-null typed error codes before any validation callback or replay page
publication. In particular, a succeeded subscription page carrying a revoked
device code cannot replace the last committed display/cursor. Absent or explicit
null code remains compatible. Fragmented replay tests cover both first-page
refusal and later-page preservation; this is not a live TLS/device result.
Malformed/missing receipts or failed local confirmation persistence retain
the original envelope. Production TLS instrumentation injects a post-receipt
read failure and checks encrypted pending state remains cleared.
Subtab success receipts also enforce operation semantics before that clearing:
select must report the requested active subtab, and close must not report the
removed subtab as active. A different well-formed ID plus incremented revisions
is insufficient. Rename/move/concurrency may legitimately retain another active
tab; their existing owner/project/revision checks remain unchanged. JVM tests
exercise fragmented wrong-target responses without confirmation; Rust tests
check the producer's selected/closed identities and exact-retry behavior.
Cancellation admission receipts now survive desktop restart in its existing
owner checkpoint. An interrupted reservation remains unknown and is never
re-executed on retry. A full global request ledger and interrupted-execution
reconciliation remain unfinished.

Prompt status rows are tied to request IDs, not inferred from assistant text.
The host exposes bounded device-scoped status while the provider owns its
execution lock and journals terminal progress. Android can show status from a
workspace read even if the following transcript read is Busy. Cache v3 retains
terminal display, turns accepted/running into unknown offline and does not grant
controls; legacy v1/v2 records remain readable. Cancellation before the worker
starts and abnormal worker termination still require reconciliation coverage.
Failed rows additionally accept a bounded optional failure_code from the
existing wire error enum, only when state is failed. Input rejection, busy
owner, resource limit and access denial have distinct fixed display labels;
remaining codes use the generic failed label. Null, non-string, unknown code,
raw diagnostic fields and failure codes on non-failed states are refused.
Cache v3 retains this terminal diagnosis without restoring mutation authority
or live text; older cache records remain readable. The status label has a
weighted, two-line ellipsized layout beside the request ID and any active
cancel button. JVM codec/cache tests run locally; the new 320dp offline Compose
case is source-compiled only, not claimed executed on a device.

Live authorized workspace reads now carry bounded pending
approval summaries from the desktop's existing coordinator, even while the
Agent execution lock is held. Android shows the tool, side-effect category and
request ID. The confirmation additionally renders an optional host-redacted
diff path and patch in a scrollable, monospace view. Patch data is capped at
2048 UTF-8 bytes, with visible redacted/truncated/withheld flags; recognized
credential files are withheld entirely. Arbitrary unknown secrets are not
guaranteed detectable. Raw tool arguments are never transmitted here.
The digest binds the full host request for later decision correlation but does
not authorize a decision. Summaries are omitted from cache and hidden offline.
Separate live approval_control enables Allow/Deny icon actions and explicit
confirmation. The captured summary/digest must still match the latest live
pending request; pending writes/revocation, offline and connecting states fence
confirmation. One-shot approval saves its exact request/digest/decision through
the existing pending-write mechanism, validates a matching admission receipt,
and never creates a remembered permission. The host has a bounded durable
approval ledger; uncertain outcomes require explicit same-envelope retry.
The production PTY/Android harness verifies two actual provider write_file calls:
Android confirms one-shot Allow/Deny over TLS, the host writes only the allowed
file, and checkpoint/admission/audit records match. Live diff path/patch display
is also asserted before confirming either decision. Broader sensitive-preview
coverage and
the broader tool/race/cancellation matrix remain required before claiming
complete remote approval parity.
Remote decisions also carry host-authenticated device/request/project/session
attribution through the coordinator to the worker's durable approval audit.
The production harness matches this attribution and original request digest
against the checkpoint receipt; it is not an Android-supplied tool field or a
remembered permission. Simultaneous local/remote coordinator answers have one
winner in Agent tests, while full distributed race/fault coverage remains open.

On macOS, the production PTY/Android check is:

```sh
python3 tools/stage2_desktop_android_smoke.py --serial emulator-5580 --receipt Docs/quality/stage2_android2unix/desktop-android-shared-owner-20261001.json
```

It independently decodes real terminal QR pixels using macOS Vision, then
checks Android against the same desktop owner. A second provider answer is
submitted through desktop keyboard input, and Android must observe the new
turns with the same session ID and an increased journal sequence. The test also
checks revocation and listener teardown. It preserves NO_COLOR=1 during QR
validation. adb reverse is still not evidence of camera scanning or LAN routing.

For direct private-interface TLS validation, add `--lan-ip` with the current
host RFC1918 IPv4 address, for example:

```sh
python3 tools/stage2_desktop_android_smoke.py --serial emulator-5580 --with-workspace-control --lan-ip 192.168.50.14 --receipt Docs/quality/stage2_android2unix/desktop-android-direct-lan-interface-20261001.json
```

This binds the production listener to that interface, requires the decoded QR
to advertise exactly that address, and connects Android without `adb reverse`.
The harness rejects public/non-local addresses and pre-existing reverse
mappings without deleting them. The reverse table must also remain empty at
the end. Receipts distinguish `lan_interface_tls_verified` from the still
unverified physical-LAN-peer and camera-scanning requirements. An emulator
accessing a host interface through its virtual NAT is not a physical phone on
the same Wi-Fi. The full LAN/camera acceptance remains open.

The same harness also keeps the real desktop running while it force-stops a
proved-live Android test process, submits another desktop prompt while Android
is stopped, then starts a different Android process. That process must restore
the encrypted display/cursor offline, authenticate with its surviving Keystore
identity, and replay the missing desktop turns from the saved cursor without
rescanning or substituting a new snapshot. It then verifies host revocation.
An additional isolated fixture tests encrypted cache restoration across two
independent instrumentation processes and wipes their test keys afterward.
This does not prove Doze, background service operation, network switching or
the API 31/34/35 lifecycle matrix.

Add `--with-workspace-control` to the desktop harness to grant explicit
Read/Control for its temporary initial/child projects and run production
ViewModel-driven Compose open/close buttons. Desktop checkpoint inspection
proves the matching tabs and active owner. It also leaves a request pending
after its real host commit, kills a proved-live Android process, and requires
the new process to resolve the original request without advancing revision
again. This is controlled crash-boundary evidence, not a complete fault matrix.

The control-mode harness also runs a real WorkManager task with isolated test
metadata/Keystore aliases. It submits another desktop answer after foreground
models stop, proves the cache has not advanced, and requires the worker to
commit the new turn/cursor. A second task must skip an unresolved workspace
request without changing host revision. Separate store tests prove stale
background commits cannot overwrite foreground updates, pending writes,
re-pair or wipe; missing-key lookup never creates a key; and backup-only
AtomicFile recovery retains the display/cursor. These tests do not prove OS
scheduling after process death or under Doze. WorkManager's merged
foreground-service declaration/permission are removed because this client
does not use foreground workers.

The production ViewModel/Compose harness now attaches two pre-existing project
subdirectories, reorders them, selects by ID, and closes each. Desktop owner
checkpoint checks prove the intermediate order/selection and final main-tab
fallback. Sentinel files must survive both closes. This proves existing
directory-tab interoperability, not Git worktree creation or deletion, new
workspace provisioning, or provider execution in the selected subdirectory.

Primary and secondary tab rows scroll horizontally and keep bounded touch
targets with two-line ellipsis labels and full accessibility names. Capacity
tests cover 64 projects and 32 secondary tabs at 320 dp width, including cached
offline display without mutation authority. This does not replace host-backed
worktree creation/deletion or concurrency controls.

The desktop harness also force-stops a proved-live Android process after host
self-revocation is durably recorded but before local completion. A different
process must load the surviving key and exact intent, remain offline despite
foreground/network callbacks, fail normal authentication, reject an unrelated
receipt, and resolve the original receipt through the production Check button.
Only then are its signing alias and encrypted metadata absent. This covers
actual process death at the local-completion boundary, not a dropped TLS reply.

## Cached Cursor Interop

Saved cancellation requests use the same nonblank, 128-byte, no-control-character
target boundary as the Rust owner. Invalid restored targets cannot become
pending retry envelopes. Valid target IDs, including internal spaces and UTF-8,
retain their exact request identity; restore does not generate a replacement.
Approval construction requires the selected project and current session to
match as well as live approval capability and exact pending target. A changed
project session cannot lend its authority to an old approval. Dispatch still
rechecks owner/revision/scope and host authorization remains authoritative.

ApprovalDecisionPolicy is shared by entry controls, confirmation and ViewModel
preflight. It fences scanning/reset/unresolved intents and requires a unique
selected project with a matching session plus exactly one request/worker-session
approval identity equal to the displayed target. Construction and fresh command
dispatch revalidate this identity too. Same request IDs in different workers
remain independently selectable; duplicate identity records cannot authorize a
decision. Exact persisted pending retries stay separate from fresh decisions.
JVM and actual Kotlin/Rust coordinator fixtures cover this boundary; prepared
Compose confirmations are compiled only, not current connected-device evidence.

Live reconnect's TranscriptRestorePolicy is separate from session-change probe
admission. SessionChangeProbe now requires
a complete validated automatic-read frame, exact envelope fields and empty
inspect/subtab_list arguments before requesting a fresh authorized listing.
Malformed/null identity, foreign cursor, attached authority or explicit path
arguments cannot trigger recovery; the original refusal remains unchanged.
SessionChangeProbeTest checks zero probes across hostile requests and preserves
valid one-probe recovery. This does not claim connected desktop-switch evidence.
TranscriptRestorePolicy checks canonical u64 event/journal counters and event <= journal, bounded
unique project identities and exact selected-project/session membership before
replay or Busy fallback. Invalid state cannot normalize a cursor or swallow Busy;
a fresh snapshot remains the recovery path, not retargeted replay. JVM matrix
and current Rust snapshot mutations verify refusal; valid zero/max-u64 remain.
The separate cached-cursor ACK/resume source lane is run by
stage2_android_source_check.sh through remote_android_ack_interop and
AndroidRustAckTest. Kotlin restores a
projection cache and emits ACK frames with the same EventAckCodec request
builder used by HostClient. Rust dispatches them against a current authenticated
read-only device, checks the persisted checkpoint, restores a new host owner,
and appends a desktop turn before replay. Kotlin consumes fragmented Rust ACK
receipts and replay, restores the updated cache and generates the next ACK.
The ACK builder/receipt validator now reject malformed retained scope and
noncanonical or out-of-u64 cursor strings before numeric conversion. ACK
admission checks the supplied frame exactly matches the expected project,
session and sequence, permits only an optional valid correlation ID and runs
frame validation before exchange. A mismatched frame cannot consume a receipt
or become a deferred Busy success. JVM adversarial construction and the actual
Rust ACK lane test zero exchanges for altered scope before the original frame.
Duplicate/lower ACKs cannot regress the durable cursor; busy ACK defers, future
and unauthorized ACKs fail without changing checkpoint/journal. The lane is
fixture-local domain/frame evidence, not encrypted Android storage, process
death, sockets, TLS, LAN, camera or device acceptance.

## Local Reset And Corrupt Metadata

Local forgetting now requires a confirmation distinct from host revocation.
It removes the signing alias, encrypted host/display/cursor metadata and
unresolved local requests; it does not revoke the desktop registry entry or
undo a host-side operation with an unknown outcome. Corrupt metadata exposes
an explicit reset route instead of silently replacing credentials.

While resetting, the ViewModel fences new scans, incoming invites, cancellation
and foreground/network callbacks, and joins its owned connection and read jobs
before deleting credentials. A no-backup `.reset` marker is written and its
file descriptor synced before key deletion. Store reads, writes and background
resume commits reject that marker. It remains until metadata key absence and
AtomicFile base/backup/new file deletion are checked. Failure leaves an explicit
retry-only state. Existing paired connections load an existing signing key;
they do not create a replacement identity when the old key is missing.
Transport entry points now require an explicit DeviceIdentity argument:
HostClient.inspect and PairingClient.pair never implicitly call loadOrCreate.
New key admission remains in the foreground, fresh-invitation confirmation
owner; saved-host/background reads load only existing keys. The routing device
test checks incorrect SAS and saved device-ID mismatch reject before socket
binding, and verifies the supplied key remains unchanged.
Pairing key admission shares the ViewModel storage mutex with credential save
and local reset. Cancellation/generation is checked after taking that mutex,
before non-suspending Keystore work. An already-started creation holds the lock
until it returns, even if its coroutine was cancelled and the connection field
was cleared. Reset therefore cannot wipe first and leave a late-created key.
JVM regression tests exercise this ordering with a held blocking creator; they
do not substitute for Android Keystore device acceptance.
Invitation freshness is also rechecked inside that admission barrier, after
waiting for storage. An invitation that expires during the wait cannot start
key creation; the transport separately rechecks before sending its one-use
pairing request. The boundary regression checks exact expiry refusal and the
still-fresh case with a controlled clock.

The currently retained connection and all ten read jobs are included in that join barrier,
including session history as well as the session catalog. LocalResetTest holds
each coroutine's cancellation cleanup separately and checks reset stays pending
with credentials intact until every job finishes. Session history is released
last so an omitted join cannot be hidden behind another still-running read. The
desktop suite and lifecycle receipt validator require its named completion
phase `local_reset_all_owned_jobs_joined_verified`, not the old two-read phase,
a successful runner count or cleanup boolean. This regression
is source-compiled only until current APK/device execution is available.

All non-reset ViewModel launches are registered in OwnedJobs before execution.
Completion, not cancel(), removes their registration. Closing a panel or
changing the route can clear its active-job field while cleanup still runs;
local reset drains that registration set as well as the currently retained
fields before deleting credentials. Initialization and pairing reconciliation
are registered too. The reset coroutine itself is excluded to avoid joining
itself. Main-owner resetting state fences new operation admission while the
drain runs; this registry is not a general concurrent-admission barrier.

OwnedJobsTest executes JVM regressions for cancelled-but-not-completed jobs,
all-cancel-before-join ordering, immediate completion/cancelled scope and a
cancelled reset wait that cannot claim cleanup completed. A separate device
regression closes a registered connection/session-history read first, verifies
both fields are null, then checks reset retains credentials until their held
cleanup completes. Its required phase is
`local_reset_retired_jobs_joined_verified`; it is source-compiled, not executed
on a device for this revision. This adds one desktop base-suite test (32 base,
83 with workspace controls, 84 with workspace controls and Doze).

This is not a claim of filesystem directory durability across power loss,
physical secure erasure, cancellation of every already-running background
request, or host revocation. The local-reset instrumented tests and confirmation
UI compile, but have not run on an emulator for this source revision. See
[`progress-20261002-local-reset.md`](quality/stage2_android2unix/progress-20261002-local-reset.md)
for verified scope and remaining gates.

## Host Resource Selection

Resource insertion/reload/page controls require the catalog's exact current
project/session and no error/reset state. Continuation decode validates an
adjacent requested offset and rejects resource identities repeated against the
current or seven retained forward pages. Retained scope/generation/snapshot
chains must be consecutive; cached backward pages check generation too. Late
successful reads for a replaced selection close the catalog. No full digest
recomputation or immediate detection of an unobserved host reload is claimed;
host generation/snapshot/CAS validation remains authoritative.

The composer now exposes a native resource menu for the desktop Agent's loaded
Markdown skills and prompt templates. Catalog reads use the same pinned TLS,
device proof, LAN binding, timeout and socket-cancellation path as host files.
Only scoped metadata and invocation commands cross this read boundary, never
resource bodies or source/provenance fields. Template descriptions are explicit
frontmatter only: a missing/empty/blank description is not replaced with a
body snippet on the phone, while the desktop's local fallback remains intact.
Selecting prepends the command to
plain draft arguments and does not send. Reload-required skills, offline or
pending-write state, lost prompt control and an active prompt disable selection.
An existing slash/shell draft is retained instead of silently replacing it.

Catalog data is transient, cleared with other foreground reads on background,
route/selection/reset boundaries and absent from the encrypted display cache.
Each bounded page has up to 32 entries and 1024 UTF-8 description bytes. Native
previous/next controls preserve the draft and bind all navigation, including a
return to the first page, to the same snapshot identity. The client retains one
page rather than accumulating the catalog. Reload/restart changes invalidate
navigation, clear the page and require a fresh open. Remote reload of already
selected paths is implemented through the typed resources_reload owner route.
Resource path/settings management and complete desktop resource parity remain
unfinished.
Compose selection/offline/stale/paging tests compile
but have not run on a device for this source revision. Host-side authenticated
TLS and production headless expansion are verified separately; see
[`progress-20261002-resource-catalog.md`](quality/stage2_android2unix/progress-20261002-resource-catalog.md).
Paging and snapshot-bound navigation evidence is recorded in
[`progress-20261002-resource-pages.md`](quality/stage2_android2unix/progress-20261002-resource-pages.md).
Explicit-only remote description evidence is recorded in
[`progress-20261002-resource-description.md`](quality/stage2_android2unix/progress-20261002-resource-description.md).

The headless device harness now prepares 40 filler templates, one tail template
and a catalog skill, then drives the production ViewModel/Compose menu through
forward/back/forward pages before selection. It verifies unchanged draft/cache/
cursor state until explicit prompt construction, uses the selected command for
the pending-request process-death workflow, and checks exact expanded provider
input and raw-command-only journal provenance. This prepared acceptance chain
has compiled but has not run with current APKs; see
[`progress-20261002-resource-device-gate.md`](quality/stage2_android2unix/progress-20261002-resource-device-gate.md).

While a message is running, steering remains a plain-text queue input. New
leading skill/template/slash/shell commands and explicit @ file references are
rejected before local pending-intent persistence; Send stays disabled without
discarding the draft. Inline paths/commands and email addresses remain text.
The same Rust check gates both desktop TUI enqueue/edit and authenticated remote
steering. Historical exact pending envelopes still restore for receipt lookup,
not fresh resubmission. JVM boundary/command tests pass; Compose draft-retention
tests compile but have not run on current devices. See
[`progress-20261002-steering-input.md`](quality/stage2_android2unix/progress-20261002-steering-input.md).

## Host Model Status

The model dialog's read-only projection shows the current host model, reasoning level,
effective context/reserved-output budgets and known provider capabilities.
It is available to Read-only devices without prompt mutation control. Unknown
model/capabilities remain explicit. No endpoint, account credentials, connection
snapshot, catalog prices or source paths are returned. Status is transient and
cleared/cancelled with other foreground reads; local reset joins its job before
key deletion. Refresh is disabled offline, while loading and with pending writes.

This status projection does not itself switch a model/connection, report measured context usage or imply
full settings parity. Strict codec/JVM tests pass and Compose read-only/offline
test compiles; current-device execution remains unverified. APK build diagnosis
with a writable temporary Gradle home now reaches a separate daemon socket-bind
EPERM, not merely the original user-cache lock. See
[`progress-20261002-model-status.md`](quality/stage2_android2unix/progress-20261002-model-status.md).

## Host Model Selection

Control-authorized current hosts advertise model_control. For a scoped,
registry-bound status, the dialog adds an editable Model ID and explicit Apply
action. Read-only/older hosts have no mutation control; offline, loading and
unresolved writes disable it. Selection retains the host's backend/account and
reasoning effort; a model incompatible with the retained effort/history is
visibly refused, not silently substituted.

The command includes both workspace revision and model selection revision. It
uses the existing encrypted pending WorkspaceCommand ledger, authentication,
pinned TLS, immutable exact-request retry and receipt-confirmation boundary.
Receipt validation requires request/project/session/model equality and a newer
unsigned selection revision; the revision is a journal sequence, not expected
revision plus one. A failed refresh after host confirmation does not regenerate
or retry the mutation. Status remains transient and must be reread to inspect
the current model. No provider credential, endpoint or selection snapshot is
stored on Android. Model catalogue choice, effort/account mutation, full
settings parity and current-device acceptance remain unfinished.

## Host Reasoning Selection

The current host advertises reasoning_control separately. Its model_status
adds a unique bounded array of choices validated against the current registry
model and actual provider protocol. Non-registry owners have none; older hosts
with no choices field remain display-only. The Android dropdown never invents
levels from model names or generic capability flags. Provider default is JSON
null, distinct from the explicit none level. Selection needs explicit Apply;
offline/loading/pending writes disable it.

reasoning_select carries the same project/session/workspace/selection authority
as model_select and shares encrypted pending-write, pinned TLS, durable receipt
confirmation and immutable retry handling. Host refusal or uncertainty does not
claim successful settings. New JVM tests validate null/none, allowed choices,
old-host fallback and exact receipts; Compose dropdown/offline tests compile
only. Current-source APK, device execution, account selection and complete
settings parity remain unaccepted.

## Host Model Catalog

The host model dialog now offers a paged catalog beside manual Model ID entry.
Each entry contains only ID, lifecycle and current selectable flag. The flag is
validated against the retained backend, effort and history; selecting an entry
only changes the draft and requires Apply. Unsupported entries are disabled.
Unknown/custom IDs can still be entered manually. Previous/next controls use
16-entry snapshot-bound pages, preserve the draft and disable offline/loading/
pending. A changed host selection or catalog rejects an old page, clears status
and requires refresh. No prices, source paths, credentials or endpoint metadata
are cached or projected.

Current selection controls also require the displayed status to match the
selected project/session and project membership, with no model error or local
reset. A fresh status read may recover an error; old page navigation and Apply
cannot. Responses must follow adjacent 16-entry offsets and strictly increasing
UTF-8 model IDs across the page boundary in either navigation direction.
Retained previous pages share one scope, selection revision and snapshot, with
no repeated IDs across the bounded seven-page history. A late response for a
changed selection closes the panel. A pushed newer selection discards old
page history rather than presenting it as valid. Host revision checks remain
the final write authority; these client checks do not replace them.

Legacy hosts without catalog remain manual-entry/display compatible. Reads use
the existing pinned authenticated foreground route and shared cancellable model
job, including reset join. New strict JVM pagination/shape/Unicode order tests
pass; Compose draft-only/disabled-entry/offline tests compile. The prepared
headless device harness now chooses its model from the catalog before Apply.
Current APK/device execution remains unverified; see
[`progress-20261002-model-catalog.md`](quality/stage2_android2unix/progress-20261002-model-catalog.md).

## Selection Device Gate

The prepared production headless/Android process-recovery harness now also drives
the real Model ID and reasoning dropdown controls between its two existing
provider turns. First turn must use gpt-5.2/high; after confirmed model and effort
changes, the second template-expanded turn must use gpt-5.2-2025-12-11/low.
The UI rereads status after each mutation, checks strictly newer selection
revision, settled encrypted pending writes and unchanged transcript cursor,
and captures the selected settings screen. Background fence probes include both
mutations. This adds no extra provider calls and retains the exact second-prompt
retry after actual process death.

The Python verifier requires two matching reserved/selection/succeeded chains,
device/project/session/fingerprint/operation checkpoint bindings, journal
selection revisions and actual provider model/effort. Selection must occur after
the first prompt completes and before the second is admitted. The API 31/34/35
headless matrix now requires android_model_reasoning_selection_provider_verified
and a checksum-verified selection screenshot; old receipts cannot satisfy it.
The harness compiles and validator tests pass, but current APK/device execution
is not yet verified. See
[`progress-20261002-selection-device-gate.md`](quality/stage2_android2unix/progress-20261002-selection-device-gate.md).

## Foreground Model Synchronization

Bounded authenticated subscription views now include the existing host model
status and first catalog page, or null while the owner is Busy. Legacy two-field
views still decode. Local desktop model/reasoning commits wake the subscription;
these changes do not manufacture transcript events, cursor progress or ACKs.
The open model panel adopts newer revisions and resets stale drafts without
sending mutations. Equal snapshot updates preserve only the catalog page,
not the old model-status metadata: effective budgets/capabilities/reasoning
choices refresh even when selection revision and catalog digest are unchanged.
The seven-page back history remains available, and returning to a retained
page adopts current metadata rather than reviving its old budget. Actual
Rust-owner/Kotlin interop changes the Agent context budget to 32000/2048,
requires unchanged selection/catalog/journal and checks the merged Android
status. This is source interoperability, not connected budget UI acceptance.
Changed snapshots replace stale pages. In-flight page reads cannot overwrite newer
pushed selections. Model status/catalog remains transient and is excluded from
encrypted cache and restored authority. Malformed, foreign or regressing status
refuses the whole batch before publication, including after a Busy null sample.

Rust TLS stream tests and 164 JVM tests pass; the new Compose pushed-selection
test compiles only. This is not current-source APK/camera/LAN/device acceptance.
See [`progress-20261002-selection-sync.md`](quality/stage2_android2unix/progress-20261002-selection-sync.md).

## Backup And Migration Protection

The app explicitly disables backup and references both legacy full-backup and
API 31+ cloud/device-transfer XML policies. All app file, database, preferences,
external and device-protected domains are excluded. Paired metadata continues
to live in noBackupFilesDir with non-exportable Keystore keys. Device moves or
reinstalls require fresh QR pairing; copying encrypted metadata without its key
must fail closed, not generate a replacement key or discard an unresolved write.

The cached source verifier now checks this policy with
`python3 tools/validate_stage2_android_backup.py`. New instrumented tests inspect
packaged policy and encrypted legacy restore, missing keys, copied/tampered
ciphertext and complete namespace-local wipe. They compile but have not been
device-executed. Rust migration tests and JVM legacy cache upgrade tests pass.
Actual merged APK, backup transport and uninstall/reinstall gates remain open;
see [migration.md](quality/stage2_android2unix/migration.md).

## Shared Protocol Corpus

Rust and Android now decode the same 51-case raw-wire corpus under
fuzz/remote_protocol without preparsing it. Gradle unit resources and the cached
source verifier both load that single corpus. Android preserves numeric token
spelling for strict unsigned fields, refuses null failure codes, malformed
Unicode, non-finite numbers, leading BOMs and noncanonical pairing-key base64url.
Malformed programmatic Unicode also fails before a frame prefix is emitted.
Nullable optional routing fields follow Rust's absent semantics. Both test
suites exercise 2048 deterministic bounded mutations and exact complexity
boundaries. This is codec interoperability evidence, not device/LAN acceptance
or coverage-guided fuzzing; see the corpus README and current progress receipt.

## QR Renderer And Parser Interoperability

Rust and Android share 45 raw invitation cases with a recorded validation
clock, normalized address and six-digit safety code. They agree on private
IPv4/IPv6, explicit loopback fixtures, expiry boundaries, query ordering and
escaped field names, and refuse duplicate fields, origin changes, malformed
tokens and unsafe address classes. Host invite validation now also refuses
IPv4 link-local and unsafe IPv4-mapped IPv6 targets; Android mapped broadcast
checks use the parsed address. Scheme/authority and version spellings are exact.

The source checker now runs the remote_qr_fixture Rust example on every check,
generating current-source luminance images from production terminal qr_lines.
Its exact 54-case manifest covers private IPv4, IPv6, minimum/maximum ports,
a full-length private IPv6 address, explicit IPv4/IPv6 loopback fixtures and
48 deterministic varied fingerprint/token payloads across those endpoints.
Kotlin decodes each with the app's ZXing engine, then verifies URI, address,
safety code, 300-second admission window, exact wall expiry refusal and no
revival after expiry. Historical IPv4/IPv6 resources remain the ordinary Gradle
fallback; an explicit current-fixture path cannot silently fall back to them.
These are public fixture tokens, not live invitations.
This is image/parser interoperability only, not physical
camera scan, APK/device or LAN pairing acceptance. Evidence is recorded in
[progress-20261002-qr-interop.md](quality/stage2_android2unix/progress-20261002-qr-interop.md).

The retained 2026-10-04 false-center pixel regression showed ZXing 3.4.1's
single-symbol detector stopping at an interior finder candidate before the
real bottom-left finder. PairingCaptureActivity now installs a dedicated
PairingQrDecoderFactory after CaptureManager intent initialization and before
camera resume. It uses ZXing's existing QRCodeMultiReader geometric detector,
requires exactly one decoded QR with real result points, and refuses merged
structured-append output. It does not use PURE_BARCODE, rewrite the invitation
or relax expiry/fingerprint/safety-code confirmation. Multiple decodable
symbols keep scanning rather than selecting an arbitrary host. The Activity
is private and in-process; JourneyApps still owns permissions, camera release,
timeout, cancellation and result delivery.

The fixed original pixels and metadata are in test/resources/qr-regression.
JVM tests reproduce the old failure and cover rotations, off-center and
perspective scenes, ambiguous/blank frames and the actual JourneyApps decoder
factory with result-point callbacks. The permission-denial instrumentation
case additionally checks that the production scan launches this Activity with
the new factory; compilation does not prove device execution. Fresh APK and
physical camera/LAN validation remain mandatory.

## Model Read Admission

ModelReadPolicy is shared by model entry/refresh/catalog paging, ViewModel
reads and configuration preflight. Reads require connected non-scanning state,
no reset, pending write/revoke or model load, and a uniquely listed selected
project/session. Successful open current-scope catalog pages may continue;
selection revision or catalog snapshot changes invalidate continuation. Failed
reads can refresh but not reuse stale pages. Codec request/decode/merge paths
also refuse missing, foreign or duplicate selected projects.

Catalog display and paging are no longer nested under modelControl. Read-only
clients can inspect every catalog page without receiving a model/reasoning
write grant. Catalog choices remain non-selectable without control, and apply
controls retain current registry/selection checks. Running host sessions can
be inspected; read admission does not require an idle model owner. Scanner
state disables both reads and configuration preflight. Exact pending-write
resolution remains governed by WorkspaceDispatchPolicy, unchanged.

JVM tests cover read-only/running reads, scanner/reset/pending refusal, missing
and mixed duplicate scope, error refresh versus continuation and changed
selection/snapshot. The real Android/Rust model lane checks read admission
against actual host views. Existing Compose model cases cover read-only
catalog paging, scanner/reset/duplicate-scope disabled controls and refresh
recovery; source compilation is not device execution or LAN acceptance.

## Resource And File Scope

Resource catalog and file metadata policies now reuse codec-level readable
selection checks. Selected project ID must appear exactly once and its session
must match; mixed duplicate IDs cannot slip through by matching only one
session. File requests also require one actual active subtab in the selected
project. Both initial request and response decode refuse these invalid scopes
before network/key work. Resource paging preflight uses the same policy as UI
navigation; reload preflight now includes scanner/reset fences. Fresh refresh
remains available after a failed read.

A controlling client with an accepted/running message now has the independent
Browse host resources entry, matching the already available file metadata
entry. It can inspect resource descriptions and pages but cannot insert a
resource into the running message. Read-only browsing does not acquire prompt
or reload capability. JVM refusal coverage and actual Rust/Kotlin resource/file
lanes check ambiguous/unlisted project and absent active-tab cases. Existing
Compose resource paging coverage checks the running entry and scanner/reset
disabled controls; it is compiled only, not device-executed.

## Task Read Admission

WorkerStatusReadPolicy is shared by task-list entry/refresh/paging controls and
ViewModel dispatch. Reads require connected, non-scanning/non-resetting state,
no pending workspace write/revoke, no list load, and one uniquely listed
project matching the selected session. Continuation additionally requires the
open successful current-scope list; errors can refresh but cannot reuse stale
offsets. Read-only clients and running master sessions remain eligible.

WorkerReadPolicy applies the scanner and unique-project fence to completed
task output/history, including transient page-only task scopes retained after
closing the list. WorkerStatusPageCodec and WorkerHistoryCodec refuse missing,
foreign-session or mixed duplicate project selections before transport/key
work. Worker-history request construction is preflighted before clearing
dialogs or publishing loading state. UI summary/list task readers use these
same policies rather than weaker connected-only checks. Existing task digest,
subtab, owner/revision and retained-page checks are unchanged.

JVM coverage exercises read-only/running admission, scanner/reset/pending-write
refusal, page-only scope loss, error refresh versus continuation, and codec
refusal of ambiguous/unlisted scope. The existing task-list Compose case adds
scanner/reset/duplicate-scope and refresh-recovery assertions. Compilation is
not connected device or LAN acceptance.

The explicit AndroidRustWorkerReadTest lane now generates production Kotlin
task-list/history/output/detail requests, dispatches them through an actual
ProjectRuntimeHost/TuiState echo fixture and decodes fragmented Rust responses
with Android codecs and read/navigation policies. Eighteen real sequential
worker tasks produce independent worker journals while the master conversation
and desktop draft remain unchanged. The target old task is absent from the
bounded 16-task summary; task-list pages (16+2) provide its transient read scope
without publishing control capability or writing it into the display cache.

After local permission downgrade and fresh P256 authentication, the same
device is Read-only and can read its own completed task. Output and selected
user-message detail each span three 8192-byte-bounded pages; history contains
the actual truncated user/assistant previews. Full text length/digest and back
navigation are checked. Host restart restores journals without resubmission;
an old prompt is unauthorized after downgrade, not blindly replayed. Foreign
device task access, foreign session, stale digest and revocation refusals are
decoded as failures. Worker journal bytes remain unchanged across all reads.
This is offline real-owner/JVM interoperability, not TLS/LAN, physical camera,
APK execution, provider-network or connected lifecycle acceptance. Echo history
has two turns; a multi-page tool-loop worker history remains separate coverage.

An additional explicit unit-test interop lane configures one real worker Agent
with a deterministic local backend and the production read_file tool under
Read-only side-effect policy. Nine actual file calls form twenty durable turns
(user, nine assistant/tool pairs, final assistant), spanning history pages
16+4. Kotlin generates history and selected tool-argument requests and decodes
fragmented Rust responses using production codecs. Every tool result must
contain the fixture file content, and the fixture secret assignment is redacted
before crossing the wire. A valid path longer than the 512-byte argument
preview exercises truncated preview versus full argument reads. Foreign task
device, wrong tool-call ID and stale history digest are refused. Master and
worker journal bytes are unchanged by reads; Android navigation pauses while
scanning. This exercises the real WorktreeHost/Agent/tool chain, not the desktop
UI, actual provider service, phone camera or LAN transport. The lane uses an
isolated writable ZENPI_HOME and is explicitly admitted by the source checker.

## File Browser Paging

The host file picker now traverses complete bounded directory metadata in
64-entry snapshot-bound pages. Previous/next/page number controls preserve the
same project/session/subtab/revision and directory snapshot. Entries past the
first 64 are reachable; directory metadata changes refuse an old page. An
incomplete 2048-entry/deadline scan fails visibly instead of hiding a tail.
Failed pages clear stale entries; offline/loading/pending/active-prompt states
retain existing interaction fences. Selecting a file still only inserts the
quoted reference into the draft; prompt admission reopens and validates it.

Client requests/replies allow only adjacent directory-page offsets and refuse
entries repeated across the current and up to seven retained forward pages.
Retained pages must form one consecutive snapshot/directory/scope chain.
Attachment, parent-directory and page buttons check the file list against the
current project/session/active-tab/revision, and error/reset states fence them.
A late successful result for a replaced selection closes the picker instead
of leaving it loading. This bounded consistency check does not recompute the
entire directory digest or retain an unbounded directory listing.

Only an initial explicit invalid_request can fall back once to the legacy
three-field read. Other errors and page continuations never fall back. No file
content, workspace write, transcript event, ACK or cached authority is added.
Prepared headless device harness now puts its attachment after 80 filler files
and must page to select it. This harness and Compose pagination tests compile
only; current APK/device execution remains unverified.

## Private File Selection Boundary

Remote legacy/paged file lists reuse the host tool credential-path policy,
omitting credentials/auth JSON, known SSH private-key names, credential folders
and PEM/KEY/P12 files case-insensitively. Android refuses these paths in browsing,
decoded entries and quoted file insertion. Manually typed raw attachment
references are also refused by host admission before a journal/checkpoint or
runner effect; Android checks do not replace host authority. Historical durable
receipt retries still resolve without replaying an operation. Local desktop
discovery/attachment behavior remains unchanged. This is a known-path boundary,
not proof that arbitrary file contents or provider/tool instructions are safe.

## Remote New Session

Current host listings advertise a separate session_control capability. Android
offers a confirmed new-session action for the main subtab, using the existing
encrypted pending-write flow. Fresh requests require connected foreground and
current scope/owner/revision; the host refuses pending inputs/attachments and
active/worker-owned execution. The host uses its existing atomic NewSessionPlan,
with a durable reservation before replacement and a success receipt committed
with the new mapping. Old conversations are retained. Unknown reservations are
not automatically recreated; exact retries, including after restart, return
the original result. The TUI reconciles only an actual session change and
refreshes owner input/approval handles. Android fetches the new snapshot and
does not carry the old transcript cursor forward. Session control is absent
from offline cache. Historical catalog/resume support is described below;
rename/delete parity and current device acceptance remain incomplete.

## Historical Session Catalog

Browse sessions opens a Read-authorized, transient 16-entry page for the current
project/session. Previous/next pages bind the same catalog digest; a changed
directory/journal requires refresh rather than mixing snapshots. Current entries
cannot be resumed. A confirmed historical selection produces one encrypted
pending session_resume request with target ID and catalog digest, never a path.
The host uses canonical Agent resume and the atomic mapping/receipt transaction;
Android retrieves a new snapshot after confirmation. Scope/authority loss clears
the catalog; offline/loading/pending writes fence interaction. JVM validators
and a narrow-screen Compose test cover the source contract, not device execution.

The sessions dialog also searches the host's bounded catalog by projected name,
session ID or redacted user/assistant conversation text, case-insensitively. It sends query on every filtered page and
checks the exact echoed query; filtered/unfiltered history cannot mix. Query
is at most 128 UTF-8 bytes, trimmed and control-free, and the host rejects
redaction-sensitive text. Tool/system turns, event payloads and tool arguments
are excluded. Redaction precedes matching, so a hidden credential cannot produce
a search hit. Search entries include matches and up to four 240-code-point
snippets, never a complete historical journal. Android validates counts, types,
size and controls and renders bounded snippets. Unfiltered metadata is unchanged;
older query responses without snippets remain readable.
The current Rust/Kotlin session lane now reads actual case-insensitive desktop
conversation search results alongside every owner replacement and restart. It
checks the unchanged catalog digest, exact two historical snippets, refusal of
partially missing search fields and compatibility with legacy metadata-only
results. A separate name-only fixture retains explicit zero-match results.
These are source interoperability checks, not connected search UI acceptance.
The full catalog digest remains the resume authority, so the chosen result can
use the existing exact session_resume request. Search is Read-authorized and
never changes current session/journal/cursor. The admitted query survives read
failure for explicit refresh, is transient and is cleared on scope/authority
loss. UI shows empty results and fences search offline/loading/pending/reset.

Catalog reads are restricted to the adjacent next/previous offset. Empty
noninitial pages, adjacent duplicate session IDs and duplicates against the
seven retained forward-navigation pages are refused. A fetched backward page
must equal the retained preceding page, not merely echo its snapshot digest.
Switching pages clears the pending UI resume confirmation; rejected pages enter
the existing catalog-changed error state without replacing the current session
or producing a resume write. The client does not recompute the entire host
catalog digest and does not retain an unbounded catalog.

## Invitation Monotonic Expiry

Each parsed invitation captures elapsedRealtimeNanos and at most its initially
remaining 300-second wall-clock window. Freshness requires both wall expiry/window
and monotonic elapsed time; Android elapsed realtime includes device sleep.
Observed expiry or invalid clock transition permanently retires that parsed
invitation, so correcting/backdating the wall clock cannot revive it. The object
is transient, never serialized; process recreation requires parsing a new invite
under current checks. Host expiry/one-use admission remains authoritative.
The existing confirmation, storage-lock credential admission and pre-token
handshake checks all use this same freshness fence. Accepted host receipts are
not retroactively rejected merely because time advanced after token submission.
UI countdown takes the smaller remaining bound and ticks even when the wall
clock does not advance; code entry/confirmation stay disabled after expiry.
Countdown now validates and calculates from one monotonic observation under the
same invitation lock as requireFresh. It no longer reads the clock again after
freshness admission, so a boundary crossing or anomalous second observation
cannot return zero without retiring the invitation. JVM checks require exactly
one countdown clock read and permanent refusal after expiry/clock recovery.
Prepared Compose proof and unique scanner-verifier marker are not device evidence.

## Current Conversation History

Browse conversation reads the selected active branch through transcript_read
view=session_history, independently of the latest-64-turn snapshot/cache. The
host checks Read ACL, project/session and exact journal revision, then returns
at most sixteen redacted 2048-byte previews with tool summaries per page.
Digest-bound continuation refuses a changed journal/branch. The route is bounded
to 16384 durable turns, 64 MiB serialized selected history and a one-second scan opportunity;
resource exhaustion fails instead of returning an incomplete conversation.
Android retains at most seven previous pages, validates scope/revision/digest,
counts, distinct IDs and typed previews, and allows full message/tool-argument
reads through the existing transcript route. Historical previews are transient
read authority only and never replace the foreground snapshot or enter its
encrypted cache. Journal/scope/authority changes close history and any selected
historical message; navigation is fenced offline/loading/pending/scanning.
Connected tests are prepared but have not run on a current APK/device.

History reads borrow selected durable turns and stream JSON into a SHA-256 writer
with a hard byte/deadline budget instead of cloning/serializing the full message
set into another buffer. The digest remains byte-for-byte compatible with the
previous JSON-array digest. JSON escaping counts toward the serialized budget.
Android validates the complete retained page chain before navigating or lending
historical read scope; malformed/duplicate/noncontiguous pages are refused.
Closing a historical message returns to its exact valid page, including after
a transient message-read error. Changed journal/scope/tool identity discards it.
Historical detail requests carry that page's journal_revision on initial and
continuation reads, including tool arguments. The host checks it under the
owner lock and echoes it; Android requires the exact numeric echo. This fences
the race between history listing and opening details before subscription can
observe a desktop change. Legacy live transcript and worker reads are unchanged.

## Current Session Rename

The connected main subtab displays the host's optional session name and offers
a rename dialog. Validation checks 64 UTF-8 bytes, trimmed non-empty text and
no controls/path separators. The host additionally applies credential redaction
and duplicate-name checks; mobile validation never substitutes for authority.
The write binds the current journal revision plus existing owner/workspace CAS,
and is persisted through the encrypted pending command path. Unknown outcomes
retain the original request. A retry cannot override a later local rename.
Online workspace/catalog metadata carries safe names; legacy hosts may omit
them and the existing offline cache drops them. Compose source checks cover
confirmation, invalid input, changed session and offline fences; current APK/
connected-device behavior is not yet proved.

## Connected Session Workflow Gate

The explicit session device suite now includes SessionCatalogScreenTest along
with the real-host workflow, scope, new-session and rename tests: eight tests
are required, not the former seven. The desktop control suite also includes
WorkspaceFilesScreenTest, TerminalControlScreenTest, TerminalHistoryScreenTest,
WorkerScreenTest, WorkerStatusPageScreenTest, ResourceCatalogScreenTest and
ModelStatusScreenTest, DiffScreenTest and TranscriptToolScreenTest. It requires 80 tests, or 81 with Doze (29 base plus 51
control). Source-count regression tests keep the
explicit class sets and expected counts aligned. Lifecycle receipts record
the exact suite and count; reduced historical output cannot pass the current
matrix. These harness changes are tested locally, not executed on Android.

The shared instrumentation verifier now matches every current source
class/method to one AndroidJUnitRunner start/success pair. Exact total alone
cannot pass: duplicate, foreign, skipped, failed, missing or unfinished tests
and inconsistent indexes are refused. Desktop receipts retain the parsed
completed identities; the lifecycle validator reparses output and compares
them. Desktop/session runs bind the verifier file hash. This checks evidence
completeness, not cryptographic authenticity or device/LAN execution itself.

SessionHostTest now exercises production PairingViewModel/Compose buttons for
confirmed rename, new empty session and historical resume of the seeded provider
conversation. It checks saved pending requests resolve, transcript/event cursor
survive rename, workspace revision advances once per replacement, and offline
cache has no control authority. The corresponding harness independently decodes
the real desktop terminal QR, grants the fixture device locally, checks desktop
name reconciliation and validates old/new journals plus atomic mapping receipts.

The workflow also asks the real desktop PTY to execute local `/new` after
Android restored the original conversation. No mobile refresh or write button
is called in this phase. The test waits for automatic foreground snapshot
replacement, verifies the distinct empty session/cursor zero and updated
encrypted display, and captures a second screenshot. The harness separately
requires a local session_new_transition, one workspace revision advance, final
mapping to that new journal, unchanged remote receipts/bindings and unchanged
original journal. This checks desktop-origin sync rather than assuming the
mobile-origin path proves it. It is compiled/prepared, not device-executed.

Local TUI session open/resume-last and local headless slash/typed resume now
refresh the owner pool's input/approval handles after successful replacement,
as remote resume already does. Old input-port clones retain their old identity;
failed checkpoint/resume never publishes replacement handles.

After a source-bound APK build and current host build, run from the repo root:

```sh
python3 tools/stage2_session_android_smoke.py --serial emulator-5580 --expected-api 35 --receipt /private/tmp/session-workflow.json
# Direct private host interface, no adb reverse:
python3 tools/stage2_session_android_smoke.py --serial DEVICE_SERIAL --expected-api 35 --lan-ip 192.168.50.14 --receipt /private/tmp/session-workflow-lan.json
```

This is a prepared connected-device gate, not a current pass. It refuses stale
APK provenance before adb. Terminal QR image decoding is not physical camera
scanning, and an emulator's direct-interface path is not physical-phone LAN
acceptance. Receipts retain explicit false values for those broader claims.

## Desktop-Initiated Session Switch

A desktop/other-client session replacement now ends the old authenticated
subscription with session_changed rather than generic owner_mismatch. Android
recognizes only a correlated read-only subscribe refusal. Foreground/read-only
state discards its old in-memory cursor/transient dialogs, freshly authenticates
the same project and retrieves its current snapshot. Persisted old display is
retained until the new snapshot commits. Unknown writes/revokes, reset/offline/
background and foreign scope prohibit this automatic transition. Revocation
and true owner mismatch still require explicit authority reconciliation.
JVM and fragmented authenticated TLS tests exercise the boundary; physical
desktop-to-phone session-switch behavior remains a connected-device gate.

The prepared SessionHostTest workflow now requests a real desktop `/new`
after Android rename/new/resume and waits without a mobile refresh call. Only
the initial post-grant empty projection permits a fixture refresh. Each phase
requires matching encoded encrypted-cache display, session and cursor before
advancing. Workspace revision is checked on the live projection and host
checkpoint, not the authority-stripped cache. The desktop harness independently verifies the
local transition journal, unchanged original journal and remote receipts,
one revision increment and a distinct empty target. It captures the restored
and automatically switched views separately; compiling this test is not a
device pass.

The session harness selects SessionHostTest, SessionScopeScreenTest,
SessionNewScreenTest, SessionRenameScreenTest and SessionCatalogScreenTest and requires exactly eight
successful instrumented tests plus all nine ordered host-workflow phases.
The current workflow extends the earlier six phases: after cancel/reattach,
the Android client backgrounds, the local production grant command replaces
the project ACL with a fixture-only absent project, and a separately authenticated
read confirms the original project is unavailable. Foreground attach must
remove old controls while retaining encrypted display and pairing. Restoring
the original local grant does not silently change that held view; the production
Refresh host button must reacquire it. The host harness binds three device
records with unchanged identity/capabilities, consecutive revisions and exact
project removal/restoration, plus byte-identical owner checkpoint/current
journal during the permission-only change. The saved command evidence includes
those public registry records. This is a prepared test, not a current device pass.
Before requesting desktop `/new`, the production test leaves a rename dialog
open with an unsent draft and requires it to disappear after automatic sync.
It then cancels the foreground connection, checks retained pairing/display
without control authority, explicitly refreshes and requires authenticated
reattach to the same current session. This adds no new host mutation.
Message/task/terminal drafts and pending session/approval confirmations are
keyed to pinned host/device and project/session scope. Workspace and subtab
confirmations also reset on scope changes; subtab confirmations reset on
active-tab/revision changes. Reconnects and ordinary same-session event updates
retain drafts. Switching away and back does not resurrect an old confirmation.
Open mutation confirmations disable while offline, pending/revoking/resetting
or without the applicable host capability; backend validation remains required.

An automatic foreground read can also race a desktop switch after listing
but before its ACK, subtab listing, resume or initial inspect. A correlated
owner_mismatch on those reads triggers exactly one new authenticated
workspace_list on the same connection. Only a listed authorized project with
a changed session becomes a scoped session-change recovery signal. The failed
read is not resent and none of its frames are published. Missing/unchanged
projects keep the original refusal; revoked/malformed probe responses fail
closed. Explicit page reads, background work and mutation reconciliation do
not use this probe. Recovery still applies the foreground/pending/reset fences
and freshly authenticates before acquiring the new snapshot.

Cancel stops the owned foreground connection and clears transient readers,
but keeps the paired endpoint/device and bounded durable display in memory.
Offline controls are stripped using the same strict cache codec; unknown
writes and pending revoke IDs remain unresolved and are not resent. Explicit
refresh can reauthenticate without a new scan or Activity recreation. Cancel
does not wipe encrypted records or keys; Forget remains the separate local
reset operation. Cancelling an unpaired invitation returns to the empty state,
while cancelling a replacement invitation restores its prior saved pairing.

Activity background and every observed network-route replacement likewise
strip in-memory projection authority through the strict offline cache codec,
after cancelling owned readers/connection generation. Durable transcript,
cursor, tab metadata and unresolved write/revoke identity remain. In
particular, a failed authentication after returning to foreground cannot make
old approval/cancel/steering controls reappear merely by changing Offline to
Error. Only a verified current projection restores live capabilities. This
in-memory fence does not itself rewrite the encrypted cache or resend a write.

Local TUI and headless session open/resume also republish the owner's input
and approval handles after durable mapping commit and Agent replacement.
Regression tests cover both headless entry points, current-session input
queue routing, old-session refusal and answering an actual approval without
executing its denied shell command.

## Ordinary Tool Argument Reader

Ordinary transcript tool previews now open the same transient argument reader
as completed worker history. Requests bind current project/session/turn/call;
responses must match call ID and name, assistant role, digest and contiguous
UTF-8 byte offsets. Arguments are structured-redacted on the host, limited to
64 KiB and paged in at most 8192 bytes. No tool execution or metadata is exposed.
Next preserves the selected call; recent previous pages remain call-bound.
Changed call/turn projections invalidate pending reads. Existing foreground,
network, authority and pending-write fences apply, and page text stays out of
the offline cache. JVM and authenticated in-memory TLS tests cover this path;
the Compose interaction test is compiled, not device-executed evidence.
New v4 cache writes keep only a strict assistant-turn omission Boolean when tool
summaries were removed. A restored projection carrying that marker requests a
fresh authenticated snapshot rather than only replaying events after its saved
cursor. This recovers the previous messages' invocation controls without
persisting their arguments. Plain v4 cached transcripts keep incremental replay;
the marker survives display-only transitions and disappears after fresh host
projection. Since legacy v1/v2/v3 caches cannot prove that their assistant
summaries were complete, they acquire the same local recovery marker on decode.
Their text/cursors and original encrypted bytes are not changed by reading.
The marker persists through v4 migration until an authenticated snapshot
replaces it. Legacy user-only or empty displays still use incremental replay.

When snapshot recovery returns the typed Busy refusal during execution, a
foreground read with advertised subscription support can retain its existing
same-project/session cursor and display, refresh authenticated listing/subtab
metadata and subscribe to transient updates. Missing/foreign cursors, mutation
requests and other refusals do not use this path. No cursor or transcript is
invented. Omission markers remain through transient updates and new event-tail
messages, so a later idle read still requests the full snapshot rather than
pretending to have recovered old summaries. Background reads keep bounded retry
behavior. JVM framed busy-to-tail-to-snapshot tests are not device/TLS evidence.

Offline v4 prompt-status history has a combined 256-record budget across all
workspaces, prioritizing the selected workspace while preserving project order
and every retained message/cursor. The actual JSON-encoded byte budget is also
enforced by dropping lowest-priority status tails first; project ordering,
messages and matching cursors are never trimmed by this metadata rule.
Live listings are unchanged. This avoids a
listing and transcript that each pass the wire node cap producing an encrypted
record that cannot be decoded together. PairedHostStore validates its complete
plaintext envelope with FrameCodec before key creation/encryption/AtomicFile
write; unreadable scalar/complexity records leave prior ciphertext untouched.
JVM combined-view and prepared encrypted-store regressions cover the boundary;
the encrypted-store regression is compiled, not device-executed.

## Transcript Navigation Selection

Message-page navigation now shares exact selection validation across UI and
ViewModel: current project/session/message role, worker request/subtab and
selected tool call/name must match. Reset, pending requests, loading/error and
offline states fence previous/next. The entire at-most-seven-page history must
be valid, contiguous, continuation-bearing and digest/scope/total-consistent;
malformed intermediate pages cannot hide behind a valid last page. Each page's
UTF-8 byte size, offsets, hasMore and document cap are rechecked for navigation.
Retained pages now also validate bounded project/session/turn IDs, supported
roles, paired worker-request/subtab and tool-call/name metadata, assistant-only
tool arguments and canonical u64 historical journal revision. Worker pages
cannot carry master historical revision. Even a whole contiguous chain with
identically malformed metadata cannot authorize navigation or continuation.
TranscriptNavigationTest and current Rust historical detail page mutations
cover refusal; valid ordinary/worker/tool/history routes remain distinct.
Previous navigation rejects a broken chain without throwing or replacing text.
Ordinary and worker routes remain separate, with no effect or cursor/cache
change. JVM/compiled Compose regressions cover the contract; actual device
execution is still pending. TranscriptToolScreenTest is now mandatory in the
desktop control suite (80 tests, or 81 with Doze).

Task-history navigation and message/tool expansion also share a live-selection
gate. It validates the succeeded task, project/session/subtab and every retained
history page, including bounded turn counts, digest, offsets, continuation flags
and duplicate turn IDs. Broken earlier pages, reset, stale selection and pending
requests disable controls and refuse ViewModel actions. Returning from a message
also verifies the retained history before restoring it. These checks do not
grant execution authority or persist transcript content.

## Physical Camera Acceptance

Cancelling first-time pairing now reconciles encrypted storage as well as
replacement pairing. If a credential save already owns the storage lock, cancel
does not falsely publish an empty installation: it waits for that record and
restores a display-only offline host. New scans and refreshes are fenced during
reconciliation; incoming deep links are queued until storage is resolved.
Missing records restore an empty state, and unreadable records require explicit
local reset. PairingCancelTest exercises a held commit without fabricating a
network success. Its named completion is required by the desktop/lifecycle
evidence gates, but the regression has only been source-compiled here.

Backgrounding or a route change during pairing uses the same storage
reconciliation barrier. A saved record restores an offline host and may
authenticate again only when foreground/network conditions permit. With no
record, the interrupted invitation remains an unknown-outcome error; its
one-use pairing request is never automatically retried. PairingCancelTest
covers both absent and completed saves for each interruption, and the lifecycle
gate requires distinct background/network recovery phases.
Explicit cancel while that storage read is pending suppresses its automatic
reconnect. A completed save still restores offline credentials; an absent save
returns empty rather than reviving the interrupted invitation. Foreground and
LAN restoration while storage is held cannot override that stop. The existing
background/network regression methods exercise both outcomes before publishing
their recovery phase, so the suite count is unchanged.

External invitation/deep-link admission now also refuses a replacement while
the first pairing is CONNECTING with no published host. This matches the scanner
fence: the old host may already have registered the device or owned a synchronous
credential save, so cancellation by a new URI cannot safely imply no prior
effect. The existing PairingCancelTest held-save cases check that a different
valid URI leaves both the connecting state and save-owner Job unchanged before
explicit cancel/background/network interruption reconciles storage. This is
compiled device regression coverage, not a current connected deep-link run.

stage2_physical_qr_android_smoke.py is a separate operator-assisted device path,
not part of automatic emulator suites. It requires a current attributed app/test
APK and host binary, explicit physical device/API, private LAN address, empty
adb reverse table and a visible 140x100 terminal. The fixture displays the real
desktop QR; Android receives only public expected endpoint/pin/project args.
The operator points the physical camera and manually enters the SAS. The test
refuses either pre-existing Zenpi package before installation, as well as
pre-existing production pairing/key state. Named
camera/confirmation/registration/transcript/desktop-update/prompt-answer phases and a full JUnit start/success
pair are required. After the Read-only transcript gate, the host grants Control
for the same temporary project. The Desktop TUI first submits a local message;
the already connected phone must render its answer without an explicit refresh,
advance its event cursor and retain that answer in encrypted storage in the same
session. Next the test stops the Activity and cancels its scheduled WorkManager
opportunity to isolate foreground replay. Desktop submits another local message
while the phone's saved display/cursor remain unchanged and live authority is
absent. Returning the Activity to RESUMED must reauthenticate and display that
answer without Refresh or cursor injection, in the same session exactly once,
with a strictly advanced cursor and matching encrypted display record.
Android then submits a message through its production
composer and must render the desktop provider fixture's answer in the same session.
The host verifies read-only journal preservation and source
provenance at both ends. Only this fixture's new phone pairing/key are removed
through the production ViewModel reset-and-join path, not direct store deletion.
The v5 receipt's --verify-receipt mode rechecks saved output/test phases, cleanup,
exact public-only argv, device/API, Read then Control grant transitions, preserved
original journal prefix, six ordered local/background/remote user/assistant turns,
exactly three ordered provider requests/answers, same-session advancing u64
foreground replay observations, one matching remote durable admission
and running/succeeded receipts,
current build attribution and harness hashes without touching adb. Old v1/v2/v3/v4
receipts do not satisfy this stronger gate.
The independently decoded production QR must also pass the existing host-smoke
invite-envelope validator: exact version/fields, usable endpoint, canonical one-use token and
fresh bounded expiry, then the selected LAN address and certificate pin. Receipt
attribution covers the transitive terminal/rendering and build-verifier helpers.
Existing receipt/output files or dangling links are refused for every filename
suffix; operator evidence is not overwritten.
If instrumentation observation times out or a host callback/tick fails, the
shared runner first kills/reaps its owned adb subprocess and joins its output
readers, then reports a typed failure carrying partial stdout/stderr. The
physical harness preserves observed phase output in its unsuccessful record,
redacts invitation secrets and never promotes partial phases to success.
Raw callback exception text is not exposed. This does not imply the remote
Android instrumentation stopped with adb; the physical harness still performs
its existing force-stop cleanup and records cleanup failure separately.
This prepares physical scan, Desktop-local/live and background/replay updates
delivered to Android, and one Android-to-desktop prompt/answer LAN roundtrip,
not complete sync/API-matrix/release acceptance;
it has not run on a current APK/device. Automatic suites require 80/81 tests.

## Terminal Viewport And Keys

Terminal reads, input/resize and creation now use the same complete current
subtab resolver as worktree controls. Duplicated inactive tab IDs and oversized
lists cannot preserve terminal authority merely because the active ID appears
once. The Compose viewport uses that same scope guard; a valid read-only list
remains readable and an exact saved write remains retryable. JVM tests and the
current Rust PTY/Kotlin lane cover refusal and unchanged healthy paths; extended
Compose source compilation is not connected APK evidence.

Terminal reads, input/resize and creation reject mixed duplicate project IDs
and missing/duplicate active subtabs. The shared fresh-write dispatch boundary
also requires the addressed project ID exactly once with a matching session;
it cannot borrow capabilities from one duplicate entry. Exact persisted retry
envelopes remain resolvable after projection changes, but scanning/reset still
fences them. Terminal UI input, keys and creation are disabled during scanning.
Current JVM/real-PTY Kotlin-Rust checks cover these identities; prepared Compose
assertions are not physical-device/LAN evidence.

Selected subtab decoding also validates the listing's terminal projection:
project/session, active subtab ID and subtab revision must match exactly.
A contradictory listing/subtab pair is rejected before initial or subscription
publication, not merely hidden by the terminal UI. If the desktop changes tabs
between separate listing reads, the client treats that mixed view as unavailable
and reconnects rather than attaching old PTY output/control to the new tab.
JVM tests verify failed subscription updates retain the prior view and cursor.

The terminal icon opens the current desktop-owned shell through terminal_open,
only when the host advertises terminal_open_control. The request is an encrypted
pending CAS write bound to project/session/subtab, not a local Android shell.
Unknown outcomes retain the exact request; successful receipts verify a returned
host terminal ID before refreshing the display. The capability is never cached.
Old hosts lacking this capability do not show creation controls. Current-source
APK and production connected execution remain required acceptance gates.

Exited host shells retain visible output/history with running=false. Android
hides input/resize, displays the exit state and offers explicit creation when
the current host grants it. New input envelopes reject exited instances even
if a stale capability flag remains true. Pending exact-request reconciliation
is preserved; it does not target the replacement shell. The production device
workflow now exits/reopens and checks distinct host instance IDs and file effects;
it remains compiled, not connected-executed evidence.

The connected desktop terminal now preserves host line layout with no soft
wrapping and a horizontally scrollable monospace viewport. Scroll position
survives same-instance output refresh and resets on host/session/subtab revision
or terminal instance replacement. No ANSI stream or local terminal emulator is
introduced. Exact project/session/subtab project matching fences the viewport.

A keyboard-icon dropdown adds Backspace, Delete, Home/End, Page Up/Down,
Insert, EOF, suspend, line-start/end and redraw keys. Fixed bytes match the
desktop's existing terminal mappings and use the same durable instance-bound
terminal_input path; no new operation or permission is added. Pending writes,
revocation/reset/offline and lost Control close or disable the menu. EOF/suspend
affect the existing host program or shell; input acceptance is not execution
success. JVM construction/fencing checks pass; narrow-screen layout/menu tests
are source-compiled only, not connected device evidence.

## Scanner Invocation Evidence

Background resume passes its worker context and captured device identity into
the encrypted-store commit. Both are checked under the shared monitor after
lock acquisition and immediately before AtomicFile startWrite, after encoding
and encryption. A worker cancelled while waiting for foreground storage cannot
later overwrite display or advance the saved cursor. ResumeStoreTest observes
the actual IO thread blocked on that monitor, cancels its coroutine, releases
ownership and checks unchanged encrypted bytes/projection; it also cancels at
the last pre-write check and verifies a later live commit succeeds. Desktop
instrumentation includes the new method (33 base tests, 85 with workspace and
Doze), and lifecycle receipts require its exact named phase and gate. This is
compiled device-test preparation, not connected WorkManager acceptance.

The scanner evidence harness now records two separate named instrumentation
invocations: ScannerActivityTest (one permission-denial test), followed by
ScannerResultTest/PairingScreenTest/DiffScreenTest/DeviceIdentityAdmissionTest/
PairingActivityIntentTest (25 state/UI/entry tests).
stage2-scanner-evidence/v2 requires every expected class/method start/success
pair, exact suite/index/count and completed identities, per-invocation output
hashes, combined saved output, unique permission/cold-restore markers and the
current shared instrumentation-verifier digest. Summary-only, skipped,
duplicate, foreign, reduced-suite and old receipts fail closed. The harness
revalidates its saved successful receipt; cleanup failure is recorded as failure
rather than losing the receipt. Permission-denial evidence never claims actual
physical QR scan or LAN communication. New device execution remains pending
fresh APK provenance.

MainActivity is a singleTask entry with documentLaunchMode=never. Repeated
external pairing links reuse onNewIntent and the same ViewModel rather than
creating a second credential owner. onNewIntent updates the Activity intent;
the existing model still refuses a new invitation during connecting registration.
The actual Activity regression sends an ordinary link without task-reuse flags
and a document/multiple-task link, checks one retained Activity/ViewModel and
unchanged connecting state, then permits a confirming replacement. Scanner
receipts require external_pair_intent_single_owner_verified plus its exact
named test. Source and packaged release inspection reject standard/singleTop,
document duplication, reparenting and task/process overrides. The Gradle
preBuild invokes the source entry policy, and APK attribution inventories that
validator. Compilation/policy tests are not connected task-manager execution.

## Session Read Identity

Session catalog IDs use the same opaque host identifier contract: nonblank,
at most 128 UTF-8 bytes and no control characters. Catalog admission, retained
page validation, saved resume requests and new-session success receipts retain
Unicode, punctuation and whitespace exactly. The real Rust journal/Kotlin
session lane restores history:session/\u4e2d, resumes its actual owner and checks
the durable receipt across restart; this is not connected-device evidence.

Conversation history turn IDs follow the host's opaque identifier contract:
nonblank, at most 128 UTF-8 bytes and no control characters. Unicode and
punctuation are retained exactly rather than restricted to ASCII or rewritten.
The same predicate validates incoming and retained history pages. A current
Rust journal/Kotlin lane reads history:0/\u4e2d beyond the live tail and pages its
full redacted message with the exact ID; source framing is not device/LAN proof.

Session catalogs, conversation history and transcript details require the
selected project ID to occur exactly once in the current host projection and
to name the selected session. Mixed duplicate IDs with different sessions
cannot authorize initial reads, continuations, retained messages or session
new/rename/resume commands. Session-change recovery is also suspended during
QR scanning or while the selected project scope is ambiguous. These checks
do not grant write capability to read-only clients. JVM and actual Rust/Kotlin
owner/journal lanes cover this boundary; Compose assertions are prepared and
compiled, not connected-device evidence.

## Diff Pages

Diff scope admission requires the selected project ID exactly once with the
selected session and one actual active subtab. Paged navigation validates every
retained page's UTF-8 byte count, offsets, digest, total size, path and metadata,
not just pairwise identity/contiguity; closed dialogs and corrupt history cannot
issue continuations. Asynchronous results revalidate current advertised scope
and capability before publication. Failed reads may still refresh from the
current selection, and legacy unpaged responses retain their bounded read-only
compatibility. These source/JVM checks do not replace APK/device/LAN acceptance.

Android diff reads now request 8192-byte UTF-8 pages of a bounded 1 MiB host
document. Previous/next icons retain seven preceding pages and bind digest,
total/path/scope/flags/status. Loading/error/offline/pending/reset or mismatched
selection disables navigation; refresh starts again after changed host output.
A legacy host invalid_request on the initial read falls back once to its 64 KiB
summary, with no next-page affordance. Continuations never fall back. Pages stay
transient and do not grant cache/write authority. Truncated still means some
data was omitted by document/status/untracked bounds, not a complete patch.
JVM/real-Git fixtures cover reconstruction and refusal; current APK/device
navigation and patch apply/revert remain unverified or incomplete.

The desktop --with-worktree-workflow gate now prepares a tracked UTF-8 patch
beyond 64 KiB. WorkspaceModelTest drives the production ViewModel and page
buttons, checks backward/forward identity and exact full patch SHA-256/byte
count, then verifies changed desktop content refuses an old continuation and
refresh obtains a different digest. The host harness checks tab/checkpoint/
journal immutability during these reads and restores only its own fixture
file before the existing dirty-worktree removal test. Ordered phase and
content evidence is mandatory in the API 31/34/35 verifier; a success boolean
or short-diff receipt alone cannot satisfy it. This is prepared source, not a
connected-device execution claim.
