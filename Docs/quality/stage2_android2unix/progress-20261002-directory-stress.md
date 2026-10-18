# Directory Selection Stress And Independent Receipts

The previous goal turn separated validated write confirmation from follow-up
read failure and emitted attributed passing evidence, while retaining an
intermittent directory-selection failure. This increment strengthens real
Android/desktop validation without claiming that unresolved cause is fixed.
The full Stage 2 objective stays active; all 36 Blueprint items stay unfinished.
No commit or push was made, and unrelated changes were preserved.

## Work

Two full unchanged-artifact diagnostic runs passed and did not reproduce the
failure: desktop-android-directory-diagnostic-20261002.json and its repeat.
The current diagnostic categories were therefore not evidence of a root cause.
The existing production Compose/ViewModel directory workflow now alternates
between the two actual directory tabs 12 times before closing either tab.
Each action goes through the real button, encrypted pending-envelope store,
pinned TLS adapter and desktop owner. The existing exact-envelope bounded
explicit retries are unchanged; no assertion or timeout was relaxed.

Initial stress and unchanged-artifact repeat runs both passed the full 23-test
instrumentation plus subsequent network/process-death/revoke/cache gates.
These are behavioral evidence, not guaranteed reliability or full API coverage.

The final harness additionally snapshots the desktop checkpoint at the earlier
directory-order phase, before stress, and independently verifies its later
state: exactly 12 new receipts, unique request IDs, authenticated device,
fingerprint shape, project/owner identity, alternating target IDs, contiguous
workspace/subtab revisions, unchanged tab data and correct final active tab.
This rejects a UI-only success or duplicate mutation counted as progress.
New Python negative cases reject wrong revisions/device/target, duplicate IDs
and wrong final selection. Phase capture is separated from the next mutation
to avoid attributing a following operation to the stress block.

## Checks

Instrumentation APK assembly passed. Fifteen Python harness/Blueprint tests,
Python compilation, strict Blueprint validation, controller validate-only and
whitespace checks passed. Validate-only launched no workers. Rust and Android
production code/app APK are unchanged this increment; their earlier unit/build
results are not represented as rerun.

The final independent-checkpoint run passed and retained
desktop-android-directory-ledger-20261002.json. Its
directory_selection_stress_verified=true now includes all 12 receipt/revision/
target checks, not just an Android phase marker. All 23 initial instrumented
tests and later actual process-death network/pending-write/revoke recovery,
WorkManager and independent cache/intent fixtures passed. The injected
post-write read failure, prompt identity binding, approval actor/preview,
Allow/Deny and listener teardown gates also passed. Ten desktop provider calls
were recorded, with direct 192.168.50.14 TLS and no adb reverse. Camera and
physical LAN peer flags remain false.

Attributed hashes:

- Rust: `23fe1bcc3cda9ed05ad3e3708f87aae7b036f4e2de77c7651ef9aebb3909af75`
- App APK: `505361d1ba69ff3c1ad8bc02895948dbdaeb4c0e0093fb9f8041683a8d855b2b`
- Test APK: `82ef185889232348d7c6fc57f7146cba2a0a3799d4b51989d218cd8deb6c5e8b`
- Harness: `cce83aa3265f49f03453f31eec9f84c2ad21ae17592e483a2c43ffd5749dffe3`

## Remaining Scope

The intermittent directory-selection failure still needs positive reproduction
and attribution; successive passing runs cannot prove its absence. Physical
camera/LAN peer, supported API matrix, selected-root execution, steering,
streaming/ACK, PTY, scheduler quotas, Git crash transactions, historical request
migration, killed-host/storage faults and release/controller acceptance remain.
No complete S2-034/S2-035/S2-038 acceptance follows from this bounded stress test.
