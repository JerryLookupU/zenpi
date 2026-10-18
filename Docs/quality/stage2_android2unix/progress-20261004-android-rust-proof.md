# Android P256 Proof Verified By Rust

Provisional S2-012/S2-013/S2-031/S2-032/S2-052 progress on 2026-10-04. Previous
goal turn added current Android-to-Rust mutation/receipt interoperability and
executed it; it was progress. Current execution skill, dirty worktree,
Blueprint, DeviceIdentity/DeviceProofCodec, PairingHandshake and Rust
InviteIssuer/DeviceRegistry/DeviceChallenge inspected. No nested agents,
worker admission, unrelated reset or authoritative checkbox promotion.

## Implementation

Added an explicitly invoked proof interoperability lane to remote_android_interop.
Rust generates a fresh production host identity, one-use invitation and seven
connection challenges. Android parses the exact URI, runs actual PairingHandshake
and signs canonical DeviceProofCodec bytes with a fixture software P256 key.
Rust reads emitted hello/pair framing, verifies the ASN.1 signature, checks SAS,
consumes the invitation and registers default Read/no-project permissions.
The fixture device name contains actual U+2028 plus literal backslash-u2028;
this checks exact signed UTF-8 compatibility, not equivalent parsed JSON.

Android also signs authentication and revoke-status proof messages. Current Rust
DeviceChallenge validates successful authentication, consumed challenge refusal,
foreign host/nonce/device refusal, revoked identity refusal and exact signed
self-revoke receipt. Reusing the revoke challenge or changing the query is
refused. Pair proof substitutions for host/token/request/name fail validation.
No private key is serialized; only public fixture data is passed in temporary
files. JVM phase is bounded to 15 seconds. Ordinary Cargo ignores this lane
because fresh compiled JVM classes are required; ordinary Gradle assumption-skips
its helper. Source checker excludes helper from ordinary count and explicitly
runs it, requiring success after the existing read and write lanes.

The offline handshake receipt is synthetic, as are its prelude frames. Production
Rust verifies the proof and invitation afterward, not a live server exchange.
This is software cryptography/protocol interoperability, not Android Keystore,
TLS/ALPN/certificate execution, camera or LAN acceptance.

## Verification

Commands ran from /Users/mac/Github/zenpi against the dirty integration.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-android-rust-proof-verified-20261004:
  exit 0. Rust read fixture 1 pass/2 ignored; production/unit/instrumented Kotlin
  compile; 456 ordinary JVM tests pass; explicit Rust write lane 1 pass with two
  JVM phases; explicit Rust proof lane 1 pass with Android proof phase 1 pass.
  Existing warnings remain; no connected test execution.
- Initial Rust fixture compilation incorrectly assumed optional Pair fields;
  corrected to the actual required String contract. First compiled proof run
  exposed the fixture's default Android SystemClock call on desktop JVM; fixed
  by injecting monotonic System.nanoTime through the existing parser clock API.
  Production parser/crypto behavior was not weakened. Final full run above passes.
- cargo test --locked --offline --test remote_auth --test remote_pairing
  --test remote_security --quiet: exit 0; 12/8/12 top-level tests pass (32 total).
  Pairing includes a nested isolated child test. RTK aggregates that child as
  33 pass/7 filtered across 4 suites, not 33 distinct top-level cases.
- Python Stage 2 discovery 132 pass; lifecycle verifier 21 pass; Blueprint
  verifier 5 pass. Rust format, strict Blueprint and git diff --check pass.
  All 36 rows remain [ ].
- Host debug rebuilt after Cargo test artifacts changed; current attribution
  verified below. APK verify-only still refuses missing provenance, exit 1.
- No unchanged restricted Gradle/ADB/socket retry, stale APK installation,
  physical scan, connected lifecycle or direct-LAN success was attempted.

```text
remote_android_interop.rs 088353972694e6e6c6d7a73de8596d111717c8ee71c1f63ff1e41d40f3dde5c8
AndroidRustProofTest.kt 5814995e10fcea5e71c9586821c4f5abd4c6fcceb636e9bbcc8dad513a5448ce
stage2_android_source_check.sh 7e36e008b2ef3259e39d1a5bca6e87978a50b7880e6e87975cfdc1eab6379941
main.jar f4a19f7e4072678db59c6dabb5267166ddb4d31950252c9fad9600dcfd5130b5
unit.jar a2f56390b9b28f93c4dfd1d7848b3d6a54f7b92d0dc506c2af4f0f1e5d7ecfbd
instrumented.jar 49185cec7ed267be124e9ab6a9dc3bfbfaffcf0fe3dbf1794b10b32f75a49eba
debug-host-receipt 926b7f01476107dd3a306297bc6da3b7a3ae4ca316afc654dc31a887c71194a2
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active. Current-source APK, real camera QR/direct-LAN bidirectional
sync/recovery, current API 31/34/35 connected matrix, full feature parity and
signed release remain incomplete or unverified. This new evidence does not
supersede earlier full-suite failures or accept a whole Blueprint item.
