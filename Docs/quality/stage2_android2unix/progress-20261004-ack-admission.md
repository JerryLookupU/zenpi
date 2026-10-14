# Stage 2 ACK Request Admission

Recorded 2026-10-04T07:20:04Z. Partial S2-020/S2-032/S2-033/S2-053
evidence, not acceptance. Prior goal turn progressed Busy subscription scope.
Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved;
no nested agents, reset, push or permission bypass. All 36 Blueprint rows remain
[ ]. Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

EventAckCodec built numeric cursors directly from retained strings and validated
receipts against separately supplied expectations, without validating that the
supplied request matched those expectations. Noncanonical retained cursor text
could normalize silently, and a mismatched request could enter exchange/Busy
deferral before the frame/expectation disagreement was noticed elsewhere.

Shared ACK sequence validation now requires canonical u64 text before request
construction or receipt comparison; bounded nonempty project/session identifiers
are checked locally. acknowledge compares the full supplied frame against its
expected builder output except for a valid optional correlation ID, then runs
FrameCodec encoding/validation before exchange. Extra authority, foreign routing,
wrong cursor/operation or null ID refuses without consuming a response. Valid
duplicate/lower ACK high-water receipts and typed Busy deferral remain unchanged.

Two new EventAckTest cases cover invalid sequences/scopes, maximum u64 and a
seven-case zero-exchange mismatch matrix. Existing tests now pass real ACK frames
instead of empty placeholders. The actual AndroidRustAckTest lane mutates each
current ACK to foreign project scope, verifies zero exchanges, then consumes the
original Rust receipt/restart replay as before. No host checkpoint semantics or
request IDs change; client validation does not prove an untrusted client saved
its bytes, and no ACK-based journal deletion is added.

## Verification

First source run exited 1 with one new test failure. ID validation/source edits
had occurred after that compilation started, so that result is not used as
current-source evidence. Observed the original process terminal before rerun.
Final source run 2026-10-04T07:17:17Z through 07:19:55Z exited 0; no source
edits after final launch. Canonical cwd; OUT and per-lane ZENPI_HOME isolated
under /private/tmp/stage2-ack-admission-final-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-ack-admission-final-20261004:
  current main/unit/instrumented compilation, 568 ordinary JVM tests, production
  QR corpus and explicit current Rust/Kotlin lanes pass, including ACK restart/
  receipt/admission negatives, six-test desktop target and actual Agent prompt/
  tool/running-worker cancellation paths. Instrumented assertions compiled only.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-ack-admission-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 verifier
  fixture tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  current debug host reattributed, no production Rust source change.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/socket/ADB constraints not retried or bypassed;
  no historical APK installed.

Failed log /private/tmp/stage2-ack-admission-20261004.log.
Final log /private/tmp/stage2-ack-admission-final-20261004.log.
ACK exchanges /private/tmp/stage2-ack-admission-final-20261004/android-ack-results.json.

```text
EventAckCodec.kt 7a6847f25b1ac5a4fe8ff2121530d851cc29b4d91517735a6c7de3758d9c2bd8
EventAckTest.kt cc3ffd8de102591735d0ad50d502b106f4eee211a2f1b86736ca7129c81f630f
AndroidRustAckTest.kt f834445473d07fa877f7d663a747b54a50f54ee867e45d4e756d9b3b028eadaa
source-final-log 5b906e4d8a158ed5c546fba40a8129bd1aec03f17d39058e41da85f8d4fe3dd6
python-log 544d10fa4701499b1b6cac07d5c8fe97d5698275cfdd110fa889f0b6b74790e2
ack-results 59478a028fc8e574135cb5a8defab6e36e2d38b39e13eed7da471a3ae5c7d646
main.jar 9caaf8e8c833403b7609894cf4e1ace6a171a857ec93b3d92ca7684771482fea
unit.jar a756c11605a7b32d2485334aba7fe146cc01628207f5db0f500228b07f9cc467
instrumented.jar bcfa06477d4fa7a9de67f3c4b9cb51bf8a3a03b53b277ff2191c0ee47eac53e4
host-receipt 2d051a0f34c9a16e9db1011d2b1b1ba6e3ff7e77da71d2c053aff8dc4e0f7d5f
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required product gates.
