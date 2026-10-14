# Stage 2 Subscription Delivery Refusal

Recorded 2026-10-04. Partial S2-020/S2-032/S2-038/S2-052/S2-070/S2-072
evidence only. Previous goal turn was progress (owner-race refusal fidelity).
Current execution skill, dirty worktree, subscription delivery, Blueprint and
APK provenance were inspected. No nested agents/reset or unchanged denied
Gradle/ADB retry. All observed command sessions reached terminal state.

## Repair

Production host_pair routes subscriptions to serve_authorized. Its per-frame
registry check previously closed with I/O error when authorization became
unavailable, even if the pending frame was a typed refusal. Android then saw
a transport error rather than the explicit authority fence.

The delivery check now substitutes one correlated Failed/null-result frame:
Unauthorized for stale/revoked devices or the registry's typed error (Busy for
lock contention). It then returns a terminal error so no later queued event,
metadata or successful cursor receipt can escape, even after grant restoration.
Partial writes and receipt-write failure remain terminal, not retried.
This does not recall bytes admitted before the registry check, nor guarantee
receipt delivery through a broken socket.

Encrypted-pipe tests now require the exact refusal on revocation and grant
revision change. A second flush gate revokes after a transcript event but
before its successful page receipt; the next frame is null-result Unauthorized,
not a cursor commit, and the durable host journal stays unchanged by refusal.
The existing client reader buffers events until a validated successful receipt.

The Rust/Kotlin fixture also invokes actual serve_authorized with the real
device registry after grant change/revocation. Kotlin decodes both refusal
frames and verifies zero publication/session recovery, unchanged durable
transcript/cursor and no old workspace/prompt/approval authority. These are
production function/registry and encrypted-pipe tests, not production TCP/LAN
or Android SSLSocket/Keystore evidence.

## Verification

Commands ran from /Users/mac/Github/zenpi:

- Subscription + encrypted-pipe suites initially 63 pass; after adding the
  event/receipt race, encrypted-pipe suite 45 pass.
- Subscription/events/auth/security/frame suites: 63 pass, 1 ignored.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-subscription-delivery-20261004: exit 0; main/unit/
  instrumented source compile, ordinary JVM tests and every invoked cross-
  language lane completed, including real-owner prompt and worker cancel.
- Independently reran current ordinary JVM classes: 591 pass. Current Rust
  wire class alone: 7 pass with fresh fixture (no assumption-skipped new gate).
- Python Stage 2 tools: 159 pass; lifecycle fixtures 25; Blueprint fixtures 5.
- cargo fmt --check, git diff --check, strict Blueprint: pass. All 36 items
  remain [ ], no [_]/[x]. APK verify-only still refuses missing provenance.
- Debug and release rebuilt by guarded host builder; both verify-only pass.
  Release codesign --verify --strict passes. This is current source attribution
  and signature integrity, not fresh two-location reproducibility or acceptance.
  Historical reproducibility receipts remain historical and were not rewritten.

```text
subscription.rs a195703357b98c3ead179a2b432a7f3fe7ee220caa053a9987c0751c071516da
remote_subscription_tls.rs 24bea1b8c4a7137995074c8d1e914fa236d66f36701e857fe7eaa7edc2091b97
remote_android_interop.rs 8d5aa001f112ab48b9f98ca0c1e808061cbb0f4c489716b1f8a91ec1f14a01a1
RustAndroidWireTest.kt 5bdda9a58c69c2fc24edc98981442cfa86ff2462e102f4b3244a36da9ca64fc3
unit.jar fabb91bf7f65b2a683e97c089c4740690e9be438362f75245d5cbcfae8502fe0
android-sync-wire.json 32feb0264288b438f3ed7aab2a8b844a1a3617bfea47d55acf088eb7a2b304b0
debug-binary 70335c71e5edf128f8183ec87a65e26633ce81126ae355e72c13c23e333b85e6
debug-receipt 311e6428f761df31c3c0c6eb812c4e3e0dd4a1a83946c43648dde99be7fba0c3
release-binary 0368bb32c96052364f93afed3436ab213aa1195441e637eb5ddaeba9c808ff95
release-receipt 8d2f11c03cd925a282a4baed0442e71ad7967f346aec97c8581b27ad01623b5f
host-inputs 329babfd49d88d1a10ecf69ee61e8e3211ad4b5a6ad315ba4b79fb10fe39e426
```

Goal active/incomplete. Fresh APK/signed install/upgrade, physical camera QR
and direct-LAN pairing/sync/recovery, connected API31/34/35, current independent
release reproducibility and full-library gate closure remain required.
