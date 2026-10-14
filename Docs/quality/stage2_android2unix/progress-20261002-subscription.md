# Bounded Foreground Transcript Subscription

Provisional S2-020/S2-033/S2-036/S2-038/S2-053 progress. Previous goal turn made
verified local-reset and cache progress. This turn adds server-driven durable
transcript delivery on one authenticated request rather than repeated phone
TLS reconnects during the bounded subscription. All 36 authoritative Blueprint
rows remain unfinished. No nested agents, commit, push or acceptance mark.

## Production Path

Rust strict schema and Android FrameCodec accept a capability-gated subscribe
message: bound project/session/cursor, limit 1..128, duration 1..5 seconds.
workspace_list advertises availability. PairingHost handles it after real
device authentication, delegates each internal sample to existing HostAdapter
Resume dispatch, and rechecks current device before each output. Both attached
desktop mailbox and shared owned pool retain normal Read ACL/session checks.
The handler emits existing event/bulk-snapshot batches plus correlated replay
responses with subscription_done, finishing on deadline/tail/gap/count limit.
It samples every 250 ms with no persistent task, queue or parallel Agent.

The entire subscription has one seven-second socket deadline, not a refreshed
deadline per frame. Frame/byte bounds (128/196608), socket closure and existing
listener connection cap bound slow/disconnected peers. No ACK is invented.
Busy/unauthorized/output errors stop; no provider or mutation retry occurs.

Android HostClient opts in only with the advertised capability and foreground
ViewModel, after ordinary snapshot/replay. It receives all batches on the same
TLS exchange, using strict SubscriptionReducer and existing replay identity/
cursor checks. Changed projection is saved before display publication; failed
saves leave the prior display/cursor, and the next request ACKs only saved data.
Generation/cancellation/network/background/local reset remain fences. Empty
unchanged heartbeat batches do not rewrite encrypted cache every 250 ms.

This is a first bounded transcript subscription, not the full final streaming
design. Host source sampling is still polling internally. Provider text deltas,
approval/PTy/workspace state are still list-refreshed. Foreground currently
starts a three-second subscription per read opportunity, with its existing
two-second interval afterward. WorkManager remains read-only and unsubscribed.
No infinite background keepalive or physical LAN acceptance is claimed.

## Verification

- Rust: `ZENPI_HOME=/private/tmp/zenpi-subscription-tests-20261002 cargo test
  --offline --locked --test remote_headless --test remote_subscription
  --test remote_events --test remote_protocol --test remote_faults` passes
  **23 tests** across five suites. Includes new durable SessionStore turns
  pushed under one request, actual Agent/ProjectOwnerPool/DeviceRegistry adapter
  replay, revoked-device refusal, strict duration/routing, tail/gap/count
  overflow, output BrokenPipe and existing fault/privacy/replay regressions.
  The adapter fixture uses an idle production owner/backend, not a live provider
  call. It does not stand in for real listener/TLS/Android execution.
- `cargo check --offline --locked` passes, 19 existing library warnings plus
  the existing vendored crossterm warning. `cargo fmt --check` passes after
  correcting module order; `git diff --check` passes.
- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-subscription-final-20261002` passes: all main Kotlin/Compose
  sources, all JVM sources and all instrumented Kotlin/Compose sources compile.
  All **67 JVM tests** pass across 18 classes, four new SubscriptionTest cases
  covering contiguous batches/completion, malformed cursor/completion atomic
  refusal, bounded empty pages and strict wire routing/duration.
- The new source-check script reuses read-only cached Kotlin 1.9.24, Compose
  1.5.14, Gradle 8.11.1 transformed APIs, API 35 android.jar and JDK 17. Writes
  only the supplied output directory. No Android framework stub is included
  on the JVM runtime classpath. It is deliberately not Gradle, resource/manifest
  merge, DEX, APK/signing, emulator execution or a release gate replacement.
  Seven existing source/test warnings remain. Missing cached inputs fail closed.
- 44 Stage 2 Python tests pass. Strict Blueprint validation passes with all
  36 rows `[ ]`. Historical APK/LAN receipts remain unchanged.

Final direct-source artifact SHA-256:

```text
main.jar 787937ca52ce952e4a57994e796c4b8dfceaaabec5aff569a3588701b2fdc14e
unit.jar 9a84ef373190ca75e209a6d6b39cd9857bd403a9acf44b86f906a761db3570f3
instrumented.jar b48c8c87a1cc6e3f25d98aa4a2be969127f7db0a3d7792ca7b53c55ea14f29b7
```

Source digests at verification:

```text
src/remote/subscription.rs a36a06a32ddb997be2cdbe658ab544ad9f08b75f79d3747fd7c5cd8bf14d501f
src/remote/protocol.rs e901cfef8ff7b5c4170e7f11159824ed64b88f249b2e08f7bcf112f3f7c3b1d5
src/host_pair.rs d46fd41a611b1ff32ffa9c01f7c3de19c94f312e5048f76fbe073903ac405889
SubscriptionReducer.kt 3020c316926c8c1d31d95ba6266589f22757c5d1c1168558ee310228fbb5112e
HostClient.kt bfabfbfb2fe4ac3ea91e02d56a7c3038c585298c7a429973102bd03f8417f11b
PairingViewModel.kt c11d1ce84dd336ccedf1c977a6a7ae29b35973fd1f76c9e257868c204d705301
```

This note is provisional source/test evidence, not a checksum-valid Master
acceptance receipt. All observed command sessions terminalized. No device run
was restarted or old APK relabeled. Gradle external lock/device/socket
permissions remain the previously observed external acceptance limits.

Next: build fresh APK/host artifacts and test one authenticated TLS subscription
while desktop turns complete, cancellation/half-close/revocation and process
restart after partial delivery. Then extend live event classes/backpressure
without inventing durable cursors for transient text. Full phone camera QR/LAN,
worktree scheduler 1..64, complete PTY/bulk, migration/uninstall, signed release
and security/privacy review remain within the unchanged objective.
