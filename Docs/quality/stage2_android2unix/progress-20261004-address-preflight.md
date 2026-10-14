# Stage 2 Desktop Pairing Address Preflight

Recorded 2026-10-04T07:55:45Z; host rebuild completed after this timestamp.
Partial S2-011/S2-012/S2-021/S2-052 evidence, not acceptance. Prior goal turn
progressed transcript page metadata. Canonical cwd /Users/mac/Github/zenpi;
dirty HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved;
no nested agents, reset, push or permission bypass. All 36 Blueprint rows remain
[ ]. Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## Finding And Change

Explicit host bind/advertise address policy was weaker/earlier than invite
validation: link-local/broadcast/mapped unsafe addresses could create identity
state or attempt a listener before later invitation refusal. HostPairOptions
now shares pairing::valid_address through a crate-private predicate, checking
parse, run, attached start and start_with_registry before side effects. Port
zero is evaluated with a placeholder valid port because listener allocation is
intentional. Wildcard bind remains legal; scoped/flow-labelled IPv6 and unsafe
mapped addresses refuse. Advertisement must be safe and match family/exact bind.

New remote_tui_pairing test exercises production run with 12 invalid combinations,
requires a pairing argument error and absent identity/registry files, repeats
parser refusal, and accepts six legitimate wildcard/loopback/private IPv4/IPv6
port-zero forms. Existing host unit tests and LAN/invite tests pass. This does
not prove interface assignment or phone reachability, nor change listener opt-in,
ACL, QR content, loopback fixture distinction or ephemeral-port behavior.

## Verification

All commands canonical cwd, explicit host validation uses isolated
ZENPI_HOME=/private/tmp/stage2-address-preflight-20261004/home. Credentials
unchanged. No production/test edits after source checker launch.

- cargo test --locked --offline --test remote_tui_pairing
  unsafe_explicit_addresses_refuse_before_identity_or_listener_creation -- --exact:
  exit 0, one test; no socket is needed for this refusal-first production path.
- cargo test --locked --offline --test remote_pairing --test remote_lan:
  exit 0, 8 pairing + 7 LAN tests. Hardlink/FIFO safety test's child subtest is
  not counted twice. Interface inventory is not phone connectivity evidence.
- cargo test --locked --offline --lib host_pair::lan_selection_tests:
  exit 0, five tests. No restricted real listener tests retried.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-address-preflight-interop-20261004:
  2026-10-04T07:49:45Z through 07:55:32Z, exit 0; main/unit/instrumented compile,
  572 ordinary JVM tests, production QR corpus and explicit current Rust/Kotlin
  lanes, six-test desktop target and actual Agent prompt/tool/worker cancellation.
  Per-lane homes isolated; instrumented assertions compiled only, not device-run.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 verifier
  fixture tests, not connected API acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0;
  36 [ ], zero [_]/[x].
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after Cargo lanes;
  debug binary rebuilt from changed production Rust sources, not release acceptance.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/socket/ADB constraints not retried or bypassed;
  no historical APK installed.

Logs under /private/tmp/stage2-address-preflight-{cli,domain,unit,python}-20261004.log
and /private/tmp/stage2-address-preflight-interop-20261004.log.

```text
host_pair.rs 41d92c3672b41d4a979bba66c833455269e6f1cd6876694a837e2efe468e2ec5
pairing.rs 20a57c037703a8ca612eebeccc3901abec1ccd1d97d95bcfbe33b619aaaa1bd0
remote_tui_pairing.rs 86ea20f9f200e2fe1b48d6400be96ff3827b591ad293a2720a07a998d4f38da5
cli-log 99610f116c6e8383d86fa0638930de4f476a8ff1d7e2975839385bea01326797
domain-log a71e4f848705a8393974bb864c02fc5fb1cd8102e4fcdaaa79b45562c063bb36
unit-log 8d8e31490323a8b1df1914fe57c6a7c7d2d97e6e92f31f075dd6a85152b7903a
python-log 0dcf52220c516e36eec968ea74ace3b2fe0551db557e2a34f590ed418f7c1c97
interop-log 2940d405bb7a71d1edde2536ca84459f4c5600326844c08531a6fa2c5f21d8aa
main.jar 2d05d6fc9b28d9fb837571e5f203e0c4f2b67172c569bdd781a7aaccc2f5394b
unit.jar 2b9d562e803850a26992a2542a1a41f8a820d76085d3e46051d6ec6e04aabf70
instrumented.jar 7d5296e147572595b9301eff81d6a857db7eb1ee2ee0e4e1faf063576505c80a
host-receipt 2cd4ef82d8b1aa1b8d61e64d296dc6641b0afbb8667810a9ca8d842f99adc14f
host-inputs 9f19711f579ec8f632cb9115475dcfa195bd1debac7232d6d6eec3681b840ba3
host-binary 624366e2ccc3152d935a2605bead3c1037809bdf0ce44533b568cb2813c8860c
```

Goal active/incomplete. Current APK, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required product gates.
