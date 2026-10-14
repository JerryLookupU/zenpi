# Stage 2 LAN Binding Detects Source-Link Replacement

Recorded 2026-10-04. Partial S2-032/S2-038/S2-053 evidence. Previous goal
turn progressed session write scope; this turn improves the actual Android
LAN socket binder used by QR pairing, authenticated foreground operations and
WorkManager resume. Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push or permission bypass. All 36 Blueprint items remain [ ].
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

The binder reobserved matching routes across Network.bindSocket, comparing
network handle, prefix, forwarding type, destination, gateway and route
interface. Source address replacement, LinkProperties interface or MTU change
could leave those fields identical. NetworkMonitor observes those changes too,
but a queued main-thread callback does not establish that binding observed a
stable source link before TCP connect.

LanRoute now carries a snapshot of LinkProperties interface, MTU and complete
source address/prefix set. Both observations use the same LinkProperties
object for route and link fields. Existing bindStable equality rejects a link
change before returning to the clients' connect calls; it never rebinds to a
replacement/default/cellular/VPN route within that attempt. Address set order
and duplicates are normalized, matching existing route normalization.

Expected/actual: new JVM tests prove source removal/replacement/addition,
address prefix, interface, MTU and missing link metadata cannot reach the
fixture connect continuation despite unchanged network/destination route.
Exactly one original bind occurs. Unchanged link address sets with reversed
enumeration/duplicate entries pass. Existing longest-prefix/ambiguity/reject/
gateway/order/failure cases remain passing.

This is an observed before/after race guard, not an atomic OS route lock or
proof of reachability. A change after the final read remains handled by socket
failure/cancellation and NetworkCallback. No actual DHCP, Android routing,
socket, physical camera or direct-LAN execution is claimed by JVM fixtures.

## Verification

Source run 2026-10-04T05:13:56Z through 05:17:24Z, canonical cwd, exit 0;
OUT and per-lane ZENPI_HOME isolated under /private/tmp/stage2-lan-link-binding-20261004.
No source/test edits after that run began. User credentials unchanged.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-lan-link-binding-20261004:
  main/unit/instrumented source compilation, 539 ordinary JVM tests,
  production terminal QR corpus and all explicit current Kotlin/Rust lanes;
  six-test desktop target and actual Agent prompt/tool/worker cancellation
  lanes passed. Instrumented source compilation is not device execution.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0,
  137 pass; /private/tmp/stage2-lan-link-binding-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 fixture
  verifier tests, not connected API31/34/35 behavior.
- cargo fmt --check, git diff --check and strict Blueprint validator: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo
  lanes; reattributed current debug artifact without Rust source changes.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/socket/ADB blockers not retried or bypassed.

Final log /private/tmp/stage2-lan-link-binding-20261004.log.

```text
LanSocketRouting.kt 642c9831aeb6a1385c3083abbe73e104520c2825c8f436b00ff56ffd2af32095
LanRoutePolicyTest.kt 3f526895544bab992b275571c7d22243867c3c2e07b36d8e72e23d1939c89c61
source-log cd8e336da576d2f9c77de926364d7d2b10199e390617b45db5f262b5fdd75202
python-log 383ddd6be4c71c2b1b96058518ce76f61226c6d04a2dbfeb75785dfe6908f5de
main.jar bb0e7594ba41511cf8824469862e1c76a35d690fbe7c95b466905fc2db2c48e4
unit.jar 419ff3869ad28af32c89b2280aa7db4bc1bc26140fa395b41aed04418e7303d5
instrumented.jar c34f7b317b15ebf26f5f3643360c7358ae0d8569cecaac199edd4b16dd7733ba
host-receipt cdb180b2402390f0d5ec4dbb4a24f86ca5eb8b298b864715f07540d4f5a34426
host-inputs 373a8f5697a5bc95c10eda94c9b9b35facda736cd84bcdb0c0fa3e8703c8e58f
host-binary 2939439799b49d0e22f6072e95288a44b239e4d8eb999ad9f44bfaaf5480a861
```

Goal active/incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API31/34/35, complete desktop parity and signed
reproducible release remain required product gates.
