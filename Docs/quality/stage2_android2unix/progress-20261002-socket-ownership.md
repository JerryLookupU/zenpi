# Terminal Android Socket Ownership

Provisional S2-032/S2-038/S2-053 progress. Previous goal turn verified pairing
send-time freshness. This turn fixes a raw-TCP/TLS socket replacement race in
both Android production connection paths. Full QR/LAN/native Android/Rust
desktop objective remains active; all 36 Blueprint rows `[ ]`. No agents,
commit, push or historical device receipt changes.

## Changes

PairingClient and HostClient previously used AtomicReference<Socket?>. A
cancellation/deadline close could getAndSet(null) the raw socket while TLS
wrapping was in progress, followed by active.set(tls) publishing a later wrapper
after the close had already fired. SocketLease now marks close terminal under
the same ownership lock used for install/transfer. A late raw socket or TLS
wrapper is refused and closed, not adopted. Valid transfer retains raw until
its owning TLS wrapper closes. No premature raw close breaks normal TLS.

Socket close runs outside the short ownership lock. Close failure still leaves
the lease terminal. Finally blocks use nested try/finally so raw.close runs
even if owned close throws. TLS1.3, certificate pin, zenpi/1 ALPN, challenge,
15-second deadline, request deduplication and commit/ACK rules unchanged.

## Verification

Observed final source 2026-10-02T00:33:12Z onward; dirty worktree preserved.

- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-socket-lease-verified-20261002`: main/unit/
  instrumented Kotlin source compiles; **101 JVM tests pass across 22 classes**.
  Eight new SocketLease tests cover close-before-install, cancellation during
  TLS wrapping, valid transfer/raw lifetime, foreign transfer/double install,
  close-error terminality with suppressed rejection diagnostics, 100 concurrent
  close/transfer races with joined worker threads, actual unconnected java.net
  sockets and cancelled coroutine late transfer. Tracked TLS wrappers are
  lifecycle fixtures, not Android SSLSocket runtime or measured LAN deadlines.
  Existing Kotlin warnings remain. No Gradle/DEX/APK/device claim.
- `env ZENPI_HOME=/private/tmp/zenpi-socket-lease-20261002 cargo test --offline
  --locked --test remote_subscription_tls --test remote_subscription --test
  remote_pairing --test remote_auth`: **34 pass** (9/10/6/9). Existing library
  and vendor warnings remain. Rust source unchanged this turn. TLS suite uses
  encrypted bounded in-memory streams, not actual sockets.
- 44 Stage2 Python tests, strict Blueprint validation, cargo fmt check and git
  diff check pass. All observed command handles completed.

Source SHA-256:

```text
SocketLease.kt 5a0ad80a86faceb865a6e10953b58ab4f01519ecff5bef9cccb30726ca14e7e5
HostClient.kt f00e679a6d54cc18eff18db879c1797bc692744e5d195494a8a1d37c9aff7ca5
PairingClient.kt a1aeee707e19f01826500359b45c9bddfd89847b0501eb51f7d2ce78af28fb6a
SocketLeaseTest.kt 0033f2dbc8fef729cf6eed8f8efb334b9769584f6006924ef962441a86d412a0
942e2b039353a48970fcddcdea6809337c1074f565d95f728acdf45d28a0ed0a main.jar
6c651db651ea86d17e7bbd1204853644ea7528b8e7735c84ce7f6cf4303d6a98 unit.jar
ce5b4ebf88a14154038ff6bc57eacef75488b7cb2cc8ff8590f39659395a038c instrumented.jar
```

Not Master acceptance. Fresh APK connected cancellation/timeout/pairing,
subscription/cache/reset races, physical camera QR/same-WiFi LAN, real socket
shutdown/deadline matrix, full stream-delta backpressure, executable independent
worktree scheduling 1..64, interactive PTY/bulk, API31/34/35 Keystore/lifecycle,
migration/uninstall and signed release remain required. These ownership tests
improve the real connection path without substituting a narrower end state.
