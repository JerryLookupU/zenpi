# Canonical Device Challenge Nonce

Provisional S2-003/S2-012/S2-032/S2-052 progress. Previous goal turn wired
current QR image generation into Kotlin decoding and was progress. This turn
inspected execution skill, dirty integration, Rust challenge/protocol/auth,
Kotlin TLS prelude/signature/receipt/key admission and current proof interop.
No nested agents, checkbox promotion, destructive reset, commit or push.

## Repair

The documented challenge nonce is 32 random bytes encoded as unpadded base64url.
Both codecs previously checked only a 43-character alphabet. That admitted
noncanonical trailing unused bits, despite the host generator producing canonical
values. Rust Envelope validation and Kotlin FrameCodec now decode exactly 32
bytes and require round-trip encoding equality. No nonce is normalized before
signing and no signing/authority path is permitted for a malformed challenge.
Normal generated host nonces and shared canonical fixtures remain valid.

New Rust/Kotlin frame regressions enumerate 256 last-byte values, their valid
encoding and each of three nonzero unused-bit variants. Rust validate/decode
and Kotlin decode/encode reject those variants. Kotlin prelude regression checks
typed HostNegotiationException, zero signing calls, authority fencing and no
background retry. PairingHandshake regression confirms only hello was sent:
no pair frame/token or device proof is sent after the invalid challenge.
Protocol and network matrix document this existing format requirement precisely.

## Verification

Cwd /Users/mac/Github/zenpi, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b, preserved dirty integration.
Observed time during verification: 2026-10-03T20:45:44Z, Oct 4 local time.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-canonical-challenge-20261004:
  exit 0; current six-case Rust QR generation, production/JVM/instrumented
  Kotlin compilation, 474 ordinary JVM tests; real current Rust read/rename/
  P256/worktree/session/ACK interop lanes pass. Existing warnings remain.
  Device assertions are source-compiled only; no resource/DEX/APK/device pass.
- rtk cargo test --locked --offline --test remote_protocol --test remote_auth --test remote_security --quiet:
  exit 0, 41 pass across three suites.
- remote_subscription_tls: 43 pass before repair, then separately rebuilt and
  rerun after repair with ZENPI_HOME=/private/tmp/stage2-command-scope-home-20261004;
  43 pass, exit 0. Real TLS over bounded fragmented in-process pipes, not the
  production TCP listener or Android TLS/camera/direct-LAN acceptance.
- Python Stage 2 discovery: 132 pass; lifecycle validator: 23 pass.
- Rust format, git diff --check and strict Blueprint pass; 36 rows remain [ ].
- APK verify-only refuses missing provenance. No restricted Gradle retry or
  historical APK installation. Current debug host rebuilt after Cargo tests;
  separate verify-only passes, not release or reproducibility acceptance.

```text
protocol.rs 413ec6e90d0708044f285857fdd75b7c1ad0725d6f0333073f101bde5f11dc7c
remote_protocol.rs e9b4df4d3ed09607b754c62a22b78c2b7dc9fbd21fdfc9cbc2e515e75f6c86d3
FrameCodec.kt f2298e5737526074f92cc31ec12c6a8e931bbe44b294a912bf5f4febb4c96e7a
FrameCodecTest.kt 95ec6a0d2439e809aaec22af724eada2794c7cd3236663539bb668ad5b6e8114
HandshakePreludeTest.kt a0b8c92af6067f1671edf13f9ac02df289f4e32907413d26fe4b14d24522799c
PairingHandshakeTest.kt 39843288378e482f0d190f1a104cd5e50c33495875edd6bbd25e75d157b3a8b6
main.jar d11429817490daef47383be790488fb2dad445c406e07ac91121277872d4f7c9
unit.jar eef303460edd642780b0da3d92216204b11b929fdc1f3ae1ba859f20fb3efb3f
instrumented.jar 8c22b80093a6d240fc71428f32893dddb9b7a3c7fe1e5b4efaff04ca507bba8d
debug-host-receipt 38afe4ebab482f4cd9a7b7cfa58751ac78a70ea13ac76ba994a0939077a2233f
debug-host-binary 677c02131aa670e18cd3813bb1a8016026b375a3155f9c010b0c62f29d287df4
```

Goal remains active and incomplete. Fresh APK, physical QR/direct-LAN two-way
sync/recovery, connected API lifecycle, full parity, backup/uninstall and signed
release still require acceptance evidence. This repair proves no exploitation
claim, real device handshake or complete Blueprint item acceptance.
