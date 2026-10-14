# Subscription Actual Wire Budget

Provisional S2-020/S2-032/S2-053 progress on 2026-10-04. Previous goal turn
changed background cancellation/identity handling and verified source/tests,
so it was progress. Current execution skill, dirty worktree, Blueprint,
subscription output, wire-frame conversion, Android reader and replay bounds
inspected. No worker/nested agent admitted; no checkbox promotion.

## Fix

Android ResponseReader budgets original JSON body lengths for every event,
snapshot and terminal response across a subscription. Host subscription serving
previously counted only the internal result object, including private
wire_frames but excluding control-envelope overhead. That could permit output
which a conforming Android reader necessarily refused near the byte boundary.

The host now converts the response into its actual wire frames before admission
and sums complete serialized JSON bodies, matching the production frame writer.
No frame from an over-budget batch is emitted. Nonterminal batches reserve the
size of the longest correlated v1 failure-code spelling, so a later refusal is
readable without exceeding Android's 196608-body-byte budget. Prefix bytes are
outside this body budget on both ends. The event/frame/page/deadline bounds and
persist-before-publication/ACK semantics are unchanged; no journal/write effect
or alternate transport was introduced.

Three regressions establish both sides of each boundary: exact terminal budget
is emitted, one extra byte is refused before event output; exact nonterminal
budget plus longest refusal fits, one extra byte refuses the first page; a
large already-committed transient page followed by revocation keeps the full
output within budget. Existing fragmented real rustls in-process TLS tests
also pass. These are not production TCP or connected Android LAN evidence.

## Verification

Commands ran from /Users/mac/Github/zenpi against current dirty integration.

- rtk cargo test --locked --offline --test remote_subscription
  --test remote_subscription_tls: exit 0; 60 pass (17 subscription, 43 TLS).
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-wire-budget-20261004:
  exit 0; production/unit/instrumented sources compile, 448 JVM tests pass.
  Existing warnings remain. Instrumented tests compiled only, no device run.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': exit 0; 130 pass.
- Lifecycle verifier tests 21 pass; Blueprint verifier tests 5 pass.
- Strict Blueprint, Rust format and git diff --check pass; all 36 rows [ ].
- python3 tools/stage2_host_build.py: exit 0; real offline locked debug binary
  rebuilt and attributed after tests; subsequent --verify-only passes.
- python3 tools/stage2_android_build.py --verify-only: exit 1; missing provenance.
  No unchanged blocked Gradle/socket retry, stale APK install, ADB/camera/LAN run.

```text
subscription.rs 611e51aae03dbfe828967aba2bf4458dea87ea9d50dd1f578101365231c15f27
remote_subscription.rs 9a28da3a4b14f82b4dfcb697a4b169aceb831c02cbf461882a73d6a7e693f251
main.jar 5ae95f8eafba1f92796cf52d2c2b766cb1f88d5bdf27660db59598ec21c6e8a5
unit.jar 27f3f23db2139bb53474825a220ff6c9223f1cba8e8084dbf6f8601f04caaea4
instrumented.jar 422f01da32de40e7fc570ca8a0d66c269e41666cf3edacde02150ca1965f3c1e
debug-host-receipt 4b3615a4ec77a551bb958ed7f0742dd4ffb65953e8739833ed41490cd1376993
debug-host-inputs 11497524e5aeecd45f07652e03c30a96fe06977a5286b2875d5ed3b2f4033773
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active. Current-source APK, physical QR/direct LAN bidirectional
sync/recovery, connected API 31/34/35 lifecycle matrix, full parity and signed
release remain incomplete or unverified. Test/source gates do not accept those
requirements or an entire Blueprint item.
