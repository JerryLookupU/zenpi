# Network Monitor Restart Identity

Provisional S2-038 progress. Previous goal turn repaired explicit routed LAN
selection. Current inspection found each NetworkMonitor.start created a new
NetworkInventoryState with revision zero. Its first available token was always
1; PairingViewModel ignores a repeated observed token. On another start/Activity
monitor instance, different or recovered availability could be mistaken for
the prior observation. Full QR/LAN/native Android/Rust goal remains active;
all 36 authoritative Blueprint rows `[ ]`.

## Changes

NetworkInventoryState allocates available tokens from one process-wide atomic
monotonic counter. Initial start forces observation even if its inventory
matches; ordinary unchanged callbacks remain suppressed. Available tokens
cannot collide between monitor cycles or instances. Offline observations remain
null. Counter exhaustion fails rather than wrapping/reusing identity. The
existing main-thread generation check still ignores callbacks from stopped
registrations, and ViewModel continues cancelling prior connections on changed
tokens before its bounded reattach policy. No socket/background write policy
or protocol cursor changes.

Three new JVM tests cover same-network restarts/forced observation, a changed
route while stopped plus offline restart, and eight joined threads allocating
800 distinct tokens. Thread exceptions are collected and asserted by the
JUnit thread. NetworkLifecycleTest now stop/starts the actual monitor while
connected, requires a different token and verifies same-session reconnect
before its existing real network-loss/recovery sequence. This instrumented
addition is source-compiled only, not executed.

## Verification

Cwd /Users/mac/Github/zenpi, observed 2026-10-02T01:23:40Z UTC.

- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-monitor-restart-20261002`: main/unit/instrumented
  Kotlin compile exits 0; **136 JVM tests across 26 classes pass**. Existing
  icon/safe-call/fixture/allNetworks warnings remain. No device pass implied.
- `env ZENPI_HOME=/private/tmp/zenpi-monitor-restart-20261002 cargo test
  --offline --locked --test remote_subscription --test remote_subscription_tls`:
  **20 pass**, exit 0 (10/10). Rust source unchanged; TLS bounded encrypted
  memory transport, not physical LAN/TCP. Python smoke 34/lifecycle 11/
  Blueprint 5: **50 pass**. Strict Blueprint, cargo fmt, git diff checks pass.
- No new Gradle/APK attempt this turn; the preceding authoritative wrapper-lock
  permission denial remains unresolved. No old APK or historical receipt is
  promoted to current-source verification. No agents, permission bypass,
  commit/push or checkbox modification.

SHA-256:

```text
NetworkMonitor.kt bf288fa3d0b6079e3801fa5f65c713299c829686bae1f37ed1f353353a8ce163
DefaultRouteTest.kt 2d772b813befd0a8eb8e6d94bfee8c5252d11b8357d18e51f179c6573678ae2e
NetworkLifecycleTest.kt 4eb1897b4fe212e05435c37a979c4af38b3b837af627cfc6173910b29ba709ec
main.jar ef03ab5464236989d7b36736c19ed5863e06155ffbd3b830d94ec02a0a4bfb49
unit.jar 40e9b23048b9b318cd44c9290c2e317e15b88aa5c7564b5df7c8898fb0ccc5b7
instrumented.jar 75e20cd71bb05fc10426e07326390d19c4d8d08573354105b56b18b50df61031
```

Fresh supported-API network restart/socket binding/cancellation/Keystore
lifecycle and physical camera QR/same-WiFi synchronization remain unproven.
Independent-worktree scheduling 1..64, interactive PTY/bulk, full stream delta
replay/backpressure and signed release/migration remain required. This repairs
the lifecycle recovery entry, not substitute completion for the full objective.
