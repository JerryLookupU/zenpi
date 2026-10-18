# Bounded WorkManager Resume

## Implementation

S2-038 now has an opportunity-based WorkManager read path in addition to the
foreground network callback. Activity stop schedules one unique delayed task;
Activity start cancels it. A task checks saved host state, skips unresolved
workspace writes, loads only an existing device key, verifies its device ID,
checks default-route presence, and uses the existing production TLS client for
one bounded inspect/replay. No token, transcript or credential is placed in
WorkManager input. The production input identifies the metadata namespace and
Keystore alias only. It never executes Android-local tools or automatically
flushes an ambiguous command. At most three attempts are admitted with
exponential backoff; explicit unauthorized is terminal. No foreground worker,
FGS declaration/permission, permanent socket or periodic task is introduced.

Background cache commit uses a digest of the exact encrypted record as a CAS
stamp under the shared storage lock. Any foreground cache write, pending
command, re-pair or wipe invalidates that stamp. A same-session stale
foreground cache write cannot regress a newer cursor. Backup-only AtomicFile
recovery no longer gets mistaken for absent metadata. Missing-key background
lookup does not create a new signing identity.

## Validation And Repair

The first production WorkManager test timed out because the emulator's
partially connected default network did not satisfy the `CONNECTED` constraint,
even though production direct IP/TLS already worked. The failed receipt is
preserved as `desktop-android-workmanager-20261001.json`. The repair removes
that constraint and checks the default route inside each bounded attempt,
preserving offline-LAN support rather than demanding Internet validation.

`desktop-android-workmanager-final-20261001.json` passed with ten initial
instrumentation tests, one post-death restore test, and two independent cache
process tests. It verifies real WorkManager execution plus skipping pending
writes. The first stronger new-turn fixture encountered an expected host busy
response while the desktop answer was still running; its failed receipt is
preserved as `desktop-android-workmanager-new-turn-20261001.json`. Read-only
observation now tolerates only that explicit busy response until the bounded
deadline, without weakening worker or production host behavior.

The stronger check submits a new provider answer through the same desktop owner
after foreground models stop, proves the encrypted cache initially remains
unchanged, and requires the real task to advance it with that answer. Host
workspace revision must remain unchanged while a second task skips a pending
request. It uses the private host interface without adb reverse.
The final stronger run is recorded in
`desktop-android-workmanager-new-turn-final-20261001.json`: `passed=true`,
`workmanager_read_resume_verified=true`, ten initial tests, one post-death
restore test, and two cache-process tests passed. Three desktop provider
requests are recorded, including the additional background-read turn.

Debug/release JVM tests and debug/app-test packaging pass. Store instrumentation
tests cover stale commit, repeat stamp, cursor regression, pending requests,
re-pair, wipe, backup-only recovery, and missing identity. The merged manifest
excludes SystemForegroundService and FOREGROUND_SERVICE permission.
Debug and release JVM suites each contain 21 tests with zero failures/errors.
The four harness admission tests, five blueprint tooling tests, strict
blueprint validation, Python compilation and `git diff --check` passed.

## Remaining Scope

This is real in-process WorkManager execution against the production desktop,
not proof of JobScheduler dispatch after process death, Doze behavior or the
API 31/34/35 matrix. Full timeout/retry/cancellation/scheduling tests, safe
queue-flush semantics for future command families, worktree/session parity,
event push/ACK/backpressure and physical camera/LAN validation remain open.
All 36 authoritative blueprint entries remain unchecked; no full acceptance,
commit, push or goal completion is claimed.
