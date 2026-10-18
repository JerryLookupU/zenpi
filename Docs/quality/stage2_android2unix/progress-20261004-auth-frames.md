# Android Authentication Frame Interoperability

Provisional S2-003/S2-012/S2-013/S2-032/S2-052 progress. The preceding RTK
tool check was not product implementation progress. This turn inspected the
execution skill, authoritative Blueprint/specification, dirty integration,
pairing/client/authentication/framing and current cross-language tests.
No nested agents, checkbox promotion, reset, commit or push.

## Change

The P256 interop lane previously verified separate signature strings but did
not decode the complete Kotlin authenticate/revoke-status frames. It now
passes every fresh Rust challenge through Kotlin FrameCodec and
HandshakePrelude, signs through AuthenticationProof with a software P256
fixture, and writes one full length-prefixed frame for each of seven cases.
Rust read_frame checks message kind, control channel, absent project/session
routing, device ID and authentication/query/request IDs, forbids trailing
frames and duplicate cases, and uses the decoded frame signatures for the
existing actual DeviceChallenge authentication/revoke-status checks.

The lane covers valid proof, foreign host/nonce/device, replay, revoked-device
authentication and revoke-status query binding. Pairing still verifies the
one-use invite, canonical P256 public key and read-only/default-deny registry.
This closes a test-coverage gap; production behavior is unchanged. It does not
prove Android Keystore, TLS sockets, camera or direct-LAN acceptance.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty integration preserved.
Verification occurred Oct 4 local time (UTC Oct 3, approximately 20:50-21:05).

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-auth-frames-20261004:
  exit 0, twice. The second run used a pipefail/rg output filter only.
  Current six-case Rust QR fixtures; Kotlin main/unit/instrumented source
  compilation; 474 ordinary JVM tests; current Rust wire read/rename/P256/
  worktree/session/ACK lanes all pass. No APK/resource/DEX/device claim.
- env ZENPI_HOME=/private/tmp/stage2-remote-matrix-20261004 rtk cargo test
  --locked --offline --test 'remote_*' --quiet: exit 101. Cargo stopped at
  remote_headless_commands: 10 pass, 2 fail. Both fail at TcpListener::bind
  with PermissionDenied/Operation not permitted, lines 210 and 1472. No skip
  or softened assertion; remaining targets are not proved by this run.
- Separate Cargo run with the same temporary ZENPI_HOME, --locked --offline
  --quiet and the following 15 explicit targets: remote_protocol, remote_pairing,
  remote_auth, remote_security, remote_events, remote_subscription,
  remote_subscription_tls, remote_workspace, remote_subtab, remote_worktree,
  remote_worktree_runner, remote_worker_owner, remote_terminal, remote_parity,
  remote_migration. Exit 0, 252 top-level tests pass. The RTK run also exited 0
  but summarized 256/19 suites by including four child-process filtered runs;
  raw Cargo output establishes the primary 252/15 count. Not full Rust suite.
- Python Stage 2 discovery: 132 pass; lifecycle validator tests: 23 pass.
- Rust format check, git diff --check and strict Blueprint pass; all 36
  authoritative rows remain [ ].
- Host provenance initially refused a changed binary. Cargo/source tests
  replace target/debug/zenpi, so the host was rebuilt and separately verified
  afterward. Debug/source attribution only, not reproducible release.
- APK verify-only refuses missing build provenance. No Gradle/socket bypass,
  old APK installation, connected lifecycle or physical camera/LAN execution.

```text
remote_android_interop.rs 33ceefeca67bf82d94088e6bf6adcda859ff79b5caa4f1a76c2d3ce5d2f1bb1c
AndroidRustProofTest.kt f2d934c60103af8814158fadcce389ff806bfd53db8a363a397c638ec5e5671e
main.jar d11429817490daef47383be790488fb2dad445c406e07ac91121277872d4f7c9
unit.jar 3910f30aa64b8495a003d4c53e6066373160907b4f5a2b4c3f3525907bc06f9e
instrumented.jar 8c22b80093a6d240fc71428f32893dddb9b7a3c7fe1e5b4efaff04ca507bba8d
debug-host-receipt 2adf3b4d1cb94a81dbc097458a3b603a1a9a3a2a09d22fdb76fd91b8ff17e1d8
debug-host-binary 677c02131aa670e18cd3813bb1a8016026b375a3155f9c010b0c62f29d287df4
```

Goal remains active and incomplete. Current APK, physical QR/direct-LAN
two-way desktop synchronization/recovery, production listener, connected
API31/34/35 lifecycle, full parity, backup/uninstall and signed release remain
acceptance gates. Socket denial is not a claim of a product logic defect or
an excuse to replace these gates with source tests.
