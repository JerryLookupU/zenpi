# Stage 2 Busy Subscription Transcript Authority

Recorded 2026-10-04T07:13:56Z. Partial S2-020/S2-032/S2-033/S2-053
evidence, not acceptance. Prior goal turn progressed invitation countdown.
Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved;
no nested agents, reset, push or permission bypass. All 36 Blueprint rows remain
[ ]. Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

The production Rust Busy subscription deliberately has no transcript phase or
journal sequence. Android's transient reducer preserved its existing transcript
but ignored contradictory phase/journal fields in such a result, and did not
explicitly check retained selected project/session before the transient path.
Malformed data could therefore publish accompanying live controls/completion
instead of refusing the whole batch, contrary to the transient-view contract.

SubscriptionReducer now rejects phase or journal_next_sequence fields (including
null) when subscription_transcript is false and verifies retained selected
project/session against the subscription. Rejection leaves projection, cursor,
completion and budget state unchanged. Valid Busy heartbeats remain legal without
a repeated view; no new journal, ACK or transcript is invented.

Added SubscriptionTest field/null/foreign-scope matrix. Extended the production
Rust adapter fixture by holding the actual Agent owner lock while the real
subscription/view dispatch generates Busy batches. The current Kotlin
RustAndroidWireTest consumes those fragmented frames, preserves message/phase/
journal/event cursor, mutates each actual result's transcript authority fields,
checks atomic refusal, then accepts the original through completion. This lane
requires the explicit fresh fixture; ordinary Gradle assumptions do not claim
that current Rust lane ran. No historical fixture changed.

## Verification

Source run 2026-10-04T07:10:37Z through 07:13:48Z exited 0, canonical cwd.
No production/test source edits after launch. OUT and per-lane ZENPI_HOME
isolated under /private/tmp/stage2-transient-scope-20261004; credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-transient-scope-20261004:
  current main/unit/instrumented compilation, 566 ordinary JVM tests, production
  QR corpus and explicit Rust/Kotlin lanes pass. Includes actual Busy-owner
  frame consumption/mutation, six-test desktop target and actual Agent prompt/
  tool/running-worker cancellation paths. Instrumented assertions compiled,
  not device-run; software fixture identity is not Android Keystore.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-transient-scope-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 verifier
  fixture tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  current debug host reattributed, production Rust sources unchanged.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/socket/ADB constraints not retried or bypassed;
  no historical APK installed.

Final log /private/tmp/stage2-transient-scope-20261004.log.
Wire corpus /private/tmp/stage2-transient-scope-20261004/android-sync-wire.json.

```text
SubscriptionReducer.kt 62d0718ee855d3704d32d9c2f8f6911a3d2d0cf91ce022e5e8dc2daa7d0d0e6b
SubscriptionTest.kt ea539fdf9c5ae6a06a65aa2a21e0f06edf1b3410b42af141316136be06ed676c
RustAndroidWireTest.kt 5acae343174b2d9eb8711008865266b207804aa9d96bb6f56b999ff930a9ab40
remote_android_interop.rs 876adf2f2b87de15580410398afa446c0f112d364b287c57e87995e1d570c26d
source-log 0bbad17dd8ffb940a541f0242236fbaa06b393e6dcc03fbacc49c30b749a0ad5
python-log 6aead4a7e3d6aa906edc87eb3411104085f221486c7ccc7100f67a45bfce0596
wire-corpus 8f7ad15c13cc07ffc9aff6f609cdb2c0e10463af4ef54a88b4325617bde9f26c
main.jar f3491a9d866cdb4540a1c99138d6538dec91eb6bdf9f09403d92c93c943060ae
unit.jar f314c023cb4efac040cea40cc4c594c4b6b89d7eb442870c6d22f01c7c4c1a68
instrumented.jar bcfa06477d4fa7a9de67f3c4b9cb51bf8a3a03b53b277ff2191c0ee47eac53e4
host-receipt d0760a238b7e92e3340aa95133e307fe5512bf8d3ba8961567c9f12dc732b32c
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required product gates.
