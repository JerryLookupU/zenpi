# Transient Display On Bounded Subscriptions

Provisional S2-020/S2-033/S2-036/S2-038/S2-053 progress. Previous goal turn
implemented and verified bounded transcript subscriptions. This turn extends
that production path with live model preview, prompt status/text, approvals,
read-only terminal and workspace/subtab views on the same request/connection.
The full QR/LAN/Rust-desktop objective remains active; all 36 Blueprint rows
remain `[ ]`. No nested agent, commit, push or Master acceptance claim.

## Source Changes

PairingHost calls subscription::serve_with_view. Existing authorized workspace
listing plus selected subtab listing provide the view, with requested session
and owner/revision agreement required. Existing attached-TUI mailbox adds model
preview and scoped terminal; existing headless mailbox adds device prompt
progress. Unchanged views are omitted, changed views included in the correlated
batch response. Both reads are side-effect-free and reuse current device ACL.

Owner execution can hold the Agent mutex, preventing transcript replay while
metadata/control-based approval and live text remain readable. After successful
authorized view sampling, Busy replay yields subscription_transcript=false with
no event or journal advance, retaining the exact prior cursor. Other failures
are not converted to success. Android initial Busy resume may enter this path
only for foreground same-session prior data and advertised subscription ability.
Without a previous cursor/snapshot, it still refuses. WorkManager never opts in.

Android reducer checks completion, total frames/result bytes/batches, project/
session, workspace/subtab owner+revision and every existing display codec before
publishing a replacement. Transient-only batches cannot append turns, advance
event/journal cursor or accept string-coerced cursor numbers. ViewModel compares
cache representations to avoid rewriting encrypted storage for each partial
text update. Changed cache data is saved before visible publication; generation
fences and diff invalidation remain active. Transient text/terminal/approval data
never persists offline; active statuses become unknown_outcome on restore.

The byte budget now covers the entire response result including transient
views and completion/availability markers. 128 frame, 196608 byte and fixed
socket deadlines remain. Large frequent view changes can refuse ResourceLimit.
It samples internally every 250 ms, is bounded to a few seconds and remains
response-based display snapshots rather than a continuous provider callback
event bus, transient replay/ACK or full PTY interaction.

## Verification

Verified source timestamp: 2026-10-01T23:50:36Z (Oct 2 local time), existing
dirty canonical worktree preserved.

- `cargo check --offline --locked`: exit 0, existing 19 library warnings plus
  vendored crossterm warning; no newly introduced Rust warning.
- `ZENPI_HOME=/private/tmp/zenpi-subscription-view-tests-20261002 cargo test
  --offline --locked --test remote_headless --test remote_subscription
  --test remote_events --test remote_protocol --test remote_faults`: **26 pass**
  across five suites. Seven subscription cases cover wire bounds, durable turn
  delivery, broken output/refusal, tail/gap/count cap, Busy owner changed views,
  transient byte overflow, session/owner/revision drift and invalid request.
- remote_headless additionally exercises actual Agent/ProjectOwnerPool/
  DeviceRegistry reads while its actual owner mutex is held: view reads succeed,
  all emitted batches have unchanged cursor/no transcript, and revoked-device
  view reads refuse. The idle OpenAI-compatible backend is not invoked; this is
  real domain/ACL evidence, not live provider/TLS evidence.
- Auth/live_text/parity/subtab/workspace regression: **49 pass** across five
  suites with the same isolated ZENPI_HOME.
- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-subscription-view-verified-20261002`: exit 0. All main,
  JVM-test and instrumented Kotlin/Compose sources compile. **69 JVM tests pass**
  across 18 classes. New tests cover transient model/terminal/prompt/approval
  updates without transcript advance, offline exclusion/unknown status and
  atomic refusal for bad session/revision/cursor/type/bytes. Seven existing
  source/instrumented warnings remain. This is not APK/device execution.
- 44 Stage 2 Python tests pass; strict Blueprint, formatting and whitespace
  checks pass. All 36 items remain unaccepted.

Network gate revalidated: `cargo test --offline --locked --test remote_frame
real_loopback_timeout_remains_a_terminal_io_error -- --exact` exits 101 because
TcpListener bind at tests/remote_frame.rs:180 returns PermissionDenied / Operation
not permitted. This is an unmet environment-dependent gate, not waived. The
explicitly filtered non-socket frame rerun passes eight tests, one filtered.
No old APK or historical LAN receipt is relabeled as current-source evidence.

Final direct-source jars (under the output directory above), SHA-256:

```text
main.jar 1f31fb7150d1e9783fdc6335afb326965b46383c7045b35f2d6de97bf2152935
unit.jar b20a0a2831729db1707dbb38c7de884f820cd54f8d76b9bf4f8372ef61ec66c3
instrumented.jar b48c8c87a1cc6e3f25d98aa4a2be969127f7db0a3d7792ca7b53c55ea14f29b7
src/remote/subscription.rs dc86719ae14eea015419330259649f0fcd5a07c48f3b077e689d5c8268a2d4e6
src/host_pair.rs e59f3a9a37acef709f7afe2206b1f2fdde80ec5c3eeda8cf4a0dd7e98767393b
SubscriptionReducer.kt 99acfd923cf5bde72cb27d17e38e1732b917d8956dc835045f4cd835ab7ee4ad
HostClient.kt dafaf45e87ef430ea767e8c99a10924b5991b7e03a54a21d594eab6c809ed551
PairingViewModel.kt 0ec102779e84effb09ad3ec4091fc2301a9e25b774a472a763ed213710a7d3df
```

This note is provisional progress, not a checksum-valid acceptance receipt.
All observed command handles terminalized. Fresh host/APK TLS streaming,
mid-stream revocation/half-close, real camera/LAN, supported-API UI/lifecycle,
worktree 1..64 executable scheduling, full PTY/bulk controls, migration/uninstall
and signed release/security gates remain required. No overall blocked status:
source/test progress remains possible despite the network/device limits.
