# Current Rust QR To Kotlin Decoder

Provisional S2-012/S2-021/S2-031/S2-050 progress. Previous goal turn tightened
cancel restore/approval session scope and was progress. This turn inspected
execution skill, dirty integration, current invitation encoders/parsers,
scanner admission, terminal QR renderer and existing shared corpus. No new
definite parser defect was found; the current-renderer evidence gap was repaired.
No nested agents, destructive reset, checkbox promotion, commit or push.

## Current Source Lane

The previous Kotlin QR test consumed historical generated luminance resources.
Those are useful regression inputs but did not prove the current Rust terminal
renderer remained compatible with the current Kotlin decoder. The source checker
now runs cargo run --locked --offline --example remote_qr_fixture first, then
passes its output directory as zenpi.qr.fixture to the ordinary JVM suite.

The generator calls production qr_lines and serializes its actual black/white
half-cell colors to PGM luminance. An exact six-case manifest covers private
IPv4/IPv6, port 1/65535, a full-length private IPv6 address and explicit v4/v6
loopback fixtures. All use fixture-only public deterministic token data and a
300-second window, not a live listener/invite registry. Kotlin requires the
exact manifest identities, reads those generated files without resource fallback,
decodes with the app's ZXing QR engine and verifies exact URI/address/SAS.
It also checks the TTL boundary, one second before expiry, refusal at expiry,
permanent refusal after observed expiry, too-early admission and token redaction.

When no current directory is explicitly supplied, ordinary Gradle tests retain
the historical two-image fallback. The shared 45-case invitation corpus is
unchanged. This lane proves current image/parser interoperability, not camera
focus/exposure, actual terminal font/colors, Android Activity result delivery,
APK/device, TLS, network route or LAN reachability.

## Verification

Cwd /Users/mac/Github/zenpi, preserved dirty integration, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Observed verification time:
2026-10-03T20:40:18Z, Oct 4 local time.

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-current-qr-20261004:
  exit 0; fresh six-case QR generation, production/JVM/instrumented Kotlin
  compilation, 471 ordinary JVM tests including all six current QR decodes,
  current Rust read/rename/P256/worktree/session/ACK interop lanes pass.
  Existing warnings remain. No Android instrumentation was device-executed.
- rtk cargo test --locked --offline --test remote_tui_pairing terminal_pixels_decode_to_the_exact_invite_with_quiet_zone -- --exact:
  1 pass, 4 filtered, exit 0. This separate Rust decoder test is not a camera run.
- Python Stage 2 discovery: 132 pass. Lifecycle validator: 23 pass.
- Rust format, git diff --check and strict Blueprint pass; all 36 rows remain [ ].
- APK verify-only still refuses missing provenance. No repeated restricted
  Gradle attempt or historical APK installation. Debug host rebuilt after
  Cargo checks; separate verify-only passes, not release/reproducibility proof.

```text
remote_qr_fixture.rs 992722d8bd2714004e93ecc2f972110a2b9c558811eca891891960adf5282d5b
stage2_android_source_check.sh 20eddb24475a2d518c446770ef52a5435030d8f0788746f3908b2800608aeedb
PairingCorpusTest.kt 2bb33143a2474863aec3d449e92b1b681674712dd70f5a67d4b5215997e93380
manifest fb31c9b98d2deeac9ea8da94685e69137a4dac3afdc17cc3ce59123b2cac2403
ipv4.pgm 5d953294a41130be0a77b05fbef29944fd89305096a1ba814bb9d45ee86aa71d
ipv6.pgm 80081c618b4b4f3853742a4459141fdf7ed6797c6b54bd85dd66df4f74b41c5a
ipv4-port-min.pgm 5e37a3a20446cdc1e7c9b9b162e4334bad06772b4e38a3b2a5eff2c4bf489927
ipv6-long.pgm 39cfe6eda0f0596c8955b519aa742cc3b8f6f0746079afa7582c33e0f8156ce0
ipv4-loopback.pgm 9a9e6d76c74645d8d592dc2097f86d64e3b9cc1e1e12cb2a936a808a9b0dbad8
ipv6-loopback.pgm 48799673d8943fe4f9d22b4974ae59508938a62dedb947bd17ff393ded95c83b
main.jar df5faa20b3408c401082ccb5760ad3a9cd666db6785ac5d2c23b69cffc38c079
unit.jar 97a67a8502832333769052a56d43b5b8bd4451ec7d15c824a04487734c2d6778
instrumented.jar 8c22b80093a6d240fc71428f32893dddb9b7a3c7fe1e5b4efaff04ca507bba8d
debug-host-receipt 6056f9fa9d5848e01525ba5c9243ce55df6c9e01c65a33724d6468fd6337df1d
debug-host-binary 701d3e597cd640090cdc254f6ad2f810b986dbd626e42179ca3da6d8e50c5d77
```

Goal remains active and incomplete. Fresh APK, physical camera/direct-LAN
bidirectional sync/recovery, connected API lifecycle, full parity, backup/
uninstall and signed release still require acceptance evidence.
