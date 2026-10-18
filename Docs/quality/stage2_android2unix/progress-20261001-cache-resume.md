# Encrypted Cache And Process Resume Progress

Provisional Master implementation for S2-033/037/038/053/056, not complete
acceptance of these items or the original Android/LAN/QR/desktop objective.

## Implementation

The Android store now encrypts a bounded display and its matching event cursor
in one atomic no-backup AES-GCM record. Projection encoding reuses the strict
snapshot parser and validates its project/session ownership before writing.
Old metadata-only records remain readable and use a fresh snapshot. Missing,
malformed, oversized or tampered caches cannot advance the committed cursor.
Host changes are checked before projection commits; store instances share a
serialization lock. The ViewModel commits the record before publishing the new
projection, restores cached content offline, and resumes foreground reads.

The confirmation screen consumes its content insets and avoids the keyboard.
The numeric safety-code field has a Done action that dismisses the keyboard
and clears focus. Confirmation still requires human entry and enabled state;
the Compose test uses real click input after Done, not a direct callback.

## Behavioral Evidence

Receipt: `desktop-android-process-resume-20261001.json`. The harness uses the
production Rust desktop PTY and local HTTP provider, the real Android TLS
client and Keystore, and independently decoded terminal QR pixels. No raw QR
or invitation token is written to the receipt.

The initial Android process observes the second desktop provider answer by
ordered event replay, persists its display/cursor, and restores them in a new
ViewModel offline. A bit flip in the encrypted record is rejected. The test
then leaves its test host record for independent-process verification.

The harness starts a waiting instrumentation process, observes its exact PID
both from Android test status and `pidof`, force-stops it, and checks that no
app PID remains. In the passing run the killed live PID was 4723. While Android
is stopped, the same running desktop generates another provider answer through
keyboard input. A new process (4775) restores the offline cache and signs in
with the original device key. It must receive the missing turns through replay,
retain the session identity, increase its event cursor, and persist that cursor.
Snapshot substitution would fail the replay-count assertion. Local host revoke
then makes this new process receive explicit unauthorized.

Two more independent instrumentation processes verify fixture cache restore,
non-exportability of the device key and cleanup. Test records/signing keys are
wiped. The desktop exits and the real listener port is proved closed.

Android Debug and Release unit suites pass 16 tests each. The production
desktop sequence passes three initial instrumentation tests, one resume test,
and two one-test cache runs on API 36. The deliberate live-process death is
not represented as a passing JUnit test: the harness proves its PID and death.
Five Blueprint tooling tests and the strict 36-item validator pass.
The transcript screenshot was visually inspected at 320x640; text wraps and
the system/status insets do not overlap the content.

## Scope Still Required

adb reverse is emulator interoperability, not camera scanning or LAN routing.
Committed-transcript pull replay is not server push or provider/tool deltas.
Remote mutations, workspace/worktree CRUD, approvals/diff/PTY, durable write
deduplication, NetworkCallback/WorkManager, Doze and API 31/34/35 lifecycle
coverage, migration/backup/uninstall and release gates remain unfinished.
All 36 Blueprint items remain unchecked. No worker or cron is claimed live,
no commit/push is made, and the full original goal remains active.
