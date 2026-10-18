# Process Recovery, Release Budget And Headless Workflow

Previous goal turn: progress. It established the complete initial connected
suite while preserving a failed post-death assertion. This continuation uses
the actual local SDK/emulators, preserves the dirty checkout, keeps the full
Blueprint goal active and changes no checklist marks. No nested agent,
unrelated cleanup, commit or push.

## Supported Recovery Branches

`NetworkProcessTest` previously required incremental replay for every cache.
Tool-bearing encrypted display caches intentionally omit tool arguments;
`TranscriptRestorePolicy.replay` correctly requires a refreshed authenticated
snapshot instead. The test now covers both real supported paths, without
changing production security/replay policy:

- Complete cache: replay count must be positive, the recovered cursor advances
  and the exact cache prefix remains unchanged.
- Omitted tool metadata: incremental replay must remain refused with count
  zero; every omitted turn keeps its ID/role/content and regains nonempty tool
  metadata, and safe recaching preserves the entire original turn prefix.

Both branches still require authenticated same-session recovery, the desktop
answer produced while Android is stopped, explicit retry of the saved pending
envelope and duplicate idempotency. Foreground/network/cancel transitions cannot
silently flush unknown writes. The desktop verifier requires exactly one
matching branch marker and canonical advancing u64 seed/recovered cursors.
Its new negative tests cover missing/wrong/duplicate evidence, malformed,
overflowed, equal and regressed cursors. The Stage 2 tools suite passes 183
tests, also rerun after the later headless fixture edit.

`desktop-api35-process-rehydration-20261005.json` actually passes: 93 named
initial tests, 15 provider requests, workspace/directory/terminal controls,
prompt/steer/cancel and approval audit, network/WorkManager/Doze, actual process
death with pending retry and tool rehydration (cursor 28 to 30), a second death
after durable revocation and final named receipt resolution/credential wipe.
The killed processes intentionally report `Process crashed.`, not JUnit
success; their exact starts, live-PID phases and subsequent distinct recovery
processes are retained. The final cleanup test reports `OK (1 test)`.

`desktop-api35-plain-process-replay-20261005.json` separately passes 39 named
initial tests, two provider calls, real complete-cache event replay and the
same process-death/revocation chain. Retained identity/output/helper/PNG hashes
were independently rechecked. Both runs use ADB reverse, not physical LAN.
Earlier failed receipts are unchanged, not relabeled.

## Camera And Emulator Network

`emulator-camera-api34-process-rehydration-20261005.json` actually passes the
production terminal QR -> imagefile camera -> production scanner -> pinned
TLS workflow with bidirectional messages, background replay and controlled
Doze. Independent retained verification checks the named test, 14 phases,
three provider requests, six ordered durable appended turns, preserved journal
prefix, exact Android request binding, cursor 4-to-6 and helper/output/PNG hashes.
The screenshot is readable with no overlap. Transport is emulator host alias
10.0.2.2, with no ADB reverse; physical/manual camera and direct LAN flags stay
false. Its owned API 34 process is terminal and cleaned up; GUI API 35 remains.

These three receipts bind app
`780f374c3967704173981550e72c34e67136ddc6c975689b68cebd35244706cb`,
test APK `bbe5bd1e610bbd5135ad4cf8bbb974ce24fd165f37080a6c812df62e66f4ed22`
and their then-current debug host
`c93f6d6385f31c092b3abaf34a6e5f959d041ad9a1e671aacdf69add23da63bd`.
Later source/build changes make this historical attributed evidence, not
automatic current-matrix certification.

## Rust Gate Repair

The nonblocking OAuth test assumed a single pump immediately accepted and read
a newly connected client. It also treated an empty server connection list as
proof an oversized request had been handled, even before accept. The fixture
now waits within a bounded deadline for the exact partial bytes before forced
expiry, then reads the real oversized-request response through EOF and requires
HTTP 413 plus an unconsumed login flow. No production OAuth code changed.
All eight callback tests and the full 453-test library stage pass; pipe deadline
tests pass unchanged on this full run.

That first full `cargo test --locked` then correctly fails the existing release
budget: the retained release is 9,338,448 bytes, above 8 MiB. A separate fat-LTO
diagnostic build is 6,253,264 bytes and passes the unchanged size/startup/RSS
test. `Cargo.toml` now selects fat LTO, keeping all product capabilities and
the original budgets. The actual release attribution builder emits a canonical
6,253,264-byte binary with SHA-256
`03beda670d2f4568bd60257e0975dca8e1af126cf3fb085d95da9253eb3a8813`.
Its unchanged budget test passes too. The bounded production CLI smoke on the
actual Mac interface 192.168.50.14 passes; this proves host listener/invite,
not Android physical-LAN reachability. Full-test rerun status follows below.

## Actual Non-TUI Workflow

The first new non-TUI direct-interface attempt fails before pairing with
`Network unavailable`. Emulator LinkProperties contain nondefault routes only
for its virtual 10.0.2.0/24 network, not the Mac's 192.168.50.14. The production
LAN binder correctly refuses a default-route fallback. No route injection,
cellular fallback or emulator bypass was added, and the failed receipt remains.

A separately scoped ADB-reverse run reaches the real file picker but fails at
`performScrollTo` on the pinned dialog page button: that control has no scroll
ancestor because it is outside the scrollable file-list body. The headless
fixture now requires visible/enabled file/resource page buttons and performs
actual clicks without a nonexistent scroll action. Scrolling of list entries
and other genuinely scrollable controls remains. No callback shortcut or
production UI behavior changed. Fresh APK packaging succeeds afterward.

`headless-api35-release-fat-dialog-20261005.json` actually passes with the
current release binary, unchanged app APK and instrumentation APK
`425eff06d7957f2090c51610dd9afdc64c9e4615328fd1883b7b9a329a65ee2a`.
It verifies the running provider preview before completion, background read/
write fences and foreground reattach, three concurrent authenticated reads,
read-only Git diff, actual host-file pages/attachment, model/reasoning selection,
resource paging/literal template expansion, two provider calls, real Android
process death, encrypted original-envelope retry without provider replay,
durable ACK and listener expiry. The saved provider/journal/checkpoint evidence,
request identity, current artifacts/provenance and all six PNGs pass retained
verification. File/resource/stream/model screenshots were visually inspected.
This is ADB-reverse non-TUI evidence, not camera or physical LAN.

Remaining: complete current-artifact API 31/34/35 desktop/non-TUI/worktree matrix,
fresh camera matrix after test-APK changes, physical phone/manual QR/SAS/direct
LAN, fat-LTO reproducibility, signed Android install/upgrade, security review
and all per-item Master acceptance. All 36 Blueprint items remain unfinished.

## Full Rust And Native Listener Results

The first fat-LTO full run passes the repaired library and release-size gate,
but stops at three extension-fixture startup failures (1000 ms negotiation and
a missing running-child marker). A target-only recheck has a different CLI
disconnect failure. Both failed logs are preserved. No hook deadline, production
timeout or assertion was enlarged. The test CLI now retains bounded stderr and
exit status when its response channel fails, instead of hiding all stderr.
The next complete extension target passes 22 tests, one subprocess helper
explicitly ignored. This does not prove startup variance can no longer occur.

`cargo test --locked --no-fail-fast` then runs every default target and doc test
and exits zero, including the 453-test library, all 22 extension tests and
the unchanged release budget. The added flag does not filter tests. Existing
explicitly ignored native/Kotlin fixtures still require separate execution.
Full log: `/private/tmp/zenpi-stage2-full-fat-no-fail-fast-20261005.log`, SHA-256
`560b82372afde3bf9edc53d3f93078bbae87b3b7c8a39d6c0baf4c63daea7f8e`.

The separate native attempt exposes a fixture bug: its HTTP provider inherits
the nonblocking listener flag on macOS and `read_exact` fails with WouldBlock
before approval. That log stays failed. The fixture now explicitly makes the
accepted socket blocking while preserving two-second read/write deadlines and
bounded framing. Production listener policy is unchanged. The subsequent
`remote_frame`/`remote_host_e2e`/`remote_listener` run, including ignored tests,
passes 11 + 5 + 5 tests: actual production TLS, prompt/tool/approval/cancel/
replay/restart and connection limits/shutdown.

`cargo check --locked`, `cargo fmt --check`, all 183 Stage 2 tool tests, all
15 Blueprint validator tests, strict Blueprint validation and diff whitespace
validation pass. Strict validation still reports all 36 items unfinished.

The fresh cached-source checker also completes successfully:
`zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-stage2-source-recovery-20261005`.
It recompiles Kotlin main/unit/instrumentation sources, passes 593 JVM tests,
and executes every declared Rust/Kotlin interop lane against those new classes,
including pairing proofs, workspace/worktree/session/ACK/approval/PTY, paged
model/resource/file/diff/history/worker reads, production worker tool output,
prompt/cancel and isolated real-worker cancellation/receipt recovery. These
are real cross-language owner effects, not physical-device or external-provider
evidence. Source/compiler warnings remain; no warning-free claim is made.

The actual final Gradle `:app:test` invocation exits zero. Unit tasks are
up-to-date (only instrumentation changed); retained XML reports for both debug
and release each contain 610 collected, 590 passed, 20 skipped, zero failures/
errors. This is not a claim that skipped cross-language cases ran in Gradle;
their current execution is the separate source-checker evidence above.
Source-checker log SHA-256:
`99bbb71ffe5ae9c0a318164a5367df2a3c2d30107eee8e4267f4d76471aae425`.
Native listener log SHA-256:
`71d84671d9ff59f09a2011c92d6e4eb4fbbc4e70fad89feae909cb57927e105a`.
All invoked validator/test processes are terminal. Final Android/release-host
verify-only checks pass; ADB reverse is empty, no physical device is attached,
and the retained API 35 GUI has the production MainActivity reopened.
