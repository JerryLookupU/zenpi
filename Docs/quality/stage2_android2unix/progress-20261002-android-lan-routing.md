# Android Per-Socket LAN Routing

Provisional S2-031/S2-032/S2-038 progress. Previous goal turn fenced stale
subtab revisions. Current production Android path used only active default
network monitoring and ordinary sockets, so local Wi-Fi without default
Internet routing could be hidden while cellular stayed default. Full QR/LAN/
Rust desktop/native Android goal remains active, all 36 Blueprint rows `[ ]`.

## Changes

LanSocketRouting enumerates current visible networks and direct unicast
nondefault no-gateway routes matching the literal host. Only unrestricted,
non-VPN Wi-Fi/Ethernet is a direct candidate. Longest matching prefix wins;
equal-specificity different networks fail before connect. The selected
Network.bindSocket operates on one unconnected socket; absent direct matches
use activeNetwork, absent both fails. Loopback skips binding for adb reverse.
Strict literal syntax rejects hostname/legacy IPv4 before resolution. No
global bindProcessToNetwork, probes, DNS hostname or network request activation.

HostClient/PairingClient expose an optional bind callback invoked after
SocketLease ownership and before connect, with cancellation checks around it.
ViewModel supplies the production routing callback to pairing, refresh,
diff, revoke/status; ResumeWorker uses it too and no longer rejects offline
LAN merely because activeNetwork is null. Direct fixture clients remain
compatible. Binding failure closes the owned socket in existing finally path.

NetworkMonitor registers default plus passive unrestricted Wi-Fi/Ethernet
callbacks without INTERNET/VALIDATED requirement. Structured interface/MTU/
address/route stamps detect local link changes; opaque generation tokens fence
ViewModel connections even if cellular default does not change. Both callbacks
are unregistered on stop with generation fencing for queued callbacks.

Seven JVM tests added: longest prefix/family-independent counts, ambiguity,
duplicate routes, strict literal syntax, offline LAN inventory availability,
local loss while cellular remains, and unchanged inventory suppression.
Four instrumented tests source-compile: loopback/no process-route change,
hostname/closed socket refusal, actual network socket bind, and real production
pair/read bind callback order plus closure on injected binding failure.
These instrumented cases have not executed and do not prove actual dual-route
physical behavior. LAN binding must still be verified API31/34/35 and phone.

## Verification

Cwd /Users/mac/Github/zenpi, observed 2026-10-02T01:18:29Z UTC.

- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-lan-routing-20261002`: main/unit/instrumented
  source compilation exits 0, **128 JVM tests across 26 classes pass**.
  Existing icon/safe-call/fixture warnings plus API35 allNetworks deprecation
  warnings remain (API31 floor compatibility, no new SDK dependency).
- `env ZENPI_HOME=/private/tmp/zenpi-lan-routing-20261002 cargo test --offline
  --locked --test remote_lan --test remote_pairing --test remote_subscription_tls`:
  **23 pass**, exit 0 (7/6/10). Rust source unchanged; TLS memory streams,
  not TCP/LAN. Python smoke 34/lifecycle 11/Blueprint 5: **50 pass**.
  Strict Blueprint, cargo fmt and git diff checks pass.
- `./gradlew :app:assembleDebug :app:assembleDebugAndroidTest --offline
  --no-daemon`, cwd android, exit 1 before compilation: /Users/mac/.gradle
  wrapper ZIP lock Operation not permitted. No new APK/device receipt.
- Official Android web documentation attempts failed tool response decoding;
  local API35 android.jar javap confirmed Network.bindSocket(Socket), route
  matches/type/gateway/destination, link properties and passive callback APIs.
  No web-derived claim substitutes for runtime testing.

SHA-256:

```text
LanSocketRouting.kt 49ca35fc5edc6907f0d54267f84e09a476c0a7896512438a9da67082e239ca12
NetworkMonitor.kt 3436c00fe549f7aa7d0de15eff37ed18e42b747bc404832991cd040b0a017ab6
ResumeWorker.kt 048da0399113b7fc92eee551a16168f7d75cd0f544679fa919c5e25f6d5ca2d9
HostClient.kt 4b1bec02a4c3c328a05a03a671b87330992f5484e8b435ce10cf1c76588342e3
PairingClient.kt 97debc9ea07b8aeeff72e830bb2cfb4fe09a9b4604023969d27c62e79c3d325f
PairingViewModel.kt 3ec45ad5d8246bd9ce5d824ae4a67db0e7fa1339a07e98cf107a60732576b964
LanRoutePolicyTest.kt 6d8f578b3254cea91d34860e3682dd75c3f947f6a65d77bb924424537cbe4538
DefaultRouteTest.kt 57640cd694311357b848c782bab04cd0a888d2d360403e46ca58bb2382103d01
LanSocketRoutingTest.kt 4d7c0165d076bd6b84e0241a2b8f32dd363b48495784049b526363834bf64bf1
```

No nested agents, permission bypass, commits/push or historical evidence edits.
Residual: OS VPN lockdown can deny LAN binding; routed LAN behind a gateway
uses default fallback; ambiguity requires environmental resolution. Fresh
device/socket/cancellation/Keystore lifecycle, physical camera QR/same-WiFi
sync, full stream delta/backpressure, interactive PTY/bulk, independent-worktree
quotas 1..64 and release/migration remain required. No complete/blocked claim.
