# Encrypted Subscription Verification

Provisional S2-020/S2-032/S2-052/S2-053 progress. Previous goal turn extended
subscription display updates and verified owner/ACL/cursor/cache behavior.
This turn adds real rustls encrypted-stream evidence without claiming to
overcome the restricted TCP/Android environment. Full QR/LAN/desktop objective
remains active; all 36 authoritative Blueprint rows stay `[ ]`.

## Shared Production Output

Extracted the existing per-frame current-device check and write_frame call to
subscription::serve_authorized. PairingHost uses it after authentication and
setting its fixed seven-second socket deadline. It delegates to the same view/
replay functions and does not change registry, transcript or side-effect
semantics. Tests call that same production output function, avoiding a duplicate
test-only authorization wrapper. This is a scoped extraction, not a replacement
listener, transport or protocol handler.

Transient-only response now includes transcript_events_v1 cursor scope.
Android verifies it and refuses missing/journal scope. Existing strict cursor,
display and no-cache/no-ACK invariants remain unchanged.

## Actual TLS, Explicitly Not LAN

New tests/remote_subscription_tls.rs uses actual rustls TLS 1.3 client/server,
actual generated HostIdentity certificate, trusted certificate roots, ALPN
zenpi/1 and real signed DeviceChallenge authentication against the durable
DeviceRegistry. It uses actual Agent/SessionStore/ProjectOwnerPool/HostAdapter
with an idle compatible backend; it never invokes a provider. The byte carrier
is bounded channels with fragmented reads (31 bytes) and writes (257 bytes),
three-second read/write failure bounds and a bounded 64-chunk queue per direction.
No socket is opened; no listener, QR scanner or Android implementation is hidden
inside the fixture. The fixture handshake is not PairingHost's full handshake.

Seven cases passed:

- New real durable turn arrives encrypted exactly once, contiguous sequence,
  unchanged authenticated connection and correlated final batch/cursor.
- Holding the actual Agent lock yields authorized transient-only display with
  no journal advance, then releasing it delivers the durable turn on that stream.
- Wrong certificate fails before device authentication/subscription dispatch.
- Invalid signed challenge proof fails before subscription dispatch.
- A modified encrypted application record fails terminally, no successful batch.
- Dropped client receiver terminates the authorized production subscription.
- Device revocation after the first encrypted batch prevents a later batch.
  A bounded flush gate makes this timing deterministic, not a sleep assumption.

Server thread handles join on normal results and unwind. There is no detached
worker/retry or unbounded channel. Test-only keys/metadata live in owned temp
directories and are removed after joined completion; no credential is printed.
This does not prove socket slow-peer deadlines, listener shutdown, phone TLS/
Keystore/pinning, actual LAN delivery or screenshots.

## Validation

Observed timestamp 2026-10-01T23:56:42Z, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b, dirty worktree preserved.

- `ZENPI_HOME=/private/tmp/zenpi-subscription-tls-tests-20261002 cargo test
  --offline --locked --test remote_subscription_tls --test remote_subscription
  --test remote_headless --test remote_events --test remote_faults
  --test remote_protocol`: **33 passed across six suites**, final seven TLS
  cases included. Initial TLS three/five-case iterations also passed before
  expanding the adversarial matrix and deterministic revocation gate.
- `cargo check --offline --locked`: passes, existing 19 library warnings and
  one vendored crossterm warning. Format and whitespace checks pass.
- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-subscription-tls-source-20261002`: all production,
  JVM-test and instrumented Kotlin/Compose sources compile, **69 JVM tests pass**.
  Adds negative missing/wrong transient cursor scope to existing cases. This
  does not build an APK or execute instrumented/Keystore/UI tests.
- 44 Stage 2 Python tests pass; strict Blueprint validator passes with
  36 unfinished rows. No Master checkbox is advanced.

Source SHA-256 at verification:

```text
src/remote/subscription.rs 5c7c678e2614baf93a3eb90271d34f4c1e5041d999f83097caef7f4f77de843a
src/host_pair.rs f6103007f454e8346e758411165ad1eb33e24c3139ff292d18ad42626e85725d
tests/remote_subscription_tls.rs 12168df99c44400aba63dc11862c972f7eed50d219981238fcbc9838fedebd81
SubscriptionReducer.kt d70d9dc49c647c90e40de04db6da090bed8ad03c9dd56b682a9c4d20b4fbc00e
SubscriptionTest.kt 8f095c873ab83b6f3c4a17d812e5bede866625da30600f6f2268d2a2ca2cebf2
main.jar aa577576219c8bae8b9f6f946a07526dcc84d1f122207262f697648fdd3d3788
unit.jar 6681d92ad3301706a652563b243e7607e1b04267d87c3a1f9c4261e29b66dea7
instrumented.jar b48c8c87a1cc6e3f25d98aa4a2be969127f7db0a3d7792ca7b53c55ea14f29b7
```

Jar paths are under the direct-source output directory above. This progress
note is not a checksum-valid Master acceptance receipt. No nested agent,
commit, push, new APK or substituted historical LAN receipt. All command
handles terminalized before turn close.

Remaining: real socket/LAN and fresh APK subscription tests, phone camera QR,
supported API lifecycle/reset/Keystore runs, full callback-driven events and
backpressure, executable 1..64 worktree scheduling, interactive PTY/bulk,
migration/uninstall and signed release/security gates. The existing restricted
environment still prevents external Gradle locks and socket bind; meaningful
source and encrypted-stream verification progress remains possible.
