# Stage 2 Mapped LAN Route And Desktop Search Interoperability

Recorded 2026-10-04T08:08:54Z. Partial S2-011/S2-012/S2-021/S2-036/
S2-052/S2-053 evidence, not acceptance. The previous goal work progressed
desktop address preflight; its still-live source process was polled to exit 0,
not restarted on an observation timeout. The intervening RTK check confirmed
the installed CLI, but did not satisfy a product gate. Current turn closes the
mapped-route regression and adds actual desktop search interoperability.

Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push or permission bypass. Blueprint SHA-256
d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.
All 36 Blueprint rows remain [ ]; no worker handoff or Master acceptance added.

## Implementation And Evidence

Rust LAN selection now evaluates IPv4-mapped IPv6 route hints through the
underlying IPv4 usability predicate. Mapped unspecified, loopback (including
127.9.8.7), link-local, multicast and broadcast values cannot bypass automatic
QR address selection. A usable private IPv6 interface still wins over an
invalid hint. Valid mapped private hints remain valid for IPv6 selection, not
an IPv4-family fallback. remote_lan has eight tests after the new regression.
This is not proof of interface assignment or phone reachability.

The current Rust session fixture now obtains a searched catalog with query
desktop from HostAdapter alongside every new/resume/restart view. Kotlin checks
both historical user/assistant snippets, case-insensitive matching, the same
unfiltered catalog digest and the documented optional legacy search metadata.
Either partially missing search field refuses; removing both remains legacy
compatibility. A name-only JVM fixture checks explicit zero-match/empty-snippet
results and query-mode separation. A proposed strict-field change was withdrawn
after reading the authoritative protocol compatibility rule; production Kotlin
decoder behavior is unchanged. Search remains read-only and existing original/
historical journal byte assertions remain in the owner transaction test.

Android documentation incorrectly described a default-Wi-Fi fallback after a
missing explicit route. Corrected it to match production LanSocketRouting:
no matching nondefault route means refusal, not cellular/VPN/default fallback.
Network/parity matrices and protocol notes distinguish source evidence from
connected-device requirements.

## Verification

Final source command, canonical cwd:
zsh tools/stage2_android_source_check.sh /private/tmp/stage2-lan-search-interop-20261004.
Exit 0; production/test source edits completed before this launch. It compiled
current main/unit/instrumented sources, passed 573 ordinary JVM tests, regenerated
current terminal QR fixtures and passed every explicit current Rust/Kotlin lane,
including five session replacement/search phases, six-test desktop target and
actual Agent prompt/tool/running-worker cancellation. Instrumented assertions
compiled only, not device-run. OUT and per-lane homes isolated; credentials
unchanged. Log /private/tmp/stage2-lan-search-interop-20261004.log.

- cargo test --locked --offline --test remote_lan --test remote_pairing:
  exit 0, eight LAN and eight pairing tests; the pairing child subtest is not
  counted twice. Full log /private/tmp/stage2-mapped-lan-route-20261004.log.
  Subsequent RTK invocation also exited 0; its compact 17-test summary includes
  that child subtest and is not a distinct seventeenth domain test.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-lan-search-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 verifier
  fixtures, not connected API31/34/35 lifecycle evidence.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- cargo fmt --check, git diff --check, strict Blueprint: exit 0; 36 [ ], zero
  [_]/[x]. Strict authority validity is not product completion.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after all Cargo
  lanes; debug binary attributed to current Rust sources. An earlier concurrent
  attribution correctly refused a test-relinked binary; rebuilt/verified only
  after the authoritative source process completed. Not release acceptance.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. Existing Gradle wrapper-lock permission diagnostic inspected; no
  unchanged Gradle/socket/ADB attempt, stale APK install or permission bypass.

```text
lan.rs 846c895da6b705d64761e1e65dd779400d0df0ee593a531a9ea2270a96f76a56
remote_lan.rs 3fd8231eabd55328da11d3dcca4b051c61238a988aa7dbd2c312beb090ab4e9b
remote_android_session_interop.rs 11109aed03242181c4ac9081dfcb9a8c604baab02e394b3acca8c8d0fb36daec
AndroidRustSessionTest.kt 0599b32fffa39d85e99224a8d763f55923afd1c2bbcf89e80f2b09069b707a4f
SessionCatalogTest.kt ec6fcec7e08d8f75125876643496f6e08504c16de1cc8090a412b419a2cdc013
domain-log e2b9cf2225863e00b66adfd22a820d1660c242ac673dedfdbe8cbadb4c1554b1
source-log 4585919add5fa13e18a20f6a9142779ec14fdf9c4637249d9c87dce8610c47ad
python-log 0b7bbfb0a3e0bf2de4e4717a302f663a81f22231fd4be1012b8137a7fab65221
session-results 5e19f8bd53938b6110b1e8f6f10d12455404fdaf3e064179df89117606896ea2
main.jar 2d05d6fc9b28d9fb837571e5f203e0c4f2b67172c569bdd781a7aaccc2f5394b
unit.jar 7a407ee5672dec47c768acaaeee984f4a4af81d8cae40755425ce4f340714d04
instrumented.jar 7d5296e147572595b9301eff81d6a857db7eb1ee2ee0e4e1faf063576505c80a
host-receipt a9e5f697659cef54c56b47c3d8cc982e5df0d6e72ab73814dac14d2d29424a3e
host-inputs 9971547e0a69a38c9a30512ee37340ab10553825f20ae10d253c5c600e7faedd
host-binary 8c8dfc56ed6956ace031c8682d713b4c917262faba1ec6a60375e3f219532b89
```

Goal active/incomplete. Fresh APK provenance, physical camera QR/direct-LAN
bidirectional sync/recovery, connected API31/34/35 lifecycle, complete desktop
parity and signed reproducible release remain required. Missing external
runtime evidence does not prevent this turn's source/interoperability progress;
the blocked audit does not pass and completion remains unproven.
