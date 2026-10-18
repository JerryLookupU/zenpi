# Connection Handshake Scope

Provisional S2-010/S2-012/S2-013/S2-032/S2-050/S2-052 progress. Previous
goal turn fixed the single Activity pairing owner and was progress. Current
execution skill/Blueprint, host build attribution, Rust production listener
handshake, registry/issuer, protocol/frame boundaries, Kotlin handshake/proof
and shared corpus were inspected. Dirty integration preserved; no nested
agent, reset, commit, push or checkbox promotion.

## Defect and Change

Android HandshakePrelude rejected routed welcome/challenge negotiation, but
both generic frame codecs still admitted routed pair/authenticate; Rust hello
also accepted project/session fields that the production listener ignored.
Serde Option decoding erased explicit nulls, allowing handshake authority
fields to disappear before structural validation. No bypass of device proof
or existing ACL is claimed; this was an inconsistent ambiguous envelope.

Both codecs now require hello/welcome/challenge to contain only schema_version,
channel and payload. Pair/authenticate/revoke_status add only required id.
Project/session/turn routing, sequence, owner/revision and outcome/code are
refused even if null. Rust direct envelope validation rejects populated routing
and correlated negotiation; framed decode checks key presence before host
admission. Its extra structured presence parse runs only for six connection
messages after existing JSON/channel size checks. Other routed message nullable
compatibility remains unchanged; no protocol version or grant behavior changed.

Paired Rust/Kotlin tests cover all six messages with each forbidden field as
null and a correctly typed non-null value, plus no bytes emitted on rejected
outbound frames. Seven shared corpus rows add legal unrouted authentication
and routed/null negatives, with current total 58 raw cases. Existing seeded
2048-case mutation checks continue to run in both languages.

The current Kotlin P256 interop lane also mutates its actually signed pair
frame with six foreign/null routing/authority fields. Rust read_frame refuses
them while the registry remains empty; subsequently the original signed request
registers default Read/no-project capability and consumes its actual issuer
token once. No mock registration receipt is promoted to host/TLS acceptance.
The existing Kotlin proof fixture still synthesizes the pairing receipt, while
Rust independently verifies/registers its generated request; production listener
roundtrip, Keystore and physical transport remain separate open gates.

## Verification

Cwd /Users/mac/Github/zenpi, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Final observed UTC
2026-10-03T21:54:57Z. Existing dirty integration preserved.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-connection-handshake-scope-20261004: exit 0 under
  pipefail/rg filtering; current QR fixtures, Kotlin main/unit/instrumented
  compile, 475 ordinary JVM tests and all current cross-language lanes pass.
- env ZENPI_HOME=/private/tmp/stage2-handshake-final-20261004 cargo test
  --offline --locked --test remote_protocol --test remote_frame --test
  remote_auth --test remote_pairing --test remote_security --quiet: exit 0;
  60 pass, 1 ignored real-loopback test. Identity special-file regression also
  runs its bounded child test. Existing compiler warnings remain.
- Earlier RTK summary reported 61 pass because it also counted that child
  test, and filtered counts from the child; direct final per-target totals
  above are authoritative. No test skip added or refusal relaxed.
- Python Stage 2 discovery: 135 pass; lifecycle validator: 23 pass; backup
  and Blueprint validator tests: 5 each. Rust fmt and git diff --check pass.
- Strict Blueprint passes, all 36 authoritative rows remain [ ].
- Debug host rebuilt after all Cargo/source tests; separate verify-only run
  validates updated source inventory and real binary.
- APK verify-only refuses missing provenance. No repeated unchanged blocked
  Gradle/ADB attempt, permission bypass, stale APK installation or real LAN claim.

```text
protocol.rs 86cf1fce3bfbd05c32b857546fb159ee9ea3902b6a08157e44eb8289e15ac8c7
FrameCodec.kt f38bb37066fea6ec94c926cfe83dc07537295b923efe0a3f72d2d69028c88aea
remote_protocol.rs 5eb3d0de89d8cd7f8901056b5d345005616b06cc03b02eecd84d1bf8d03ca5ee
FrameCodecTest.kt 33071000f5314cfeca8c6de80352c68041b3dc9fcd7ed8df676fcee1c82abd71
remote_android_interop.rs 06c1ecfb5e1e8eb10cfc38d7dc4cc8b3a4f02659003632b1bee7af5416093d61
corpus.jsonl ac50ccbd7c10a5e20943a3ebeb90c2e167b194891d18e6d287e9f3a5c297e7c7
main.jar f39ac7261edf72ae323c5103d92c928c2d7517329a101a228001ef398362d6c4
unit.jar 1251c4a3ae844f3aa9ace8a4cbfe7fbdd75f7b18aabec0a4ef5db57c42bf2544
instrumented.jar 91c2860683358c4a27d2114bc748e8ac929e6e72035fba4f67de0aaa7cb12551
debug-host-receipt 84043a371d332d836e0463580ac5562407286754b2318bd7fd41b72ebd8d8042
debug-host-inputs d220c0bc7d19319d48bcb2468b567d07dc1154df4ffb8fe0954acb59dce26583
debug-host-binary 8a826eef42d30492ace13ea37eb454d6c7819bf91251beb6942dcd683cd039e9
```

Goal active and incomplete. Fresh source-bound APK, physical camera QR/direct
LAN bidirectional synchronization and recovery, API31/34/35 connected lifecycle,
complete parity, migration/reset/uninstall and signed reproducible release
still require acceptance evidence.
