# Stage 2 Authenticated Identity Read Cancellation

Recorded 2026-10-04T07:03:31Z. Partial S2-032/S2-038/S2-053 evidence,
not acceptance. Prior goal turn progressed pairing preconnect admission.
Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved;
no nested agents, reset, push or permission bypass. All 36 Blueprint rows remain
[ ]. Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

AuthenticationProof.current checked cancellation only before identity reads.
A Keystore read blocked during foreground/network cancellation could return a
matching ID and still admit the next signing, transmission or publication stage.
The socket closer limited subsequent I/O, but did not replace a post-read
ownership check. HostClient also lacked a current check after TCP connect and
before TLS wrapping.

AuthenticationProof.current now checks cancellation before and after validating
the device ID. The post-read cancellation stays the original CancellationException,
not a terminal authentication failure. HostClient rechecks after connect before
TLS wrapping. No wire schema, request identity, retry policy, key creation or
host rollback behavior changes; this is not an atomic external-key transaction.

Expected/actual: a latch-blocked identity read returns after cancellation and
cannot admit the next stage; cancellation inside the first identity read prevents
signing, and cancellation inside the post-sign read prevents proof transmission.
Healthy signing performs one sign and active/identity/active checks on each side.
Two added JVM tests pass. Prepared named LanSocketRoutingTest deletes a task-local
authenticated key inside bind and expects one bind, no TCP connect, closed socket
and absent alias. It compiled only. Explicit desktop base suite count updated
from 36 to 37; named completion checks remain mandatory.

## Verification

First source run exited 1: new test passed a Throwable to an Exception-only
authority predicate. Corrected that assertion to use the exact already-asserted
CancellationException. The first process was observed terminal before restarting.
Final source run 2026-10-04T07:00:34Z through 07:03:24Z exited 0; no source edits
after launch. OUT and per-lane ZENPI_HOME isolated under
/private/tmp/stage2-auth-read-cancellation-final-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-auth-read-cancellation-final-20261004:
  current main/unit/instrumented compilation, 562 ordinary JVM tests, production
  QR corpus and explicit current Rust/Kotlin lanes pass. Includes six-test
  desktop target and actual Agent prompt/tool/running-worker cancellation paths.
  Not Gradle/APK/device acceptance.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-auth-read-cancellation-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 verifier
  fixture tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  current debug host reattributed, no Rust source change or release acceptance.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/socket/ADB constraints not retried or bypassed;
  no historical APK installed.

Logs /private/tmp/stage2-auth-read-cancellation-20261004.log (failed compile)
and /private/tmp/stage2-auth-read-cancellation-final-20261004.log (passed).

```text
AuthenticationProof.kt c831fc7f9f710c5ba520ed250c617ad3afe90431b20e450b2ef58e26a0a16aea
HostClient.kt 2ce526c72b3596c24700d859aed9321121436fd89af37b1949ff97ef52a25106
AuthenticationProofTest.kt 729e869ba70af23ee50acde6564bd62df3bd133604ee80283f091781fc3a2edb
LanSocketRoutingTest.kt 94a4204d1f9aaad8f1e99ddd2d2417833fc73b3cb9ffd0627b69e42e1b8da37d
stage2_desktop_android_smoke.py 90279f247d9cdea654b8e230ec61cb8a3f0ac70b53b3a6c8ad3e89b87a57b98d
source-final-log 2d8276938e4e5345f3f1af77a875ad741aaafcc653d95ef426e123824c8d50aa
python-log d26b1ff7dfc0ebd02602513eab13a99a4dccd7a04cfba283db86265565ff012f
main.jar 16c3194638137f1f71b7c4f8dec95856e363fc7cf27703935302b25ae861c239
unit.jar 2116973c372abb8e45d44d95fc2c4c164a1b3e53672e20db0624ee8df7887d27
instrumented.jar bcfa06477d4fa7a9de67f3c4b9cb51bf8a3a03b53b277ff2191c0ee47eac53e4
host-receipt c473f8921d29f684e5ee0d5916ad3ceb9d3f9a73d8af7e08a6e81dc33e9fc740
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required product gates.
