# Background Identity Failure And Host Smoke Exit

Provisional S2-037/S2-038/S2-051/S2-053 progress on 2026-10-04. The preceding
goal implementation turn changed synchronization status and verified source
regressions; it was progress. The intervening RTK request confirmed the existing
local installation, not a Stage 2 acceptance gate. Current execution skill,
dirty worktree, authoritative Blueprint, APK attempt diagnostics, foreground
and background identity admission, and current tests inspected. No nested
agents, workers, checkbox promotion, or unrelated reset.

## Changes

ResumeWorker previously reported SUCCEEDED/resumed=false when encrypted paired
metadata existed but its signing key was missing. This hid authentication loss
as an ordinary skipped opportunity. Existing-key admission now maps absence,
lookup failure, inaccessible public key and mismatched device ID to the existing
typed HostAuthenticationException before constructing a client or opening a
socket. ResumeRetryPolicy ends that opportunity as FAILED without retries.
Cancellation remains cancellation, not an authentication refusal. No replacement
key is generated; no cached projection or unresolved mutation is deleted.
Missing metadata and unresolved writes still intentionally skip network work.

Two JVM tests exercise matching identity, all four local failure cases,
terminal/no-retry classification, no subsequent network/commit and exact
cancellation propagation. Existing WorkManager integration test now prepares
missing/mismatched alias runs, asserting FAILED/zero retries, identical encrypted
cache bytes and no replacement identity. These device assertions compiled but
were not executed. The original successful read and pending-write skip remain.

Python regression execution exposed a host-smoke harness exit race: EOF on both
output pipes could arrive before waitpid observes process exit. The old code
immediately killed that process, turning clean or SIGTERM exits into SIGKILL.
The harness now waits for exit within its original 12-second budget, kills only
after timeout, and closes its selector. Added real child fixtures close their
output before a delayed successful/nonzero exit. Success still requires exit
zero and a valid redacted invite; this does not bypass production acceptance.

## Verification

Commands ran from /Users/mac/Github/zenpi against the dirty integration.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-resume-key-failure-20261004:
  exit 0, production/unit/instrumented source compilation; 452 JVM tests pass.
  Existing warnings remain. No Gradle packaging or connected test execution.
- python3 tools/test_stage2_host_smoke.py: exit 0, 8 tests pass.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': exit 0, 132 pass
  after the harness repair. Initial execution had 2 harness exit-race failures;
  focused rerun before repair reproduced SIGKILL instead of SIGTERM.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 21 pass.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, 5 pass.
- rtk cargo test --locked --offline --test remote_subscription --test remote_subscription_tls
  --test remote_protocol --test remote_frame: exit 0, 86 pass, 1 ignored,
  four suites. TLS fixtures use memory transport, not real Android/LAN sockets.
- rtk cargo test --locked --offline --all-targets: exit 101. Library suite
  reports 394 pass, 55 fail, 5 ignored; later targets did not execute. Output
  includes forbidden loopback listener creation, default-home configuration
  writes and extension pipe/deadline fixture failures. No full-suite pass or
  claim that every failure is explained by one cause.
- Rust format, strict Blueprint and git diff --check pass. All 36 rows remain [ ].
- python3 tools/stage2_host_build.py: exit 0, current debug host rebuilt and
  source-bound. Binary hash is unchanged; receipt below is newly generated.
- APK verify-only: exit 1, missing provenance. Existing Gradle diagnostic is
  wrapper lock Operation not permitted; prior writable-temp attempt also
  encountered daemon ServerSocket EPERM. No unchanged restricted retry or stale
  APK installation, ADB, physical camera or direct-LAN acceptance this turn.

```text
AuthenticationProof.kt 02b3f4ac33ec3c5c9e239954e63679bbe5083b5523b465fc5314f2415fb0217b
ResumeWorker.kt 8c78d2a72350300a459e5405d249296731d1755afa3cb45c6b06b3491b468544
AuthenticationProofTest.kt 0b2d7fbbea5fae1e15a1a8eddf2d1d49109b2938101d1d87a196e8016e30b454
ResumeWorkerTest.kt 2c3bc6a023168fded44aee6d6819ee9b94d766671174dcb2bd0666b0152d46a4
stage2_host_smoke.py a0c3322a0d60dcf2b59738a6ce87b0de6055778615b690d6ecea9ef68d8923e9
test_stage2_host_smoke.py 1212af1eb6befc16dfb4a8bc3cc51b7ae26f7a455ac85f97e0718a6ac757172e
main.jar f4a19f7e4072678db59c6dabb5267166ddb4d31950252c9fad9600dcfd5130b5
unit.jar 5df4f54b568ff85835297ebbb60eaa559ccd87b0f8c0fb9c92a57f7374df7cdf
instrumented.jar 49185cec7ed267be124e9ab6a9dc3bfbfaffcf0fe3dbf1794b10b32f75a49eba
debug-host-receipt ec7b1e24bd0a6e291c5a60e21746be203fcf953f8031a7a4b9ffc7e5a8d8690b
debug-host-inputs 11497524e5aeecd45f07652e03c30a96fe06977a5286b2875d5ed3b2f4033773
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active. Current APK provenance, physical QR/direct-LAN two-way
sync/recovery, current API 31/34/35 connected matrix, complete parity and signed
release are still missing, incomplete or unverified. Source tests do not accept
those gates or an entire Blueprint item. This turn made implementation and
verification progress despite the external runtime restrictions.
