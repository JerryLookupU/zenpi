# Authentication Reader Authority Fence

Provisional S2-032/S2-038/S2-052 progress on 2026-10-04. Previous turn repaired
generated-schema build admission and verified its gates, so it was progress.
Current Blueprint, dirty worktree, ResponseReader, authentication receipt and
HostClient validation order were inspected. No workers/nested agents admitted;
no authoritative checkbox promotion.

## Repair

Semantic authentication receipt refusal was typed, but complete bad replies
could be rejected by earlier framing/correlation checks as ordinary wire errors.
Likewise accepted/unknown outcomes hit generic operation-completion handling
before authentication validation. A failed reconnect could therefore retain
old Error-phase decision authority even though no current authentication had
been confirmed.

ResponseReader now classifies its explicit decoding/correlation/envelope/code
refusals as HostAuthenticationException only for authenticate requests. It
also refuses unsolicited event/message traffic with that type. HostClient
validates non-failed authentication outcomes before generic completion handling.
The existing authority-loss and background retry policies then remove stale
live controls and stop automatic reads while retaining durable host/display/
cursors and unknown exact request identities. Other request types, typed failed
host codes and valid receipts keep their existing behavior.

EOF, partial prefix/body EOF, timeout and socket I/O are not converted to
semantic refusal. Two JVM regressions exercise nine hostile framed replies,
nonterminal outcomes, no follow-up admission, all truncated valid receipt
boundaries and timeout identity. Tests run production framing/receipt/policy
code, not a real Android TLS/Keystore or physical LAN connection.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-authentication-wire-fence-20261004: exit 0; production,
  JVM and instrumented sources compile; 422 JVM tests pass. Existing warnings
  remain. Instrumented tests compile only, not device-executed.
- Stage2 Python discovery: 125 pass; lifecycle verifier tests: 21 pass;
  Blueprint verifier tests: 5 pass.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 rows remain [ ].
- Rust source/binary unchanged; debug host provenance verify-only passes.
- APK verify-only exits 1: missing current-source provenance. No unchanged
  blocked Gradle/ADB/TCP retry, old APK install or physical camera/LAN run.

```text
ResponseReader.kt 518f03691b32bc10ea66a3715f84f87bbefdc755db775781ca8a62a33aed47b3
HostClient.kt ff9a1dc39bd21cbdb94ce6a8ecd99d31c9a678b79f8f18116e8ef5d3b213d96a
AuthenticationReceiptTest.kt c5875956ac24453a308bf67251c986e84125d6ff771c1a804dc5c26797e06c0d
debug-host-receipt 8a5290618441efe7e654303e98923bedb59ea6f20195f9e868fdef3f46c72bc2
main.jar 8c6e21b6d57a034e84bba567e96f003b02c9756b9339deeaa1ca3f6260550b96
unit.jar c599c54def84c849e0b9e03c84cbe0eff1de9af99955720973565dd92e198f2e
instrumented.jar a2d7ab9d56a37c3a8506707a1fe769e8173bfcab35e66d130e7baf13dcd6ec75
```

Goal remains active. Current-source APK, physical QR/direct LAN communication
and recovery, complete shared schema/parity/API lifecycle and signed release
remain incomplete or unverified. No entire Blueprint item acceptance claimed.
