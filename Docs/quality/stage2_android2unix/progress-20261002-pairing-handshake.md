# Android Pairing Send-Time Freshness

Provisional S2-012/S2-031/S2-032/S2-052 progress. Previous goal turn fixed cache
commit/publication freshness. This turn addresses QR invitation lifetime during
the actual Android post-pinned-TLS pairing exchange. Full QR/LAN/desktop/native
Android goal stays active; all 36 Blueprint rows remain `[ ]`. No nested agent,
commit, push or replacement of historical device evidence.

## Changes

PairingClient retains real TLS1.3 certificate pinning, zenpi/1 ALPN, 15-second
absolute close/deadline, socket cancellation and Android Keystore signer. Its
post-handshake frame exchange now uses PairingHandshake, allowing tests to use
the exact production path without replacing pinning or generating a fake APK.

Freshness/cancellation checks occur before hello, after welcome, after validated
matching-fingerprint challenge and after signing immediately before token send.
Previously freshness was checked only before hello, allowing expiry during
delayed welcome/challenge to reach token send. No retry is added. Partial
receipt/disconnect remains unknown and does not resend one-use invitation.
Accepted correlated receipt after local expiry is valid because host commit
can precede expiry; cancellation still fences its publication.

Success requires correlated succeeded response, matching expected device ID,
boolean paired/device_registered both true, and boolean control_available.
Unexpected result fields and contradictory flags fail closed. Inspection of
current src/host_pair.rs showed device_revision in actual successful receipts;
final decoder admits and validates positive u64 when present, while supporting
legacy schema1 receipts without it. An intermediate exact-field draft omitted
that current field and was corrected before final verification. Invalid control
characters in device name refuse before network hello/signing.

## Verification

Observed final source 2026-10-02T00:30:15Z; dirty worktree preserved.

- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-pair-handshake-verified-20261002`: main/unit/
  instrumented Kotlin source compiles; **93 JVM tests pass across 21 classes**.
  Eight new pairing tests use production FrameCodec and PairingHandshake with
  one-byte fragmented input, captured actual outgoing frames, injected clock
  and signer/activity hooks. Cases cover bound one-time request, expiry during
  each pre-send stage, fingerprint/welcome refusal, malformed/failed/foreign/
  unconfirmed receipt, every response truncation, post-commit late receipt,
  cancellation before send and after receipt, signing failure, valid legacy
  response and invalid names. No Android Keystore/pinning/socket/camera runtime
  executed; cached source compilation is not Gradle/DEX/manifest/APK evidence.
- Full attempted Rust auth/pairing/TUI run: auth 9 and pairing 6 pass; TUI 2
  pass/2 fail. Direct PairingHost start fails PermissionDenied/Operation not
  permitted at test line 238. Production binary case fails parsing absent/
  invalid stdout invite at line 327 (InvalidInvite); this command did not
  capture a diagnostic proving its exact child cause, so it remains failed,
  not automatically classified as a successful or verified network gate.
- Final `env ZENPI_HOME=/private/tmp/zenpi-pair-handshake-20261002 cargo test
  --offline --locked --test remote_pairing --test remote_auth --test
  remote_tui_pairing -- --skip
  committed_tls_write_with_busy_delivery_stays_unknown_and_exact_retry_is_idempotent
  --skip production_host_pair_negotiates_and_consumes_invite_over_tls`:
  **17 pass** (6/9/2), two network cases filtered and unverified. QR test renders
  real terminal pixels and decodes exact invite/quiet zone; not physical camera.
- 44 Stage2 Python tests, strict Blueprint validation, cargo fmt check and git
  diff check pass. Existing Kotlin and Rust warnings remain. All observed test
  handles completed. Rust production source unchanged this turn.

SHA-256:

```text
PairingClient.kt 6911c7a391a81b98a27e543841b7200696bcbfefd6b45f02e3eb1dc32fb003e9
PairingHandshake.kt c78382e149e49e5684ae750343217b9edb81f72396e5206eef20f44220072629
PairingHandshakeTest.kt e34969a4cdc2dac3dfd7bf60ea34f8c45b02ecf8605ad523288c5eba41bde76c
354a297dded0659eeaa2ea92757f96afd90968fc71a4b4d9383700207e654696 main.jar
19a84d0caf166a911c4f28cab27234e8da47a99b7bf9f5c595f396d44855257c unit.jar
ce5b4ebf88a14154038ff6bc57eacef75488b7cb2cc8ff8590f39659395a038c instrumented.jar
```

Not Master acceptance. Fresh APK/device pairing/subscription/cache race and
reset tests, physical camera QR/same-WiFi LAN, actual socket deadline/shutdown,
complete stream-delta backpressure, executable independent-worktree scheduling
1..64, interactive PTY/bulk, API31/34/35 Keystore/lifecycle, migration/uninstall
and signed release remain required. No narrowed end state is substituted.
