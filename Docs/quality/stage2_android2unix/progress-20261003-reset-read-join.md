# Local Reset Waits For Terminal And Session Reads

Provisional S2-037/S2-038/S2-056 progress on 2026-10-03. The previous goal turn
changed the physical QR validator and produced current source evidence, so it
was progress. Current source was inspected again before this repair. No nested
agents, checkbox promotion, commit/push or unrelated worktree cleanup.

## Repair

PairingViewModel.forget captured and joined its connection and most read jobs,
but omitted terminalJob and sessionsJob. closeDiff cancelled those jobs and
cleared their fields, so reset could publish completion before their coroutine
cancellation cleanup had finished. It now captures both jobs before closing
views and joins them before deleting encrypted metadata or signing credentials.

LocalResetTest adds a deterministic Android coroutine regression. It installs
two held read jobs into the production ViewModel without a production test API,
waits for both cancellation finalizers, and keeps each finalizer suspended
separately. Reset must remain pending and storage/key must remain intact during
both observation windows. After releasing both, reset must finish with both
jobs completed and credentials removed. The test finally releases/joins its
fixture jobs and waits for reset before direct fixture storage cleanup.

The production desktop suite includes that named test: 21 base plus 51 control
tests, or 72/73 total without/with Doze. Both the harness and lifecycle receipt
validator require local_reset_read_jobs_joined_verified. The lifecycle validator
rejects an absent, duplicated or unscoped marker even if a boolean and runner
summary claim success. Historical smaller suites are not current acceptance.

## Verification

- Stage2 Python discovery: 107 passed. Physical QR verifier: six passed.
- Lifecycle receipt verifier: 19 passed, including missing/duplicate reset
  completion-phase cases. Instrumentation and Blueprint tests: five each.
- Kotlin source check /private/tmp/stage2-reset-read-join-final-20261003 exits
  zero: main/unit/instrumented compile and 378 JVM tests pass. The new Android
  regression compiled but did not execute on a device. Existing compiler
  deprecation/nullability warnings remain; no new compile error.
- Rust debug/release current-source build provenance passes; Rust unchanged.
- Formatting, diff whitespace and strict Blueprint validation pass. All 36
  authoritative checklist rows remain unfinished.
- APK verify-only refuses missing provenance. No stale APK installation,
  unchanged blocked Gradle/ADB retry or physical camera/LAN pass is claimed.

Source attribution:

```text
PairingViewModel.kt 8fed0d9bfbea8dd7a54d3a52360e4f3d95f90c3bb3adc4e0bcfe9ef6c12d7094
LocalResetTest.kt 21b66766fa6f56726f6436d5be43c5838145171c4a192563d7658c5271e1ee47
stage2_desktop_android_smoke.py 8d7976fd17e48abf0e2be4bba2ffd08258413e88e842ce0d8c203610fd546444
validate_stage2_android_lifecycle.py f7c26e1b0bd1d37aa29bb461da8d950e35d19622f9ffa19b44a5f5b04b019877
test_validate_stage2_android_lifecycle.py 90811dc247318317bd094ceb43572d7dd5e9b95e0b33f3215bd9e312c15b19a9
main.jar 6b346b327a8a91f377df3dbbb0d1f13df6a2f78c868206fa8f13051d2f76cc8f
unit.jar a2a24570e2dffdd018d635b161598dad8b0ab38eb3d269fe7355ed89784f395f
instrumented.jar c7dbe03553e6af335fbc3631593cd20218c99fa52047be546fc7169b219a639f
```

Goal remains active. The new regression requires current APK/device execution;
source compilation is not Keystore/socket/lifecycle acceptance. Full Android
parity, physical QR/direct LAN desktop sync and release gates remain unaccepted.
