# Fail-Closed Network Observation

Provisional S2-032/S2-038/S2-053 progress. Previous goal turn added foreground
storage admission and was progress. Current skill, Blueprint/strict gate,
APK refusal, connection/request recovery, NetworkMonitor and LAN binder,
route-state and device lifecycle tests inspected. Existing dirty integration
preserved; no nested agents, reset, commit, push or checkbox promotion.

## Defect and Fix

NetworkMonitor handled RuntimeException while registering callbacks during
start, but later callbacks posted refresh to Main without an inventory exception
boundary. ConnectivityManager network/capability/link/default reads can fail;
that could crash the main callback and leave the ViewModel's old usable route
token rather than fence the connection.

Each inventory read now completes as one observation through NetworkObservation.
A RuntimeException retires the monitor generation, unregisters both callbacks,
clears lastPublished and publishes null/offline. No partial map is applied.
Queued refreshes from the old generation return without restoring availability.
An initial read failure returns before any callback registration; a later
Activity start admits a new cycle. Registration failure also clears the last
published stamp before offline. No LAN binder/default/cellular/VPN fallback,
socket keepalive or automatic write retry was added.

Two JVM tests cover query completion, failure after partial inventory work,
retire-before-offline ordering for SecurityException/IllegalStateException,
and stale-generation invalidation. Consumer failure and fatal Error are not
silently converted to network observations. These tests establish the helper
contract and source integration, not actual Android system-service failure or
callback unregister behavior. Existing connected network-loss/restart test
still needs current APK execution.

An asynchronous user question requests the physical Android model/API/same-Wi-Fi
details for final camera/direct-LAN acceptance; no answer assumed. Work did not
wait on that optional clarification or narrow the goal to source checks.

## Verification

Cwd /Users/mac/Github/zenpi, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Final UTC
2026-10-03T22:12:16Z (Oct 4 local time).

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-network-observation-20261004: exit 0, pipefail/rg;
  current QR fixtures, main/unit/instrumented source compilation, 477 ordinary
  JVM tests and all current cross-language lanes pass.
- Python Stage 2 discovery: 135 pass; lifecycle verifier tests: 25 pass;
  Blueprint/backup verifier unit tests: 5 each.
- Strict Blueprint, Rust fmt and git diff --check pass; 36 rows remain [ ].
- Cargo lanes changed the executable attribution; actual debug host rebuilt
  afterward and separate verify-only validates it. Binary/source hash unchanged
  by Android-only fix.
- APK verify-only refuses missing build provenance. No unchanged blocked
  Gradle/ADB retry, permission bypass, stale APK install, camera/LAN/connected
  lifecycle or release acceptance claimed.

```text
NetworkMonitor.kt d5dafba68171d3085a21820d3cf172e363e18c983f192cefb2f0d185e7cfd634
DefaultRouteTest.kt 4a13e9fd73f655f9d8732747ac23aa81d4cb52295e91893247635d05f71362b0
main.jar 59033da0c2cd6572ef7712e4378993eacbf9add7d649bb7cb4233d4067172169
unit.jar 3c72dc2efa440365b56cf5eeaa960158afc8e0c75997149e1cb5d1726847f4a6
instrumented.jar f4d12e6a318318059210506a6a3538fa58fc525b911e3fb6f71fa9129e0ee7f4
debug-host-receipt 87af07bc28add273c34b8876cde9ed7f9688e6b1c431d368d118a17f9977244e
debug-host-binary 8a826eef42d30492ace13ea37eb454d6c7819bf91251beb6942dcd683cd039e9
```

Goal active and incomplete. Source-attributed APK, physical camera QR/direct
LAN two-way synchronization and recovery, connected API31/34/35 lifecycle,
complete parity, migration/reset/uninstall and signed reproducible release
remain required unaccepted gates.
