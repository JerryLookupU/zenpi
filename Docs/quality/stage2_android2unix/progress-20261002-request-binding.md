# Durable Remote Prompt Identity

The previous goal increment added real approval diff previews and passing
production evidence. This increment closes a supported-write identity gap;
the full Stage 2 objective remains active, with no complete Blueprint item
accepted and no commit or push.

## Change

The existing checkpoint receipts rejected approval/cancel/workspace IDs reused
as prompts, but prompt journal IDs were not visible to the other operations.
Prompt admission now durably binds device/request ID, full envelope fingerprint,
project and session in the shared owner checkpoint before journal reservation
and runner submission. It stores no prompt text, does not increment workspace
revision, and remains accessible while the Agent execution lock is held.

Workspace/subtab, cancellation and approval dispatch reject a bound prompt ID.
Prompt dispatch checks this namespace before taking the Agent lock, rejecting
changed envelopes and cross-project reuse deterministically. An exact bound
request without journal admission is unknown_outcome and never resubmitted.
Checkpoint CAS failure prevents runner submission. Bindings are device-scoped,
bounded to 128, non-evicting, and retained across project/session changes.

Restore validates binding identity, exact schema, duplicate IDs and shared
receipt collisions. Legacy current-session journal admissions are imported
atomically before remote mutations are enabled, including when no checkpoint
exists. An existing restore publication used its stale in-memory checkpoint
after migration; restore now adopts the prepared state without publishing stale
receipts back into the checkpoint. Import failures leave the error visible.
Historical pre-upgrade journals outside the saved session mapping remain an
explicit migration limitation, not a claimed unlimited global ledger.

## Tests

Two new integration cases cover prompt-to-workspace/subtab/cancel/approval
conflict while the Agent lock is held, changed envelope, exact restart retry,
missing admission journal, legacy/missing checkpoint migration, malformed
bindings, cross-project ID reuse, independent device IDs and competing
checkpoint CAS refusal before runner admission. A first missing-journal test
incorrectly filtered raw records instead of their nested event payload; that
fixture was repaired and is not counted as proof until rerun.

All 87 remote plus approval_owner tests passed in 12 suites; six existing
project_workspace library tests passed. Rust production build succeeded with
the existing 20 warnings. Fourteen Python harness/Blueprint tests, Python
compilation, formatting, whitespace, strict Blueprint validation and controller
validate-only passed. Validate-only launched no workers. Android app code
was unchanged this increment, so prior JVM build results are not represented
as a new run.

The production harness now independently correlates three checkpoint prompt
bindings with three accepted journal admissions, exact device/project/session
and fingerprint, rejects duplicate/missing/mismatched fixture evidence, and
requires that correlation for its full passing gate.

The first production run passed live prompt/preview/Allow/Deny checks but
failed the later directory-tab workflow on an ambiguous pending write, then
WorkManager correctly skipped that pending namespace. Its failing receipt
desktop-android-request-binding-20261002.json is retained. The existing
directory-tab Compose workflow now uses the actual Retry same request control
on an ambiguous ERROR (at most three retries), asserting the pending envelope
is unchanged. It never creates another ID or hides definite refusals. This
exercises the intended uncertain-outcome UX, not a claim that the underlying
intermittent transport failure is fixed. The test APK was rebuilt after fixing
an initial cross-module Kotlin smart cast error.

The rebuilt-test-APK rerun passed all 23 initial instrumentation tests and
subsequent process-death network/pending-write/revoke recovery, WorkManager,
four independent cache/intent fixtures and desktop listener closure.
desktop-android-request-binding-retry-20261002.json has passed=true and
prompt_global_identity_binding_verified=true. Terminal QR pixels were decoded
independently; TLS used 192.168.50.14 directly with no adb reverse. It records
ten desktop provider calls and true live approval/preview gates. This remains
API-36 emulator evidence, not a physical camera or physical LAN peer gate.

Attributed hashes:

- Rust: `a510373c93247677c8a5fcdface40d33cbe04809832c5ae54e1160eb15e52bc4`
- App APK: `d18249e1b70c3c673c314d1562f7a91b0ea0378b4002ae33e451e5e9662b63d4`
- Test APK: `9dcf1eb485875b3ab3153c81dcd43ab44e7204dc468225cc3532ff78a51aa5ca`
- Harness: `27c02c903e1333866a7d38473bfbef7c614888bcd8b956f6a21b4d62b828fc9f`

## Remaining Scope

Physical camera/LAN peer acceptance, API matrix, streaming/ACK, PTY, steering,
selected-root execution, scheduler quotas, Git crash transactions and release
remain. Full historical identity migration, killed-host checkpoints, storage
faults, timeout/cancel and simultaneous approval races remain acceptance work.
No claim of full S2-019/S2-036 or all 36 items follows from this increment.
