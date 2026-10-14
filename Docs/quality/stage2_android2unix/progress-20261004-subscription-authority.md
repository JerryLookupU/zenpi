# Stage 2 Subscription Authority Recheck

Recorded 2026-10-04. Partial S2-020/S2-032/S2-038/S2-052/S2-072 evidence,
not Master acceptance. Previous product turn was progress (device confirmation
scope); the intervening RTK installation check was environment verification.
Current skill/Blueprint/worktree and APK provenance were inspected. No nested
agents, dirty-worktree reset or unchanged denied Gradle/ADB retry.

## Repair

The subscription's deliberate second metadata sample distinguishes an owner
race from an authorized session switch. It previously ignored a newer explicit
Unauthorized/RevokedDevice result, retaining OwnerMismatch. Both the metadata
and replay race paths now share a small classifier preserving these authority
refusals and SessionChanged. Successful or inconclusive Busy/InternalError/
OwnerMismatch rechecks still return OwnerMismatch, not success or recovery.
Both paths stop with one correlated null-result failure and never publish the
sampled preview or advance the transcript cursor.

This improves typed refusal fidelity, not a claim that the previous
OwnerMismatch granted controls: Android already treats it as authority loss.
Production serve_authorized still checks registry currency before each frame
and can close without a typed receipt when registry currency is lost.

Two Rust tests cover both paths, authoritative errors, inconclusive samples,
sample/dispatch counts and absence of data. The current Rust fixture emits the
actual subscription classifier's failure frames. A new Kotlin test decodes
them through ResponseReader, verifies zero publication/session recovery and
display-only policy preserving durable transcript/cursor. Controlled dispatch
results do not prove a production registry/TCP race.

## Verification

Cwd /Users/mac/Github/zenpi. All observed command handles terminal.

- rtk cargo test --locked --offline --test remote_subscription: 19 passed.
- Rust subscription/events/auth/protocol/frame suites: 68 passed, 1 ignored.
- remote_subscription_tls: 43 passed; fragmented encrypted pipes, not TCP/LAN.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-subscription-authority-20261004: exit 0; main/unit/
  instrumented source compiled, 591 ordinary JVM tests passed, every invoked
  Rust/Kotlin interop lane completed, including prompt and worker cancellation.
- Stage 2 Python tools: 159 passed; lifecycle fixtures 25; Blueprint fixtures 5.
- cargo fmt --check, git diff --check, strict Blueprint: passed; 36 [ ], no
  [_]/[x]. Android verify-only: missing APK provenance (exit 1).
- Debug host rebuilt and attribution verified. Release verify-only correctly
  rejects stale host source inventory after the Rust edit; old release bytes
  and historical reproducibility receipts were not rewritten or promoted.

```text
subscription.rs 5581337f368588450ecb6e6845c81920de5e50a5ad63a4bd056cdab4ce394ced
remote_subscription.rs 96e551ca8b32eb89e6efb20fdff0dae322ce1a5fc4c149955dce1ba683b65139
remote_android_interop.rs 2c3c8e93226568ce1c7c92655b9e91e7ae3b5b85fc7b19789d291e030c3d75db
RustAndroidWireTest.kt c8d9eb67b69b02d76faa82368734f2508db7c298f0a76b492246b54cff410a7a
main.jar abfe729defd0c3cd22d3d281a9e28c4bb24c2b19cd6856c08ebf8a5f50cc56c0
unit.jar 8829425b4e41babbfc458079d56d7d856264307db7c2bcf4d4e840086c469834
instrumented.jar b1f668eb5d59377c9047dea40ee4c187f4b67d9d40c99c9bbc796b655bd2d65c
android-sync-wire.json 485d8729d64a8c6d03c23d868adc1f402f69ff9694de6d36771dd00ad3acd0cd
debug-binary b0b486ec9b04a32a7201d802225c7f3db4f50653cff45c374bad7c3eddbff835
debug-receipt 243ea6a50272bea36f5c820b012c1ec8d2655056a84e17e6adc60e8303075552
```

Goal remains active/incomplete: fresh APK/signed install/upgrade, physical
camera QR/direct-LAN pairing and synchronization/recovery, API31/34/35 connected
lifecycle, current reproducible release and full-library gate closure remain.
RTK command filtering works here; its global tracking database is not writable,
so no token-savings measurement or automatic hook configuration is claimed.
