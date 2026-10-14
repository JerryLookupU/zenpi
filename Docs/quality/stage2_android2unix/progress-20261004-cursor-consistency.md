# Event Cursor Journal Consistency

Provisional S2-033/S2-050/S2-053/S2-056 progress, 2026-10-04. Previous turn
repaired identity-bound revocation cleanup, so it was progress. This turn
inspected execution skill, current dirty integration, Blueprint, Rust event
projection/replay, Kotlin snapshot/replay/subscription/cache and commit policy.
No nested agents, unrelated reset or checkbox promotion.

## Repair

Android validated event and journal counters individually but allowed an event
cursor above its durable journal sequence. Rust events::cursor counts physical
durable turns; every turn occupies a journal record while private events and
branch records consume no transcript event sequence. An event cursor greater
than journal sequence is therefore contradictory, even though equality is not
required and older paged event cursors may be much lower than journal sequence.

HostProjectionCodec now enforces this bound in initial/replacement snapshot
decoding and replay terminal validation before applying any frames. Gap
snapshot completion retains its existing exact state/terminal matching check.
ProjectionCacheCodec encoding and v1..v4 decoding share this snapshot boundary,
so impossible state cannot become a restored or newly encoded cursor/display.
SubscriptionReducer refuses it before replacing its projection/completion state.
No cursor is clamped, counter coerced, partial page published or host journal
rewritten to make the state plausible.

Three new JVM tests cover zero/lower/equal/max-u64 snapshot bounds, replay and
subscription refusal without replacing committed projection/cache, and cache
encoding plus all four supported restoration versions with unchanged input.
New Rust test checks physical cursor/journal bounds after private records,
32 turns, journal recovery, tree enable/alternate branch selection and recovery.
Current Rust-to-Kotlin wire fixtures still pass with non-equal real counters.

Initial JVM regression found an old scanner fixture using journal 5/event 7;
its display was correctly refused by the tightened cache decoder. The fixture
now uses journal 9/event 7 while retaining the same scanner authority/display
assertions. A large-page replay fixture also now advances journal sequence with
its generated durable turns instead of inventing event cursor above journal 9.
Production validation was not relaxed. First manual patch attempt failed
context verification without applying edits; subsequent patch applied cleanly.

## Verification

Commands ran from /Users/mac/Github/zenpi against current dirty integration.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-cursor-consistency-verified-20261004:
  exit 0; production/unit/instrumented Kotlin source compilation; 466 ordinary
  JVM tests; current Rust read fixture and explicit rename/P256/worktree/session
  lanes pass. Existing warnings remain. Instrumented assertions compiled only.
- rtk cargo test --locked --offline --test remote_events --test remote_faults --test remote_subscription --test remote_protocol --test remote_frame --quiet:
  exit 0; 55 pass, 1 ignored real-loopback timeout test across five suites.
  This is deterministic boundary/replay evidence, not real socket/LAN acceptance.
- Python Stage 2 discovery: 132 pass. Lifecycle verifier: 21 pass.
- Rust format, git diff --check and strict Blueprint pass; 36 rows remain [ ].
  APK verify-only still refuses missing provenance. No unchanged restricted
  Gradle/socket retry or old APK install.
- Debug host rebuilt after Rust tests; separate verify-only passes.

```text
HostClient.kt 6c41d510fd0e50078e9639c0d469ceb4464f1b857520ee48f5db19482769e54b
ReplayTest.kt 006125ca02c234e2484c7df6a5b44d8b188b47723ecb45b4991ba602e29d85cb
HostProjectionTest.kt 35b3ebd599f7cb065c2a7d81bb332ea4bf1a6cbc42ec1d98076e7aafb290b9da
ProjectionCacheTest.kt 5e3c9b37862ddac122a918d01e508834a61dc2aef5433d86542fc9a08e8509df
ScannerAdmissionTest.kt d3834b91ae84b7d5563896158ce5a761bbd058bd29c377448b445b71da4fc9a1
remote_events.rs d42a2ccc2c391365e6f98c4cc5a88ca716cc39d625bcacce7b1c361a7c9d4c9a
main.jar 6d5959ca6cfe21e51996acf4b59fce1f4dde9e11ff1e365f091c0a2d903e5c58
unit.jar 2a06663fc58d3da438ea50f1b8dd164457632148b852d937f3b9688a0a0351d4
instrumented.jar a158d058233870c485602549c2bf234b63b42c2238f09ab48bf0796460f5aa17
debug-host-receipt 2f82d0d959ceb3469987c9ed8db9e83b3a79165616050f833a9b130babb51739
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active. Current APK, physical QR/direct-LAN bidirectional sync and
recovery, connected API lifecycle matrix, full parity and signed release remain
incomplete or unverified. This counter consistency repair does not accept a
whole Blueprint item, prove transcript completeness or replace device evidence.
