# Durable Message Page Reading

Provisional S2-020/S2-036 work observed 2026-10-02T02:07:49Z. Previous
goal turn implemented durable truncated display/cursor bounds: progress. Full
native Android/QR/LAN/Rust desktop objective remains active; all 36 rows [ ].

## Implementation

Add read-only transcript_read operation through protocol, ACL classification,
shared/owner-thread HostAdapter and current Agent history. Arguments are exact
turn_id/offset/digest; reads require project/session/request identity, forbid
write authority and outer turn/sequence. Read grants suffice; revoked/foreign
owners are refused, held Agent returns Busy. Only selected current history is
readable, not arbitrary session files or abandoned branches.

Redact the whole durable content before slicing. Pages <=8192 UTF-8 bytes,
original/redacted content <=4 MiB, nonzero offset requires previous complete-
redacted-content SHA256. Invalid UTF-8 boundaries fail; changed digest fails
RevisionConflict. Result binds scope, role, digest, offset/next/total and
has_more. Journal, workspace revision, transcript cursor and ACK unchanged.
This adds bounded remainder reading, not unlimited bulk transfer.

Android strict request/page codec and HostClient use existing pinned TLS,
device authentication, socket lease, LAN binding and absolute deadline. Page
reads bypass listing/resume/ACK entirely. Connected truncated-message action
opens a bounded dialog retaining only one page. Next explicit, scroll resets,
loading/error/EOF/close states; error requires restart by close/reopen. Validate
scope, known turn role, digest/total consistency and contiguous byte offsets.
Background/network/selection/reset/new writes cancel and clear page state.
Local reset joins the prior page job before key removal. Opening diff also
clears pages without restoring the old state afterward. Page text never enters
encrypted transcript cache. Older hosts reject operation rather than fake a
complete message. Protocol and Android guide updated.

## Verification

Cwd /Users/mac/Github/zenpi, Rust tests
ZENPI_HOME=/private/tmp/zenpi-transcript-pages-20261002.

- cargo test --offline --locked --test remote_workspace --test remote_protocol
  --test remote_events --test remote_subscription_tls: **52 pass** (24/11/6/11).
  Then focused remote_workspace transcript_pages rerun after extra routing,
  revoked-device and UTF-8 boundary coverage: **1 pass**. Domain test reconstructs
  every redacted page, verifies full digest/original journal bytes unchanged,
  stale digest, missing digest, foreign session/project, busy and revoked ACL.
  Shared dispatch and actual owner mailbox both return the typed page.
- Existing rustls subscription 11-test regression passes; it is actual encrypted
  in-memory transport, not a new transcript-page TLS/TCP/LAN gate. No claim the
  new page read was device/network executed.
- cargo check --offline --locked passed after new production Rust routing.
  cargo fmt --all --check, git diff --check and strict blueprint validator pass.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-transcript-pages-final4-20261002: exit 0. Main/unit/device
  Kotlin sources compile; **144 JVM tests pass** in 27 classes. Three new page
  codec tests verify read-only requests, UTF-8 offsets, empty final page,
  exact flags/fields/scope/digest/size/role and rejection without mutation.
  Compose page/next/close/offline tests and ViewModel background page calls are
  compiled only. They are not counted as device passes.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint: **53 pass**.

SHA-256 source:

```text
src/remote/transcript.rs 9c488ef3c3e93ad07cfc7d47c50d09f7838ff2e15def326d39764c613d27d995
src/remote/protocol.rs 47bff8276567bcfc16da1cea4b76ef32deb8ddd4de40fae60e96401565567c83
src/remote/host_adapter.rs 728587d5fd75f864ef3edd2e18880768ff1bb555cc55b65260f1357012cd479f
tests/remote_workspace.rs 3b00feccc464a81e9cbd79e7ecb98224c1c74c33f277f50f7f829ae92b5d6c46
TranscriptPageCodec.kt 1d9fec582e8e7b780431138e6086731abe5671d9e596a934f9da9ba90618278d
HostClient.kt 3ded946039bef1bbea9f8988b905e697ffe399b4d54efc6663f1b9e29994de57
PairingViewModel.kt c2bf0fc0fb7d46483800abb0dbb145c1fab4b6b489aca8f7b945630ceb718657
MainActivity.kt 2f7bc2701171ca8817952aecde1b95497d5324b8f50dd0218e26800163ccc7f5
```

Compiled evidence under /private/tmp/zenpi-transcript-pages-final4-20261002:

```text
main.jar dd8f17a68a55fce59e91165daed0b3897919e15e70ee99cc193a767237a0b4bf
unit.jar 813d14b3f66b454f668d717264646c9338b43ac2dba42d32fcfbd3cbabfe3d92
instrumented.jar 96e9dab756ed1fc648f1577e330e0df8318577f8d47411bc932e3fd1820a7bbf
```

No fresh APK, physical camera QR or phone same-LAN run this turn. New page
TLS/device end-to-end, capacity boundaries, tree branch switch cases and richer
previous-page/history navigation require further coverage. Per-worktree quotas,
full TUI/headless parity, provider-delta replay/ACK, interactive PTY, current
API31/34/35 lifecycle and release/migration remain incomplete. No agents,
commit/push, permission bypass, unrelated rollback or acceptance promotion.
