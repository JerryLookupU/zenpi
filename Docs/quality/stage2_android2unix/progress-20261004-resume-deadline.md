# Background Resume Cancellation And Identity

Provisional S2-038/S2-053/S2-055 progress on 2026-10-04. Previous goal turn
changed packaged backup inspection and ran real compiled-resource positive and
negative fixtures, so it was progress. Current execution skill, dirty worktree,
Blueprint, background worker, deadline helper and retry policy inspected.
No workers/nested agents admitted; no authoritative checkbox changed.

## Implementation

ResumeWorker previously caught any TimeoutCancellationException before the
general cancellation path and returned WorkManager retry. That conflated its
own 20-second deadline with a parent timeout cancellation. It now uses the
existing OperationDeadline helper, which converts only its own expiry into a
SocketTimeoutException; parent/WorkManager cancellation propagates unchanged.
The worker retains the 20-second outer and 15-second transport deadline.
ResumeRetryPolicy now centralizes the two-retry budget and explicitly propagates
cancellation. Authentication/authority failures remain terminal; ordinary
transport/worker timeouts retry at attempts 0/1 and fail at attempt 2.

Existing signing-key load errors are mapped to typed authentication failure,
and the worker uses AuthenticationProof to verify its captured host/device ID
before socket creation. A missing key still skips network work without creating
one. Unreadable/deleted/mismatched identities do not consume transport retries.
No mutation replay, token retry or background socket persistence was added.
The existing ensureActive check precedes the encrypted-record CAS commit;
observed cancellation prevents that commit, without claiming synchronous file
IO can be atomically interrupted after the check.

New JVM regressions check attempts 0..5, typed identity refusal, exact
cancellation identity, worker-owned expiry and parent timeout propagation with
zero retry/publication calls. These exercise production deadline/retry helpers,
not a running Android WorkManager cancellation or actual device lifecycle.

## Verification

Commands ran from /Users/mac/Github/zenpi against current dirty integration.

- Final zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-resume-deadline-final-20261004: exit 0; production, unit
  and instrumented source compile; 448 JVM tests pass. Instrumented tests did
  not run. Existing deprecations and two test unreachable-code warnings remain.
- Initial source check failed on a Kotlin test expression needing an explicit
  Unit after the retry assertion; corrected before the successful final run.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': exit 0; 130 pass.
- Scanner verifier tests 13 pass; lifecycle verifier tests 21 pass;
  Blueprint verifier tests 5 pass.
- Strict Blueprint, Rust format and git diff --check pass; 36 [ ], zero [_]/[x].
- Host build verify-only passes; Rust source unchanged this turn.
- APK build verify-only: exit 1; missing provenance. No unchanged blocked
  Gradle/socket retry, old APK install, ADB/camera/physical LAN acceptance run.

```text
ResumeWorker.kt 9417e1a1855aad2cdb2ee6a52616fd549bc06225cd4a527928edcadc26195c9b
ResumeRetryPolicyTest.kt af60566e060cb6363c89c8daa98ed77eb0d6e85f3c991f59d7279be5bfc121d4
main.jar 5ae95f8eafba1f92796cf52d2c2b766cb1f88d5bdf27660db59598ec21c6e8a5
unit.jar 27f3f23db2139bb53474825a220ff6c9223f1cba8e8084dbf6f8601f04caaea4
instrumented.jar 422f01da32de40e7fc570ca8a0d66c269e41666cf3edacde02150ca1965f3c1e
debug-host-receipt 91315921cde2e9261bda15641051eae65e6a2ce6d9b644be1186822d3d8e359e
debug-host-binary d9ef9b027304d65c79591e8187fd5439127f17124771d25bb8a4fcf21b1b83e4
```

Goal remains active. Current-source APK, physical QR/direct LAN bidirectional
sync and recovery, connected API 31/34/35 lifecycle matrix, complete parity and
signed release remain incomplete or unverified. Source/helper checks do not
accept those device/production transport requirements or an entire item.
