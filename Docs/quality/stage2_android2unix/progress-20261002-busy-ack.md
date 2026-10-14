# Busy ACK Does Not Block Foreground Display

Provisional S2-020/S2-032/S2-033/S2-036/S2-038 progress. Previous goal turn fixed
soft-wrapped terminal redaction. This turn fixes a concrete request-order gap:
Android ACK preceded Resume/Subscribe and Busy ACK terminated the read before
the existing Busy foreground display fallback could run. Full QR/LAN/native
Android/Rust desktop goal remains active, all 36 Blueprint rows `[ ]`.

## Changes

HostClient converts a correlated failed response into HostRefusedException with
typed code, preserving existing user-facing message. Existing workspace writes
still throw WorkspaceRefusedException. EventAckCodec.acknowledge invokes the
same production exchange and validates successful receipts; only typed busy
defers. It returns false without retry, cursor mutation or invented confirmation,
allowing subsequent subtab/read/subscription to proceed. Caller only sends
persisted prior cursor; later saved cursor can be ACKed on a later connection.
Unauthorized, owner mismatch, invalid request, generic matching-message errors,
IO/timeout, cancellation and malformed successful receipts continue to fail.

Three new JVM tests verify one-call deferral/later successful confirmation,
negative exception matrix and actual framed Busy ACK followed by a read on
the same InputStream via production ResponseReader. Existing Rust owned adapter
test holds actual Agent lock, observes Busy ACK with unchanged durable bytes,
then reads authorized subscription view with matching listing/subtabs revision
without releasing that lock. Host ACK/refusal semantics are not weakened.

## Verification

- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-busy-ack-20261002`: main/unit/instrumented Kotlin
  source compiles; **113 JVM tests pass across 24 classes**. Existing Kotlin
  fixture/icon/safe-call warnings remain. Cached source compilation is not
  Gradle/DEX/manifest/APK, Android TLS/Keystore, camera or connected lifecycle.
- `env ZENPI_HOME=/private/tmp/zenpi-busy-ack-20261002 cargo test --offline
  --locked --test remote_headless --test remote_subscription --test
  remote_subscription_tls`: **20 pass** (1/10/9). Owned adapter fixture uses
  actual idle OpenAI-compatible Agent and signed registry; provider not invoked.
  TLS suite is bounded encrypted memory stream, not TCP/LAN. Existing Rust/
  vendor warnings remain. Rust production source unchanged.
- 44 Stage2 Python tests, strict Blueprint validation, cargo fmt check and git
  diff check pass. All observed command handles completed; dirty worktree
  preserved. No agents, commit, push or historical device receipt changes.

SHA-256:

```text
FrameCodec.kt 9ef4babd359e688f732c02d9b934291e42f044ca180ccf301845617a8d3f58a4
EventAckCodec.kt cf3b31f62e31f76837ab7ccfa50c1b588a08f5ad2df8af67cf6338faecb002df
HostClient.kt 4eb264a6808af67bc51894e9d4d17d7ba692aeb8869e36cea4a08152f4a84468
EventAckTest.kt fe3aa4d4c0dbb2577a40afb0984f312b4c0f2129691160f6e7be23389479b8b8
tests/remote_headless.rs 54e113533ee027dafbc84c2181ced7fe50effd96f31d3b694570b8853e11feea
fd8c22cb367a0f2edf43118b8f37032b295b1594d46c7cf463c3d5645e60988b main.jar
9880a5b180a0415fcf1cf501e9dc7fe5170b456926e86a0ace07a43117dbf27a unit.jar
ce5b4ebf88a14154038ff6bc57eacef75488b7cb2cc8ff8590f39659395a038c instrumented.jar
```

Not Master acceptance. Fresh APK running-provider foreground subscription,
Android TLS pin/cancellation/pairing/cache/reset, physical camera QR/same-WiFi
LAN, socket deadline/shutdown, full stream-delta backpressure, executable
independent-worktree scheduling 1..64, interactive PTY/bulk, API31/34/35
Keystore/lifecycle, migration/uninstall and signed release remain required.
Deferral fixes the actual read chain, not a substitute end state.
