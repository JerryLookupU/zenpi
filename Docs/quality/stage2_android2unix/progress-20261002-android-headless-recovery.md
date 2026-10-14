# Android Headless Recovery And Refresh Progress

Partial S2-014/S2-019/S2-038/S2-053/S2-054 progress, not whole-item acceptance.
The prior goal turn implemented the non-TUI host runner and produced Rust/CLI
evidence. This continuation adds actual Android command/process-death evidence
and a production refresh behavior correction. All 36 Blueprint rows remain
unfinished; full goal remains active. No commit/push or nested/worker agents.

## Changes

HeadlessHostTest drives the existing production Compose composer against the
real non-TUI host. It pairs a non-exportable signing identity, verifies no
implicit project grant, then waits for a fixture-local Control grant. The
first prompt displays its real HTTP provider answer. A second exact envelope
is encrypted before sending, committed on the host, and deliberately left
locally pending with the old transcript cursor. The harness proves the app is
live, force-stops that PID, and starts a distinct instrumentation process.
Foreground resume does not auto-send the pending write. Explicit Retry same
request resolves it and replays the missing transcript without a third request.

The new bounded headless harness independently verifies the provider requests,
two unique checkpoint bindings/admissions, original device/project/session
identity, exact running/succeeded records and unique transcript results. It
checks listener expiry, artifact stability and empty adb reverse tables in
direct-interface mode. Receipts preserve failed runs and include evidence,
process and screenshot hashes. Six negative/positive verifier tests cover
duplicate request/admission/progress, wrong owner/session/fingerprint, missing
results and wrong latest user inputs.

Desktop regression exposed confirmed writes immediately flickering Connected
back to Connecting during the following pure read. That could disable a label
control at the instant of the next action. PairingViewModel now retains
Connected for pure reads of an already connected projection without unresolved
writes. First attach, unconfirmed commands, offline caches and error handling
remain fenced. It does not auto-send unresolved commands.

NetworkProcessTest now stops its owned foreground polling before independently
checking an already resolved command's explicit duplicate. This isolates that
check from a simultaneous read; it does not loosen the host ACL or silently
retry writes. Extra status markers identify the two recovery boundaries.

## Evidence And Failures

The pre-refresh headless flow passed twice on API 36 direct 192.168.50.14 TLS,
without adb reverse: headless-android-process-checkpoint-20261002.json and
headless-android-process-final-20261002.json. Exactly two real provider calls,
actual process death, saved-envelope resolution, transcript replay and listener
expiry passed. PNG inspection showed readable composer/transcript/status on
320x640. These immutable receipts are not relabeled as the later APK.

The first headless receipt failed because the harness looked for the legacy
TUI checkpoint instead of session-parent project-workspace.json. It is kept.
A later grant race failed during a read started immediately after the grant
signal; the instrumentation now retains its original ungranted projection and
performs bounded fresh reads inside the existing unauthorized-aware wait.

Desktop pre-refresh regression failed at directory-two with Connecting. After
the production correction, all 24 initial Android tests passed twice, including
12 directory selections and full Git worktree creation, distinct-root tool
execution, dirty refusal, clean removal and branch retention. Both full runs
then failed at a concurrent independent duplicate check with unauthorized;
the device registry remained unrevoked with Read/Control and correct projects.
This is not claimed to prove an authorization root cause. Earlier failures were
not promoted. The harness now preserves an early-failure receipt and sanitized
instrumentation/registry evidence instead of losing it to a later exception.

After fencing the independent duplicate check, the complete desktop flow passed:
desktop-android-connected-refresh-fenced-20261002.json. It includes independent
terminal QR decoding, direct TLS, 24 initial tests, 17 real HTTP provider calls,
worktree/directory workflows, steering/cancel/approval audit, WorkManager reads,
actual process death, pending-write recovery, revocation/death recovery and four
independent cache/revoke processes. Listener shutdown passed. Physical camera
and physical LAN peer flags remain false; live event push remains false.

Desktop accepted-run artifact hashes:

- Rust: bdd666957d8a7323617b955252894cdb925df868ed939baec86ccff37bd502e9
- App: dafdf2f1fc981f793c6e1c829f39a9e118403c1961e2f160fb943dc64d08dfaf
- Test APK: aab5babee436113bcfd0da0fdec31738c25ff22fe9387be5a16de83671d1d161
- Harness: a1a1d509c4b35e2fc93a551ee3c86308095e897351afb72245e7493f13695ddf

The final headless rerun with those exact Rust/app/test APK artifacts also
passed: headless-android-connected-refresh-final-20261002.json. Its two-provider
chain, distinct recovery process and listener expiry passed; the persisted
command evidence was independently revalidated. Headless harness hash is
92c946db98bac5194eb70bd4b2484b83118588b37dffc9f64fbace193e14e65a.

Debug and release unit suites reran after the production change: 45 tests each,
zero failures. Python desktop evidence tests passed 14, headless evidence six,
Blueprint tests five. Strict Blueprint, controller validate/status, format and
diff checks passed. Controller has no live claims/transports/requests/handoffs.
Fixture provider/listeners ended and no harness/host process remained. The
owned emulator was stopped with adb emu kill; its execution handle ended 134
while saving the graphics snapshot, not a clean exit 0. ADB and PID checks
confirmed it no longer runs; this shutdown anomaly does not relabel the passed
instrumentation receipts.

## Remaining Scope

Independent worktree owners and executable concurrency, complete stdio/network
parity, attachment/resource handling, PTY/diff/bulk, live push/ACK, broader
network/adversarial/crash coverage, API 31/34/35 lifecycle matrix, physical
scanner/LAN and release security/packaging gates remain unfinished. Ordinary
registry lock contention is still fail-closed and merits a distinct Busy/error
contract rather than weakening authorization. No completion inferred from this
focused end-to-end flow.
