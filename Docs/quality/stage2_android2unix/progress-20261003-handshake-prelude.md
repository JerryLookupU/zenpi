# Connection-Scoped Android Negotiation

Provisional S2-031/S2-032/S2-052/S2-072/S2-073 progress at
2026-10-03T19:52:41+0800. Previous goal turn repaired current-scope write
admission. This turn inspected QR/pairing/reattachment and the current Rust
listener's actual welcome/challenge emission. No nested agents, worker launch,
checklist promotion, commit/push, signing or cleanup.

## Repair

PairingHandshake and HostClient had checked the negotiation payload but not
the complete welcome/challenge envelope. FrameCodec permits optional routing
for general messages; that does not mean connection negotiation may contain
request IDs, owner/revision, project/session/turn or outcome metadata.

New shared HandshakePrelude reads the actual bounded FrameCodec frame, requires
exact schema_version/channel/payload envelope and exact welcome/challenge
payload, validates negotiated version and pinned host fingerprint, then returns
the nonce. Null extra fields are rejected too. Both pairing and authenticated
read/revoke paths use it. HostClient additionally checks coroutine cancellation
between welcome/challenge and before preparing proof. Rust host emits these
exact connection-scoped envelopes, so its current negotiation remains compatible.
No wire schema or grant policy is changed.

Five new JVM tests cover fragmented valid negotiation, routed/id/null extras,
wrong order/version/channel/host/nonce and every prelude truncation. Pairing's
production exchange is tested to emit only hello, never invoke signing and
never send its invite token after malformed negotiation. Shared prelude tests
exercise the parser used by saved-device authentication; no Keystore/TCP/device
execution is inferred from a proof-call counter.

The user guide is refreshed to 2026-10-03 and no longer contradicts current
source by claiming task-list paging and historical session rename/resume are
unimplemented. Current APK/device acceptance limits remain explicitly stated.

## Verification

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-handshake-prelude-20261003:
  exit 0, production/unit/instrumented Kotlin compile, 324 JVM tests pass.
  Existing compiler warnings remain; no APK/device run is claimed.
- cargo test --offline --locked --test remote_pairing --test remote_auth
  --test remote_security -- --test-threads=1: exit 0, 32 top-level tests pass
  (8 pairing + 12 auth + 12 security). Pairing's FIFO test also launches one
  filtered child test; that nested output is not an extra top-level case.
  Existing 20 dependency/library warnings remain.
- Python Stage 2 tool discovery: exit 0, 86 tests pass. Backup/lifecycle/
  Blueprint validator unit tests: 5/15/5 pass. Strict Blueprint validator,
  cargo fmt --all -- --check and git diff --check pass. All 36 rows unfinished.
- Release host source attribution verifies current binary
  9f1ff46c2fd82ccad171534cbde89b3f7c6b49babaef8fd0f7686ad3019caaa8.
  APK verify-only refuses missing successful current build provenance.
  No repeated blocked Gradle build or stale APK install.

Rust compilation took 1m43s with quiet link intervals; the same live command
handle was polled until completion, not restarted. Process inventory was denied
by sandbox; that refusal did not infer failure or trigger a restart. All needed
command handles from this turn are now terminal.

```text
HandshakePrelude.kt e1c0cba90703ed2c483a2ab2af8a5ac72b4836e0bdb12dd2475df1bd21cbaaaa
PairingHandshake.kt 9448997c48a580f68d279d522dbc314b193804842154659d9adac40773460f22
HostClient.kt ec02172c8e21596e2a0865323d511fa21e13ea98422b0503f1600e1aa7e6dff9
HandshakePreludeTest.kt 943cf825fdad4d9b3e36041587be32e77a783515d710cceacbc68ecd77553380
PairingHandshakeTest.kt 055ce5901fb7f58d12a584111815075046d54f509a160e37c243d374d99cff37
main.jar 2bc2163e97efdef1ffb6a137737acaa3fb2fb2d780585c43db93869aa02b5c93
unit.jar c9595d36f664a9e75200a75eecb7e674943b00a05948889f1e7d49ead3c9eaad
instrumented.jar b6db64c87f0d7b384bbbee742fa9d79b90c2ee5f5b6819f26f946415ac9e284b
```

Goal remains active. Current APK, real-camera QR/direct LAN, connected
session/lifecycle parity and signed install/upgrade/reproducibility gates are
still open. These focused source/framed-memory tests cannot close those gates.
