# Pairing Identity Change Fence

Provisional S2-031/S2-032/S2-037/S2-052 progress on 2026-10-04. Previous goal
turn added and executed Android/Rust P256 proof interoperability; it was
progress. Current execution skill, dirty worktree, Blueprint, pairing client,
handshake, ViewModel commit, existing authentication proof and tests inspected.
No nested agents/workers, unrelated reset or checklist promotion.

## Repair

Pairing previously read public key and device ID separately after TLS and did
not recheck identity around signing/receipt. Key deletion/replacement could
therefore leave a successful old-device receipt published or saved despite
the current signing key no longer matching. PairingIdentityProof captures one
canonical uncompressed P256 public encoding and derives its device ID before
opening a connection. Handshake checks cancellation/current identity at initial
admission, after welcome, after challenge, after signing before token send and
after receipt read. PairingClient supplies the real key reader at every boundary.
The ViewModel additionally checks the returned device ID against the current
identity inside its storage lock immediately before metadata save.

Missing/malformed/replaced identity maps to typed authentication failure without
private Keystore diagnostics. Cancellation is preserved. No new key, signature
retry, token resend or inferred host rollback. A post-send failure remains an
unknown pairing outcome requiring operator reconciliation. These observations
are not an atomic transaction with external Keystore changes: a later key loss
is still handled by the existing saved-host authentication fence.

Four added JVM tests cover capture, malformed public values, deletion/replacement,
cancellation, all pre-send stages and receipt-time loss. Prepared existing
ResumeStore device test also captures actual Keystore public identity, deletes
it, creates a distinct replacement under the same fixture alias and refuses
both stale identities. It compiled only. The cross-language proof lane now
supplies this same identity checker and still passes real Rust verification.

## Verification

Commands ran from /Users/mac/Github/zenpi against dirty current integration.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-pair-key-race-verified-20261004:
  exit 0; Kotlin production/unit/instrumented source compilation, 460 ordinary
  JVM tests pass; current Rust read fixture passes; explicit Android/Rust rename
  lane and P256 proof lane both pass. Existing warnings remain. No device run.
- Python Stage 2 discovery: 132 pass; lifecycle verifier 21 pass; Blueprint
  verifier 5 pass. Rust format, strict Blueprint and git diff --check pass.
- All 36 authoritative rows remain [ ]. APK verify-only still refuses missing
  provenance. No unchanged restricted Gradle/socket/ADB retry or old APK install.
- Debug host rebuilt after all Rust fixture invocations; source attribution
  verified separately. No physical camera, current APK, direct LAN or connected
  lifecycle acceptance was claimed.

```text
PairingIdentityProof.kt d975947c6004892ec700c74bae29b5cd5be5f74c502c08937d5c993b4e09a561
PairingClient.kt d94541ca218138f994862f976a86396c543f759f66881c328463d7c098b7ab0a
PairingHandshake.kt 0979a16a5e496f8d29bbbd7e42f440fedd44aa9a2bab7200c974ebd0051fd4dc
PairingViewModel.kt 1cf728637652ea7808eb9d3c1146ec6286a2b64bd8de26299d2ebea7deae08dc
PairingHandshakeTest.kt 3e95baa475e7ad7404d4417f64429c508590ac619b373ef6fde258d8e7cc2cb0
PairingIdentityProofTest.kt 22e3bd6adc93e4a5a5b55cf64fa4d98c2c5fcf0977c60e535177e26181e2d970
ResumeStoreTest.kt b6ea7af8a2db049da0b30933d59cce49562c12c8840eb8af04e1651e960da775
main.jar 66db20701d2cff5559ad457cceb93fea93c6657ff85cfe03bb94b1fd8b8d4386
unit.jar 2d063841aa8b79170dd7768a96c55418a3c1fd9f255aeaa5cf66b28f5c269d42
instrumented.jar f3913356a11db188a5396e9e4f6c8246b7c5c3213de8127858cb44d5b5801332
debug-host-receipt be2293a7afd64dcbb855b2fd81c41855f0bae2876d96997cf8f7d7a4a4684b79
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active. Current APK, physical QR/direct-LAN bidirectional sync and
recovery, current connected API lifecycle matrix, full parity and signed
release remain incomplete or unverified. This repair and source regression
evidence do not accept a whole Blueprint item or supersede full-suite failures.
