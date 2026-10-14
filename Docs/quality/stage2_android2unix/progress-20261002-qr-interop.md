# Terminal QR And Android Decoder Interoperability

Provisional S2-012/S2-021/S2-031/S2-052 progress observed
2026-10-02T04:53:41Z. Previous goal turn: progress (wire corpus and codec fixes).
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; existing dirty changes preserved.
All 36 authoritative rows remain [ ]; full Android/QR/LAN/Rust desktop goal
active. No worker admission, agents, commits, pushes or cleanup.

## Implementation

45-case pairing-corpus.jsonl is consumed unchanged by Rust remote_pairing and
Android PairingCorpusTest. Valid cases assert normalized literal address and
the independently calculated six-digit SAS 096238. Negative cases cover raw/
escaped duplicates, exact origin/version spelling, expiry and token encodings,
hostname/legacy IPv4/scopes, unsafe address classes and boundaries.

Host PairingInvite validation now rejects IPv4 link-local and unsafe mapped
IPv6 effective wildcard/multicast/broadcast/link-local addresses. URI parse
requires exact zenpi://pair query origin, exact v=1 and decimal <=19-digit
expiry. Android broadcast check uses the parsed IP so mapped broadcast cannot
evade the textual check. Loopback remains legal for explicit local fixtures.

remote_qr_fixture example calls the actual production host_pair::qr_lines and
rasterizes foreground/background terminal cells at 4 pixels/module. Produced
IPv4/IPv6 PGM images and JSON bind image dimensions, raw URI, fixture clock,
address and SAS. Android JVM test uses app ZXing QRCodeReader on actual PGM
luminance pixels, compares exact decoded URI, then parses it and verifies
address/SAS. Fixture tokens are public deterministic data, not live invitations;
no listener/device registry/provider was started. Gradle unit resource root
already includes this directory; cached JVM runtime now explicitly includes
ZXing's existing jar as well as compile classpath.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-qr-interop.

- rtk cargo test --offline --locked --test remote_pairing --test remote_lan:
  exit 0, **14 pass** (7 pairing, 7 LAN), shared invite/address/SAS corpus included.
- rtk cargo test --offline --locked --test remote_tui_pairing
  terminal_pixels_decode_to_the_exact_invite_with_quiet_zone: exit 0,
  **1 pass**, 3 other tests filtered. Production rendering independently decodes
  via quircs; no whole TUI TCP suite claim.
- rtk cargo test --offline --locked --test remote_auth --test remote_protocol
  --test remote_subscription_tls: exit 0, **58 pass**.
- cargo run --offline --locked --example remote_qr_fixture -- fuzz/remote_protocol:
  exit 0; actual production cells generated IPv4/IPv6 PGM plus fixture metadata.
- zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-qr-interop-android-final:
  exit 0, **171 JVM tests**. Includes all 45 invites and both production QR images
  decoded by ZXing. Main/unit/instrumented Kotlin compilation; instrumented tests
  compile only, no current-device execution claim.
- Six Python smoke/lifecycle/blueprint/backup modules: exit 0, **63 pass**.
- cargo fmt --all -- --check, git diff --check, strict blueprint: exits 0.

Final hashes:

```text
pairing-corpus.jsonl 2d00db36bc992acc035085b7f153796fe5fad5e4fae47aedf561bf875ad62caf
terminal-qr-ipv4.pgm 753222554a80def852abd685e9eb6dd22a6dd617f3c0736518c747137ee63e4d
terminal-qr-ipv6.pgm 84128238d640afb35285f2a87409fe766ce1509dc0d7bd587616901885380e43
main.jar 097ee2528f18a3111246729cd828030d03e6d039baba3fef2dfef97cfbcb8a76
unit.jar 0deafe09dc4d580d9bd65669a198fe55530f833e5884cc93798dd7562686f8fa
instrumented.jar 889638ef4d4b8f205324458ea3e5ffd14725630e488122a2242a77f13efbe9ee
```

Intermediate JVM run failed because cached runtime omitted ZXing even though
the compile classpath contained it; runtime classpath corrected and final full
suite reran. No dependency reinstall or network permission workaround. Fixture
SAS initially transcribed incorrectly while authoring; corrected to independent
SHA-256/unsigned modulo result before accepted suite execution.

## Remaining Acceptance

PGM/ZXing and quircs verification proves tested production QR pixels and invite
parser agreement only. It does not prove terminal camera exposure, autofocus,
screen readability, actual APK scanner activity, LAN TLS or device matrix.
Current executable APK, real QR/camera/LAN, release and remaining scheduler/
PTY/account/settings parity remain incomplete. Prior Gradle/adb socket limits
were not bypassed. No broad completion or authoritative acceptance promotion.
