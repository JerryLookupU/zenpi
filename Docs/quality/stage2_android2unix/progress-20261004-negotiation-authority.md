# Negotiation Refusal Authority Fence

Provisional S2-032/S2-038/S2-052 progress on 2026-10-04. The previous goal turn
repaired camera admission live-read cancellation and verified source gates, so
it was progress. Current Blueprint, dirty worktree, pairing handshake,
authenticated HostClient, trust manager, authority-loss and background-resume
policies were inspected. No nested agents, worker admission or authoritative
checkbox promotion.

## Repair

Certificate identity failure already removed live controls, but ALPN mismatch,
invalid welcome/version or challenge fingerprint mismatch were generic wire
errors. A foreground reconnect could enter Error while old approval/cancel/
steering capabilities remained available, and automatic reads kept retrying a
confirmed negotiation refusal.

Both socket clients now share exact zenpi/1 ALPN admission. Complete invalid
negotiation and wrong challenge host use HostNegotiationException. Authority
loss removes stale live/transient projections and stops the foreground read
loop; ResumeWorker treats this scheduled read as permanently refused. Typed
cause classification has identity-cycle protection and never matches message
text. No protocol fallback, pin replacement, new key/token admission or mutation
retry is added. Stored host, transcript/cursors and unresolved exact request
identities remain available for explicit operator reconciliation.

EOF, partial frame EOF, socket errors and timeouts remain transient. This is
intentionally separate from a complete malformed negotiation reply and from
post-negotiation wire errors. Three JVM regressions exercise production ALPN/
fragmented prelude parsing, wrapped refusal classification, incomplete traffic,
cause cycles, diagnostic spoofing and retained unknown-write identity while
old prebuilt approval/cancel/steering controls are denied.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-negotiation-authority-20261004: exit 0; production, JVM
  and instrumented source compilation pass; 416 JVM tests pass. Existing
  warnings remain. Instrumented tests compiled only, not device-executed.
- Stage2 Python discovery: 118 pass; lifecycle verifier tests: 21 pass;
  Blueprint verifier tests: 5 pass.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass;
  all 36 rows remain [ ].
- Rust source/binary unchanged; debug host provenance verify-only passes.
- APK verify-only exits 1: missing current-source provenance. No unchanged
  blocked Gradle/ADB/TCP attempt, old APK install or physical scan/LAN workflow
  was run. Tests do not prove Android TLS-provider ALPN or real reconnect.

```text
HandshakePrelude.kt bf3f551c2f193ce4c5b6d07c5678313a794faa401916abc625cacb65d8e7c458
HostClient.kt 017abeb41e5af9e42206eaf6f51141b844ce7a94cf147a4ae1fc51489b74f6d6
PairingClient.kt b9edecdfdc1fdae6cbe1f127d51ccb4a0fbddee8a8bc27f034181f1c2881fede
AuthorityLossPolicy.kt 7be97ee7775fa23b01be9b64021e037b429d1357c40f51d58bb416976e7bc891
ResumeWorker.kt 540889268be923430562b743eeac38e0d1a1d5fbb8d68a9d1fc9156f55925925
HandshakePreludeTest.kt efa8aa1a948ed4da22c31506d2cc1528a21f802ad20f40cb1113f6c76503e503
AuthorityLossPolicyTest.kt 023f23b8b4c0698222d9da9443318250289eb63c1dff4aec120a26b4ecec75bc
main.jar a021afb3b0ca52c084ea51a6674d364f11408f9c333030cde978e7391157d6f9
unit.jar 476c0b06291f6b504c91efe06306f9e58fe0ad6ef81c7cb723ae422da61d5090
instrumented.jar a2d7ab9d56a37c3a8506707a1fe769e8173bfcab35e66d130e7baf13dcd6ec75
debug-host-receipt 06c70100d9496d38e8128dfe1ed06e29ac935cba3b2c2b1f570f9159f50dcfce
```

Goal remains active. Source-bound APK, physical QR/direct LAN communication and
recovery, full parity/API lifecycle and signed release remain incomplete or
unverified. No complete Blueprint item or final workflow acceptance is claimed.
