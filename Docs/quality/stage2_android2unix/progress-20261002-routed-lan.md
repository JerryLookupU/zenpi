# Routed LAN Selection And Refusal

Provisional S2-032/S2-038 progress. Previous goal turn added per-socket Android
LAN binding. Current implementation review found it only considered no-gateway
unicast routes: explicit Wi-Fi routes to another LAN subnet were ignored and
most-specific unreachable/throw routes were discarded before longest-prefix
selection. This turn repairs that actual production route path. Full original
QR/LAN/Rust desktop/native Android objective remains active, 36 rows `[ ]`.

## Changes

LanSocketRouting now passes every matching nondefault route on an eligible
unrestricted non-VPN Wi-Fi/Ethernet network into LanRoutePolicy. A forwarding
unicast route may have a gateway, enabling explicitly routed LAN subnets.
Policy compares prefix lengths before route type: any nonforwarding route at
the longest matching prefix refuses binding rather than choosing a broader
route or default cellular path. Equal-best different networks remain ambiguous
and refused. A more-specific valid route can override a broader reject.
Default routes are still excluded from explicit LAN selection; no explicit
match retains the existing activeNetwork fallback. No global routing changes.

Four focused JVM cases add reject/throw precedence, equal-length refusal,
broader-reject override and invalid/default prefix checks. An exhaustive
matrix covers 1..32 prefix lengths on two networks, both forwarding flags
and both enumeration orders (8192 evaluations). Existing IPv6-prefix,
duplicate-network, literal and offline-inventory tests retained.
Production matching/type extraction remains Android API behavior; policy
tests cannot prove actual routed-Wi-Fi connectivity. No new device pass.

## Verification

Cwd /Users/mac/Github/zenpi; observed 2026-10-02T01:21:22Z UTC.

- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-routed-lan-20261002`: main/unit/instrumented
  source compilation exits 0; **133 JVM tests across 26 classes pass**.
  Existing icon/safe-call/fixture and allNetworks deprecation warnings remain.
- `env ZENPI_HOME=/private/tmp/zenpi-routed-lan-20261002 cargo test --offline
  --locked --test remote_lan --test remote_pairing --test remote_subscription_tls`:
  **23 pass**, exit 0 (7/6/10), Rust source unchanged. TLS is encrypted memory
  transport, not real TCP/LAN. Python smoke 34/lifecycle 11/Blueprint 5:
  **50 pass**. Strict Blueprint, cargo fmt and git diff checks pass.
- `./gradlew :app:assembleDebug :app:assembleDebugAndroidTest --offline
  --no-daemon`, cwd android: exit 1 before compilation, wrapper ZIP lock under
  /Users/mac/.gradle denied. No fresh APK or connected execution. SDK35 javap
  shows IpPrefix/RouteInfo accessors; RouteInfo.Builder is not public there,
  so no fake Android route constructor or fabricated instrumented result.

SHA-256:

```text
LanSocketRouting.kt 4bb4e63d09ce2e3e9d8c8db80538db0a5241b9375ea8ed955864a6fc93873623
LanRoutePolicyTest.kt e43ee1333b86a1c81944da0c9ab233b85a66b4b280443b727afeae496e8abfff
```

No agents, permission bypass, commit/push or historical receipt promotion.
VPN lockdown/routing permissions, fresh API31/34/35 APK/Keystore/socket
lifecycle, physical QR/same-WiFi and routed LAN sync remain unverified.
Independent worktree quotas 1..64, full stream delta replay/backpressure,
interactive PTY/bulk and signed release/migration remain required. Goal active.
