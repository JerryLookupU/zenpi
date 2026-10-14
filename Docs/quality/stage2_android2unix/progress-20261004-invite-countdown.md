# Stage 2 Invitation Countdown Observation

Recorded 2026-10-04T07:08:33Z. Partial S2-031/S2-032 evidence, not acceptance.
Prior goal turn progressed authenticated identity-read cancellation. Canonical
cwd /Users/mac/Github/zenpi; dirty HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b.
Existing changes preserved; no nested agents, reset, push or permission bypass.
All 36 Blueprint rows remain [ ]. Blueprint SHA-256
d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

PairingInvite.remainingSeconds validated freshness with one monotonic clock
read, then performed a second independent read outside the invitation lock.
Expiry or an elapsed-time anomaly observed only in the second read could return
zero without setting the permanent expired flag, and a later corrected clock
could make that parsed invite appear valid again. The protocol's own freshness
checks remained a separate fence; no claim of previously accepted expired tokens.

Extracted shared freshElapsed validation. Both requireFresh and remainingSeconds
hold the same invitation lock. Countdown validates and computes from one clock
observation, retaining the smaller wall/monotonic bound and rounding behavior.
Observed expiry or anomaly permanently retires the invitation. Host one-use
token admission and post-send receipt behavior are unchanged.

Expected/actual: healthy countdown performs exactly one clock read; near-expiry
wall rollback remains capped to one second; exact monotonic expiry returns zero
and cannot revive after clock recovery. Countdown-observed backwards elapsed time
also permanently refuses confirmation/freshness. Two added JVM tests pass;
existing wall expiry, signed monotonic wrap and Long.MAX_VALUE tests pass.

## Verification

Source run 2026-10-04T07:05:07Z through 07:08:21Z exited 0. A small exception
handling adjustment was made at 07:05:25Z, before main compilation emitted its
jar at 07:06:01Z; final source hashes below match tested current source. No
Kotlin edits thereafter. Canonical cwd, OUT and per-lane ZENPI_HOME isolated
under /private/tmp/stage2-invite-countdown-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-invite-countdown-20261004:
  main/unit/instrumented compilation, 564 ordinary JVM tests, production QR
  corpus and explicit current Rust/Kotlin lanes pass. Includes six-test desktop
  target and actual Agent prompt/tool/running-worker cancellation paths.
  Instrumented assertions compiled only, not device-run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-invite-countdown-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  current debug host reattributed, no Rust source change or release acceptance.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/socket/ADB constraints not retried or bypassed;
  no historical APK installed.

Final log /private/tmp/stage2-invite-countdown-20261004.log.

```text
PairingInvite.kt 5695482a107b57d194876f1c02d32c05c36ca4feb05e46b3491e6a3f1707ccb9
PairingInviteTest.kt 2b9d26c8e4a54e7592404278315ca60491ee6a6686c48c8c805af8548f4a7e6f
source-log 98d496bf1acb8174412f9d0229ec450a17970db5c4379dd1c125f8d2383735a3
python-log 947db2d5d77109d59938d823c6bc04d785430bafc409db7976c303db9cc9b713
main.jar d574962ac175d08380aa115ced8fd5273233478fd359ca95a79ebe658c732d30
unit.jar c76a0e0647949bb1b4f533a26e0c1cd8792b056d6322a5761be599f8adf73110
instrumented.jar bcfa06477d4fa7a9de67f3c4b9cb51bf8a3a03b53b277ff2191c0ee47eac53e4
host-receipt 268d8e59e56121ca4fa7f4cca7bffdbc140609628d5c3fe6a36570817a8bbba8
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required product gates.
