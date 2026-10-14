# Worker Recovery Lease Drop

Recorded 2026-10-04. Partial S2-017/S2-018/S2-052/S2-099 evidence only.
Previous goal turn was progress (event-before-refusal Android interop).
Execution skill, Blueprint/spec, dirty worktree, historical broad failures and
current worker lease paths were inspected. No nested agents/reset or repeated
denied Gradle/ADB/TCP setup. An asynchronous question asks whether a dedicated
physical phone is available; no device availability or execution is assumed.

## Broad Audit And Repair

First ran current parallel library excluding three whole historical socket
modules: 415 pass, seven ignored, 37 filtered. This was not a full-library pass.
Then reduced exclusions to the exact 21 historical denied-listener tests:
430 pass, one worker recovery lease failure, seven ignored, 21 filtered.
The log records a false live-owner refusal immediately after dropping a lease.

Production worker handles explicitly unlocked flock, but recovery probes and
preparation/error paths returned bare File. Concurrent fork inheritance can
retain a lock past the parent's descriptor close. All worker lease paths now
return WorkerLease, whose Drop explicitly unlocks before closing. The guard
outlives the Agent through WorkerState ownership. Existing recovery admission
holds its guards across the short checkpoint commit; no lock/ACL check is
removed. The final worker handle still controls the writer lifetime.

New deterministic dup-descriptor test covers both preparation and recovery
leases: while original guard is alive a competing lease refuses; after guard
drop replacement succeeds even with inherited descriptor held; closing that
retired descriptor cannot unlock the replacement. No sleeps/retries weaken the
assertion. Four direct worker unit tests pass twice.

## Verification

All commands ran from /Users/mac/Github/zenpi with separate mktemp ZENPI_HOME
roots where host tests load configuration. All observed sessions terminal.

- Five integration targets pass: remote_concurrency 12, remote_worker_owner 12,
  remote_workspace 72, remote_worktree 6, remote_worktree_runner 13 (115 total).
- Post-repair exact-21-exclusion parallel library: 431 pass, one fail, seven
  ignored, 21 filtered. Worker recovery now passes; extension resume still
  fails at 1000 ms fixture interpreter startup. Parallel gate remains failed.
  Shell tee process exits zero, but Cargo's authoritative log explicitly says
  FAILED; that pipeline status is not claimed as a test pass.
- The extension resume exact serial rerun passes one test. Earlier pipe scope
  parallel run passes four parent cases, but RTK aggregates child results too;
  its displayed 16 is not 16 independent parent scenarios or a full gate.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-worker-lease-interop-20261004: terminal exit 0; main/unit/
  instrumented Kotlin compiles and every invoked Rust/Kotlin interop completes,
  including worker cancel and real-owner prompt. Ordinary JVM classes rerun:
  592 pass. Android source unchanged, no device execution claimed.
- Stage 2 Python tools 163 pass; lifecycle fixtures 25; Blueprint fixtures 5.
- cargo fmt --check, git diff --check and strict Blueprint pass: 36 [ ], no [_]/[x].
- Guarded debug build refreshed after source checker; verify-only passes.
  Old release/current-repeat source identities correctly refuse as stale after
  this Rust change. Historical release bytes/receipts were not rewritten.
- APK verify-only refuses missing provenance. Gradle/ADB restrictions unchanged.

```text
worker.rs 8f7474df0561c42afe7eb08bb12b621d0ac704540ac13664f568d5640c7d2197
pre-repair-exact-log 469bd0f6ac62f2e6409e9052d2a209a37c3fa72be42bd00302e97eb83d751422
post-repair-exact-log 1b111bafda24a3d373ed138e48d11320b2de0abbe498842f8435fb7b7aca3e94
debug-binary bac4e140070520197a5278ba9ede63a9adbda05744fa23dc55bc86eac12f94e0
debug-receipt 72cbe593824cc44bd07df951976442a5e9d8e205368c8a2d5cd47a968fe30351
host-inputs 2045b05d181c3bcc5754ef491dba9a8afe0aa0173f7b04ffdaf82088bc117ef5
```

Logs retained at /private/tmp/stage2-library-exact-exclusions-20261004.log and
/private/tmp/stage2-worker-lease-wide-20261004.log. No full Rust/library pass,
current release reproducibility or supported-host filesystem claim.
Goal remains active/incomplete: APK/signing/install/upgrade, physical camera QR
and direct-LAN pairing/sync/recovery, connected API31/34/35 and broad gate closure
remain required. Safe code progress remains possible despite external gates.
