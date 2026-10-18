# LAN Socket Binding Reobservation

Provisional S2-032/S2-038/S2-053 progress. The preceding RTK request verified
the installed tool but did not advance Android acceptance. Current execution
skill, authoritative Blueprint, dirty worktree, host attribution, Gradle builder,
network monitor, LAN binder and both transport callers were inspected. Existing
changes were preserved. No nested agents or checklist promotion.

## Change

LAN selection previously enumerated matching explicit routes and then checked
only the selected network's capabilities before binding. LinkProperties could
change between those reads without invalidating the selected handle.

The production binder now uses LanRoutePolicy.bindStable: observe the complete
matching explicit LAN route set, select the unique longest forwarding network,
bind the unconnected socket and observe again before returning to TCP connect.
Comparison includes network, prefix, forwarding, destination, gateway and
interface. Route disappearance, replacement, ambiguity, rejection or a failed
read/bind refuses this attempt. Order and duplicate records are ignored. No
replacement selection, cellular/VPN fallback, process-wide binding or automatic
write/token retry was added. Both PairingClient and HostClient close their owned
socket in finally when this binder throws.

Four added JVM tests verify observe/bind/reobserve/connect order, stable duplicate
inventory, route-change matrix, gateway/interface changes, observation/binding
failure propagation and initial refusal without binding. These prove policy
and source integration, not Android OS binding behavior. Reobservation is not
an atomic routing freeze: subsequent OS changes still rely on cancellation,
pinned TLS and actual connected-device recovery acceptance.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; UTC 2026-10-03T22:19:31Z.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-lan-bind-current-20261004: exit 0; 481 ordinary JVM tests,
  current QR fixtures, production/unit/instrumented source compilation and all
  explicit current Rust/Kotlin interop lanes pass. Instrumented tests compile
  only. A prior 480-test run was superseded after adding gateway/interface
  coverage. The sole later source edit clarified a comment, not behavior.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': 135 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: 25 pass.
- Strict Blueprint, Rust fmt and git diff --check pass; all 36 rows remain [ ].
- Cargo interop lanes changed binary attribution. Actual debug host rebuilt
  afterward, restoring current build provenance with unchanged binary hash.
- python3 tools/stage2_android_build.py: exit 1. Fresh diagnostic retains the
  Gradle wrapper FileNotFoundException for its global .zip.lck, Operation not
  permitted. Attempt passed=false; no successful APK provenance published.
  Verify-only refuses missing provenance. No permission bypass or stale APK
  installation attempted. Earlier writable-cache diagnosis separately reached
  the known local daemon-socket restriction; this attempt stopped at the lock.

```text
LanSocketRouting.kt f1381f851425c8c0044a5f9ad0da169741f5d38b8607a9960d5e6fa7548238d7
LanRoutePolicyTest.kt 0627fe6222739af5f39e33d1d626916bb72e22e229521c23ecc594d4fcce7ba8
source-log cccebf7156b92112d05c16a37c1e12a68c8d11476800910e3ac08a48f7ea1a94
main.jar 01762f14857539ead873df69a98c61b8e51ac94aaeaecd6846d9e4bed34cff13
unit.jar 7a44c66513cdfc8f201fba9a00b4c7b926367a18cf838ec6559c46064e0c22ef
instrumented.jar f4d12e6a318318059210506a6a3538fa58fc525b911e3fb6f71fa9129e0ee7f4
debug-host-receipt 60d8be3b2f70ba9624398599874fc8c1fa66b67b3c351050108ef0f08d7b1517
debug-host-binary 8a826eef42d30492ace13ea37eb454d6c7819bf91251beb6942dcd683cd039e9
```

Goal remains active and incomplete. Fresh source-attributed APK, physical camera
QR/direct LAN bidirectional sync and recovery, supported API lifecycle matrix,
complete desktop parity and signed reproducible release remain unaccepted.
