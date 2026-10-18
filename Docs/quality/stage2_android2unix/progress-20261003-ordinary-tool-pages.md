# Ordinary Transcript Tool Argument Pages

Provisional S2-019/S2-020/S2-036 progress on 2026-10-03. The preceding RTK
verification turn did not advance product implementation; this turn changes
the actual host and Android read workflow. Dirty user changes are preserved.
No nested agents, worker admission, checklist promotion, commit or push.

## Implemented Path

TranscriptRead accepts turn_id/tool_call_id/offset/digest for ordinary assistant
turns as well as the existing completed-worker route. The host checks current
Read authority, project/session, owner availability, active tree membership,
one matching call and tool name bounds. It returns structured-redacted
arguments only, not execution evidence, with 64 KiB document and 8192-byte
UTF-8 page limits. Digest-fenced continuation never mutates the journal.

Android's ordinary transcript exposes an OpenInFull action with a tooltip.
The existing selectable monospace dialog reads and navigates call-bound pages;
next preserves the selected call rather than falling back to message content.
Scope/turn/call changes invalidate reads. Foreground, network, authority and
pending-write fences remain in place. Pages are memory-only, not cached.

## Completed Verification

Rust commands use ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home so
configuration migration does not attempt an out-of-sandbox HOME write.
All commands below reached exit 0:

- cargo test --offline --locked --test remote_workspace ordinary_tool_argument
  -- --test-threads=1: one new domain/owned-adapter test passed. Includes full
  UTF-8 reconstruction, redaction, absent/foreign call, wrong role/session,
  stale/missing digest, call substitution, Busy, branch switch, revoke and
  journal immutability checks.
- rtk cargo test --offline --locked --test remote_workspace
  -- --test-threads=1: all 62 passed.
- rtk cargo test --offline --locked --test remote_subscription_tls
  ordinary_tool_argument -- --test-threads=1: three new tests passed.
- rtk cargo test --offline --locked --test remote_subscription_tls
  -- --test-threads=1: all 40 passed. This uses authenticated rustls TLS 1.3
  over bounded in-memory pipes, not a TCP listener or physical LAN. New cases
  reconstruct ordinary arguments and refuse continuation after revoke or
  stale digest, without journal/checkpoint writes.
- rtk cargo test --offline --locked --lib remote::transcript::tests
  -- --test-threads=1: six passed; includes UTF-8 and raw/redacted cap tests.
- rtk cargo test --offline --locked --test remote_protocol --test remote_auth
  -- --test-threads=1: 26 passed across two suites.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-ordinary-tool-20261003: production, unit and instrumented
  Kotlin sources compiled; all 278 JVM tests passed. Three new ordinary-call
  cases cover scope/identity, exact wire shape, navigation, role, document cap
  and authority cleanup. New 320 dp Compose open/next/offline interaction test
  is compiled only, not executed on a device.
- cargo fmt --all -- --check; git diff --check: passed.
- python3 tools/validate_stage2_android2unix_blueprint.py --strict: passed;
  36 unfinished items, zero accepted. This validates structure, not completion.

Existing compiler warnings remain. No retry of blocked TCP/ADB/Gradle sockets,
no new APK build, and no connected-device or camera/LAN acceptance claim.
All observed command handles are terminal.

## Artifact Digests

```text
src/remote/transcript.rs d220f852b3694a5185072350cb0f1c1fb734fd2f6b0629da2e67937c5bb0478f
tests/remote_workspace.rs 866767fb6de5c35238be179ad8791da2c46aed965d53169fec7d86842e0fa2fd
tests/remote_subscription_tls.rs 7396fd36496bd5b81b7deacefe3e0351fee454bd69749f2440b6309492c3e0b7
TranscriptPageCodec.kt 94605def2460ac443eede03256d982f25765b49cfbdcbf14102a7fcb7bb7119e
PairingViewModel.kt 46e2868a7dfb4bf9467e06bbd40f8f155b358c53e1cb19c642e34556e1245110
MainActivity.kt 29d042280f339f6599e6488768a2bd8c1529495f96c759d8f7185c8045e9dbc9
OrdinaryToolArgumentsTest.kt a627384ee78110fd9b4f96a198e0268cbdcadeadc5b615b697f33241807bf83a
TranscriptToolScreenTest.kt f1c75368ef522f16cb4a4568ed2300d09553fdaf75434c1b508f46e3958b8114
main.jar 28261fee46610b33628fbe24a8f5b344c8144aeef54daabfa2bc1f86a3158124
unit.jar 300bc5fbf8b2efcee8d0c18fc11a7acde3f9297bcf915fd3b9f3f0c488a8c4f2
instrumented.jar f477826f1c4ef8a80e84816a85a87bbdbb14462beb7fd87395f5f863691ca358
```

The full goal remains active. Complete interactive PTY, session/settings control
parity, calls omitted beyond bounded previews, current production-binary
provider/tool E2E, physical camera/direct LAN, connected lifecycle matrix and
signed APK/release gates remain incomplete or unverified. This focused read
path does not establish complete TUI parity or end-to-end QR/LAN acceptance.
