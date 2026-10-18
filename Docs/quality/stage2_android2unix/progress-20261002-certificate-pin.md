# Android Certificate Pin Verification

Provisional S2-012/S2-032/S2-052 progress. Previous goal turn fixed terminal
socket ownership. This turn verifies the Android trust callback with an actual
Rust-generated certificate and adds fail-closed configured-pin validation.
The full QR/LAN/native Android/Rust-desktop objective remains active; all 36
Blueprint rows `[ ]`. No agent, commit, push or historical receipt alteration.

## Changes

PinnedTrustManager rejects any configured pin other than exactly 64 lowercase
hex digits with CertificateException at construction, and decodes the pin once
instead of during each callback. Exact DER SHA-256 comparison remains constant
time. Certificate chain must contain exactly one certificate and validity is
checked. Public constructor still uses current device Date; an internal clock
constructor enables deterministic expiry/not-yet-valid tests. Both production
clients keep the public constructor. No bypass or alternate trust store added.

Six new PinnedTrustManager tests embed only public DER from the existing local
Rust HostIdentity fixture (/private/tmp/zenpi-product-pair-20261001/identity.json).
Only its certificate field was read for the fixture; no private key copied.
Tests parse real JDK X.509, verify self-signature and zenpi.local SAN, accept
correct DER fingerprint, reject all 64 one-character mutations, reject a
public-key-only fingerprint, reject null/empty/extra chains, reject expired and
not-yet-valid dates with boundary acceptance, reject malformed configured pins
and refuse client trust. No mock X509Certificate methods supply the evidence.

## Verification

- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-pin-check-20261002`: main/unit/instrumented Kotlin
  source compiles; **107 JVM tests pass across 23 classes**. Existing Kotlin
  icon/safe-call/instrumented fixture warnings remain. This is cached-source
  verification, not Gradle/DEX/manifest/APK, Android TLS/Keystore, camera or LAN.
- `env ZENPI_HOME=/private/tmp/zenpi-pin-check-20261002 cargo test --offline
  --locked --test remote_pairing --test remote_auth --test
  remote_subscription_tls`: **24 pass** (6/9/9). Existing Rust/vendor warnings
  remain. Rust source unchanged; TLS suite uses encrypted bounded memory streams.
- 44 Stage2 Python tests, strict Blueprint validation, cargo fmt check and git
  diff check pass. All observed command handles completed. Dirty worktree
  preserved; no new APK/physical phone/connected network test claimed.

SHA-256:

```text
PairingClient.kt ddcd887209c7d74a9e9bb4c5549353a8bc930a8c79cc423a153c00ef2c5ad88d
PinnedTrustManagerTest.kt 865c8afd60eb8f7ad4d4aff7df1daa527f9582f10f7db36699e4f61a66c7025f
413cc1cc902c8dcc206078c4f2866738b39c5d36a135d54cb49c76aca5eb9ea5 main.jar
3cfe2ecc1c21e4dd5462f3a00f67b7a3cbf30249326300828f727f1ed7ebcf97 unit.jar
ce5b4ebf88a14154038ff6bc57eacef75488b7cb2cc8ff8590f39659395a038c instrumented.jar
```

Not Master acceptance. Fresh APK actual Android TLS pin/cancellation/pairing,
subscription/cache/reset races, physical camera QR/same-WiFi LAN, real socket
deadline/shutdown matrix, complete stream-delta backpressure, executable
independent-worktree scheduling 1..64, interactive PTY/bulk, API31/34/35
Keystore/lifecycle, migration/uninstall and signed release remain required.
Callback unit evidence does not replace the requested physical communication.
