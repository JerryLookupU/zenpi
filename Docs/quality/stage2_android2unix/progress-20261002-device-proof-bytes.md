# Cross-Language Device Proof Bytes

Provisional S2-012/S2-013/S2-032/S2-037/S2-052 progress. Previous goal turn
verified real-certificate pin callback. This turn fixes a concrete Android/Rust
device-signature serialization mismatch. Full QR/LAN/native Android/Rust-desktop
goal remains active, all 36 Blueprint rows `[ ]`. No agent, commit or push.

## Changes

DeviceIdentity now uses DeviceProofCodec for pair/authenticate/revoke-status
messages, keeping existing domain tags, field order, SHA256withECDSA and
Android Keystore private key. Gson escapes Unicode U+2028/U+2029 even with
disableHtmlEscaping, whereas Rust serde_json signs literal UTF-8 bytes. This
caused valid separator-containing names/IDs to produce invalid host proofs.
Codec normalizes only those Gson escape tokens, preserving escaped-backslash
pairs so literal backslash-u2028 text stays literal rather than becoming a
separator. No verification leniency or identifier normalization introduced.

Shared android/app/src/test/resources/device-proof-v1.json provides six exact
byte vectors for all three domains: punctuation including HTML-sensitive chars,
quotes/backslashes, actual separators, literal escape text and multilingual
names. Both Rust and Kotlin read this one artifact. Source checker now includes
the standard test resources directory in JUnit classpath; Gradle already uses
that source layout. Two further JVM tests distinguish literal/Unicode names and
verify real P-256 signatures against exact bytes, refusing equivalent escaped
JSON or changed request IDs. Rust adds shared-vector and real signature tests.

## Verification

Observed final source 2026-10-02T00:39:02Z onward; dirty worktree preserved.

- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-device-proof-verified-20261002`: main/unit/
  instrumented Kotlin source compiles; **110 JVM tests pass across 24 classes**.
  An intermediate expanded test compile failed due to local verify() shadowing
  Java Signature.verify; renamed helper and qualified method in final source.
  Initial smaller vector run passed 108; only final 110 includes all tests.
  Existing Kotlin warnings remain. No Android Keystore execution, Gradle/DEX/
  manifest/APK, camera or network acceptance claimed.
- `env ZENPI_HOME=/private/tmp/zenpi-device-proof-20261002 cargo test --offline
  --locked --test remote_auth --test remote_pairing --test
  remote_subscription_tls`: **26 pass** (11/6/9). Shared bytes and Unicode proof
  acceptance/refusal use Rust production signing message/verify helpers with
  actual P256 key. Auth/revocation and pairing expiry regressions retained.
  Existing Rust/vendor warnings remain. No Rust production-source change.
- 44 Stage2 Python tests, strict Blueprint validation, cargo fmt check and git
  diff check pass. All observed command handles completed. Existing historical
  APK/device receipts unchanged; no physical phone/connected TCP run this turn.

SHA-256:

```text
DeviceIdentity.kt 8b40c7c94ee72b732e12228c50389a4487c110a3eac9e606d8733aec548c115e
DeviceProofCodec.kt 2171f6c7a40849448845008493185a01c65e59ccf0b42d665e66c0868d779a00
DeviceProofCodecTest.kt 9749d676188f71c4d72fade47661e4c9052cd15395c43cb9bc57901c2ab92546
device-proof-v1.json e1b6fc37e5b231bc32f3007f79b3dd902ffde8440173beeae7307988d1eb6f9c
tests/remote_auth.rs 39b3b43141dab8944a8c0d34e1f2eedac7c2f3c49e78b96e19d8938d1de37f71
tools/stage2_android_source_check.sh aff12756f8905814b881d2d9e8089d76bf1a05c26cb746d91bdac8b18e1d126b
4bf9507c149932c0f0d76aa9b5e3b273b4bb27573529673a9e278fff872e5932 main.jar
49399fe842d0915199e46f86f0f14325af6d07e6e499bd049aa1a0710bcb3400 unit.jar
ce5b4ebf88a14154038ff6bc57eacef75488b7cb2cc8ff8590f39659395a038c instrumented.jar
```

Not Master acceptance. Fresh APK Android Keystore-generated signature pairing,
TLS pin/cancellation, subscription/cache/reset races, physical camera QR/same-
WiFi LAN, real socket deadline/shutdown, full stream-delta backpressure,
executable independent-worktree scheduling 1..64, interactive PTY/bulk,
API31/34/35 lifecycle, migration/uninstall and signed release remain required.
Byte compatibility fixes move the real authentication path forward without
substituting unit tests for the requested final communication experience.
