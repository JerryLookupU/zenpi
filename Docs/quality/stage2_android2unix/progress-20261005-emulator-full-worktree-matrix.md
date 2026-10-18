# Full Emulator Network Workflow Matrix

Previous goal turn: progress. It repaired the Rust release budget, completed
default Rust and native listener gates, and passed the current API 35 non-TUI
workflow. This continuation keeps the original Blueprint goal and all 36
unfinished item marks. It preserves dirty/user changes and uses no child agent.

## Explicit Emulator Transport

Desktop and non-TUI harnesses now support `--emulator-host-network`, mutually
exclusive with `--lan-ip`. This reuses the already proven camera workflow's
10.0.2.2 host alias and owned-emulator Wi-Fi preparation, not a new application
route policy. The helper has moved into the shared pairing harness so callers
do not depend on a circular camera/desktop import. The existing known debug
association permission fallback remains limited to an observed emulator.

The new mode requires an empty ADB reverse table, confirms `ro.kernel.qemu=1`,
binds the fixture host on a wildcard ephemeral port and advertises only
10.0.2.2. The application still uses its production Wi-Fi-matching nondefault
route binder, literal-address checks, Keystore and pinned TLS. No injected
route, cellular/default-route fallback or physical-LAN credit. Receipts retain
route, endpoint, Wi-Fi preparation and false physical/camera booleans. Desktop
and non-TUI receipts now bind the shared helper sources and reject runtime
dependency drift. Empty reverse/nonemulator/Wi-Fi preparation negatives pass.

The lifecycle validator has an explicit host-alias transport mode, without
changing its default direct-interface gate. Both modes still require all 93
desktop tests, worktree create/execute/dirty-refusal/remove, large diff page
digest/change refusal, Doze, network/WorkManager, real process/revocation
recovery, complete non-TUI provider/template/file/model/ACK effects and identical
artifacts across API 31/34/35. Host alias cannot certify direct interface or
physical camera/LAN. The validator also now requires the exact authenticated
tool-rehydration recovery branch with canonical advancing cursors. Missing,
wrong-route, reverse, forged physical/camera flags and reduced workflow evidence
are refused. All 28 lifecycle verifier tests and 186 Stage 2 tool tests pass.

## Actual Full Workflow Findings

The first API 35 full worktree/no-reverse attempt fails: network callback
observation remains stale, a diff navigation button is disabled and the initial
300-second observation expires. Its receipt stays failed.

The instrumentation network token crosses Android main and instrumentation
threads. It now uses an AtomicReference instead of an unsynchronized captured
variable, and assertion diagnostics include actual ConnectivityManager network
inventory. It still requires true route loss, offline display-only cache,
unchanged cursor, no authority and automatic same-host foreground recovery.
The diff fixture now waits for the requested offset and actual navigation policy
before asserting displayed/enabled buttons; it reports phase/page/history on
failure. No production callback, route or diff policy changed.

The next API 35 observation proves network loss/resume and the complete worktree
sequence, including the exact large patch digest, changed-document continuation
refusal, dirty remove refusal and successful cleanup. It nevertheless reaches
the 300-second observer limit before all 93 tests and post-death gates finish.
The receipt remains failed, not promoted from successful partial markers. Its
later disconnect is after the harness tears down its host at that deadline;
this is not a product invitation extension or an accepted complete run.

The initial instrumentation observer can now be explicitly bounded at up to
600 seconds. The production invite TTL still caps at 300 seconds; desktop
listener ownership still ends with the desktop. Individual test, TLS, frame,
connection, replay and unknown-write deadlines are unchanged. The full 600-second
run retains every test and workflow, rather than filtering for a smaller pass.
Its actual result follows after the live process is observed terminal.

Current artifacts: app
`780f374c3967704173981550e72c34e67136ddc6c975689b68cebd35244706cb`,
instrumentation APK
`b6a064d2d042607ec90f82eb402a42de1afcec71ea2720a9f652b9b71939ef3c`,
debug host
`e94ce4b0c77e182932cbe425489c2a6e148f8d6ec8424cf6621a3af67c5ec1e5`.
Fresh attribution checks precede execution. Earlier receipts remain historical
evidence of their own recorded artifacts, not current-matrix acceptance.

Physical-phone QR/manual safety code/direct-LAN, Android signed install/upgrade,
fat-LTO reproducibility and all per-item Master acceptance remain required.

## Real Network Observation Finding

The 600-second attempt completes all 93 tests with exactly one failure:
NetworkLifecycleTest. Its diagnostic observes an empty ConnectivityManager
inventory but the published LAN token still equals 2, even after the token
became atomic. This contradicts a mere cross-thread visibility explanation:
the monitor can miss the callback needed to re-read inventory. That run
preserves the failure; all worktree/large-diff/dirty-removal phases pass but
post-death acceptance does not run and is not credited.

NetworkMonitor now reconciles its complete local inventory once per second
while its foreground-owned cycle is active. Unchanged snapshots publish no
new token or reconnect. Lost LAN inventory still publishes offline, never a
cellular/default-route substitute. stop() removes the runnable and increments
the same generation fence used by callbacks; failed observation retires the
cycle before publishing offline. This is local metadata observation, not a
background socket keepalive or WorkManager guarantee.

The JVM regression checks 100 unchanged observations before and after an
observed loss, no token churn, and a distinct recovery token. The connected
network case additionally stops the monitor, disables actual networks again,
and requires unchanged token/state for two seconds before reporting the exact
stopped_network_observation_fenced_verified phase. The broad harness and both
matrix transport modes now require that phase, not just a boolean. A new
actual full run is observed separately.

The fresh application APK is
`871ddd9ffe49c2d55d8c899eb4658f3547b0628785825f0aadb7694d9747065c`;
instrumentation APK
`7eec8f0c690d4998b655d7e8896f279815bf1d4c004f50916a2d8aa4c776c412`.
Earlier artifacts and receipts above remain historical, not relabeled.

## Actual API 35 Full Pass

`desktop-api35-worktree-hostalias-reconciled-20261005.json` actually passes,
exit zero. It completes all 93 named initial tests with no failure/ignore,
then the real cache/process-death/pending-envelope/revocation follow-ups.
The independent retained check requires every desktop gate except the correctly
false physical-interface gate, 17 provider requests, all three large-diff
phases and digest/bytes, metadata-rehydration branch with advancing cursors,
four output hashes, current host/APK attribution, helper dependency hashes and
PNG hash. The screenshot is readable on the retained 320x640 API 35 GUI.

This includes actual Git worktree creation, selected-worktree tool/provider
execution, tracked diff plus more than eight bounded UTF-8 pages, exact full
patch hash, refusal after a desktop edit, dirty-removal refusal preserving
files/tab, successful removal after fixture cleanup, network loss/resume,
stopped-monitor fencing, WorkManager, Doze, real pending-write death/retry,
duplicate idempotency and a second death after revocation before credential wipe.
It uses the unchanged production route/TLS/authority paths through emulator
10.0.2.2 with no forwarding. Physical LAN and camera remain false.

The actual rebuilt debug/release Gradle unit tasks also pass. Each report now
contains 611 collected tests, 591 passed, 20 skipped, zero failure/error. The
one new regression is repeated inventory reconciliation/token stability.
Current API 35 non-TUI host-alias execution is observed separately; API 31/34
and aggregate same-artifact matrix are not yet credited.

The API 35 non-TUI run now actually passes too:
`headless-api35-hostalias-reconciled-20261005.json`, exit zero. Independent
retained verification checks all non-TUI gates except the correctly false
physical-interface gate, two provider calls, current identical app/test/debug
host provenance, artifact/shared-helper hashes, saved original request identity,
provider/journal/ACK evidence and all six PNGs. The file and resource page
screenshots are readable. Running preview, background read/write fencing,
reattach, three concurrent authenticated reads, model/reasoning selection,
attachment bytes, literal template expansion and actual pending-request process
death/recovery pass without ADB reverse. This closes only API 35's two lanes;
no API 31/34 or whole-matrix inference is made.

The fresh source checker completes, exit zero, after the production network
repair: Kotlin main/unit/instrumentation compilation, 594 JVM tests and all
declared real Rust/Kotlin interop lanes pass. Raw log SHA-256:
`83f051c08ec33258854a2a88ee389c3e8ebfd19d3e7888d648164d4196ac05b1`.
Its Cargo fixture executions replace the canonical debug output, so the
subsequent host verify-only correctly refuses the changed executable. The
actual attribution builder is run again, restoring the exact API 35 artifact
`e94ce4b0c77e182932cbe425489c2a6e148f8d6ec8424cf6621a3af67c5ec1e5`
with unchanged source inventory; a new build receipt is emitted rather than
rewriting either connected receipt. Their historical build-receipt checksums
remain original; current binary/source identity matches both retained runs.
Current APK verification, strict Blueprint and whitespace checks pass. All
test/validator processes are terminal; the API 35 GUI is retained with the
production app reopened and an empty reverse table.
