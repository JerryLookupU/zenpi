# Direct Private-Interface Validation

## Change

The production desktop/Android harness now supports `--lan-ip` instead of
requiring adb reverse. It accepts only literal RFC1918 IPv4, verifies a local
bind before starting the test, binds the real desktop listener to that
interface, and independently decodes the production terminal QR. The decoded
address must match the listener. Existing adb reverse mappings fail admission
and are never removed by direct-mode cleanup. Direct mode verifies an empty
reverse table before and after the run. The default reverse mode remains
available and only removes its own mapping.

Independent-process encrypted-cache restoration now contributes to the overall
pass gate, rather than only appearing as a receipt field. New receipts include
the harness SHA-256 alongside the Rust/APK hashes.

## Evidence

`desktop-android-direct-lan-interface-20261001.json` records the direct run on
the host's `192.168.50.14` interface with `adb_reverse_used=false` and
`lan_interface_tls_verified=true`. Production TLS pinning, one-use pairing,
initial denied access followed by a local grant, shared desktop transcript
updates, replay, production Compose workspace open/close, default-network loss
and recovery, live Android process death, same-request pending-write recovery,
Keystore/cache restoration, host revocation, and listener teardown passed.
The initial instrumentation batch has seven tests; network-process restoration
has one; two independent cache-process fixture runs each have one.
The final direct-interface run passed after the cache pass-gate and harness
digest additions. `desktop-android-reverse-route-regression-20261001.json`
records a passing run of the same seven-test/process-recovery/cache sequence
through the default reverse route, with its owned mapping removed afterward.
Both final receipts identify the same harness and Rust/APK hashes.

`python3 tools/test_stage2_desktop_android_smoke.py` covers the address admission
boundaries, rejection of existing reverse mappings without deletion, empty
reverse-table admission, and failure propagation when adb cannot prove the
reverse state.
The strict blueprint validator, its five fixture tests, Python compilation,
and `git diff --check` also passed. Wi-Fi and mobile-data settings were both
restored to enabled. The owned emulator was stopped after validation; the
temporary hosts/provider fixtures and reverse mappings were cleaned up.

## Limits

The Android device is an API 36 emulator on its virtual network, not a physical
LAN peer. This proves direct IP/TLS access to the host's private interface but
does not prove physical same-Wi-Fi reachability, camera scanning, camera
permission/lifecycle behavior, or the API 31/34/35 matrix. Receipts intentionally
keep camera, physical-LAN-peer, and broad LAN-reachability claims false.
Worktree/subtab parity, remote session controls, live push/ACK/backpressure,
WorkManager and the complete network fault matrix remain unfinished.

All 36 authoritative blueprint entries remain unchecked. No commit, push,
worker acceptance, or full-goal completion is claimed.
