# Large Durable Transcript Display

Provisional S2-020/S2-033/S2-036 progress observed 2026-10-02T02:00:20Z.
Previous goal turn changed foreground read lifecycle and validation gates:
progress. Full native Android/QR/LAN/desktop objective remains active, all
36 authoritative rows [ ]; no acceptance promotion.

## Implementation

An oversized final turn previously requested snapshot replacement, but the
snapshot's reverse budget loop could immediately stop on that turn and return
an empty transcript. Project durable turn content through full redaction,
then a UTF-8-safe 12 KiB prefix with content_truncated: true. Snapshot's overall
truncation reflects both omitted turns and partial content. Host journal and
Agent content are unchanged. Replay still uses contiguous durable turn ordinals;
metadata remains omitted. This is bounded display, not complete bulk paging.

Android RemoteTurn keeps the explicit flag in event/snapshot parsing, cache
serialization/restoration and Compose. Marker must be boolean true. Display
retains a newest suffix bounded by both 64 turns and 96 KiB JSON-encoded bytes,
including escaping. Repeated large pages can persist the matching cursor with
this explicitly truncated display instead of endlessly failing oversized cache
commits. Legacy bounded pages/cache without the marker remain readable; client
accepts up to the prior 96 KiB per-turn boundary, not a blanket 12 KiB refusal
of historical saved data. Overall UI label is Transcript truncated, while
individual partial messages say Message truncated.

## Verification

Cwd /Users/mac/Github/zenpi; Rust tests use
ZENPI_HOME=/private/tmp/zenpi-durable-tail-20261002.

- cargo test --offline --locked --test remote_events --test remote_headless
  --test remote_subscription --test remote_subscription_tls: final **27 pass**
  (6/1/10/10). After adding large-turn TLS coverage, cargo test --offline
  --locked --test remote_subscription_tls: **11 pass**. Distinct Rust passes
  **28**. Tests cover redaction-before-truncation, UTF-8, unchanged original
  journal content, actual owner snapshot/replay, and real in-memory rustls
  encrypted delivery of >100 KiB original answer with redacted prefix/flag
  and cursor 1. This is not TCP/LAN/provider execution acceptance.
- Initial headless fixture had fixed assumptions for two durable turns; adding
  a third changed wire frame count, gap cursor and future ACK boundary. Final
  test updates those expectations; future ACK uses u64::MAX, not a fragile
  presumed ordinal. No product ACK rules changed.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-durable-tail-final2-20261002: exit 0, all main/unit/device
  Kotlin sources compile; **141 JVM unit tests pass** in 26 classes. Four new
  tests cover strict truncation, many large pages with cache/cursor roundtrip,
  escaped-content byte budget and durable flag cache restoration. New Compose
  offline indicator test is compiled ONLY, not device executed.
- Initial ResponseReader budget regression used a 20 KiB event rejected by
  the first strict-limit implementation before intended cumulative-page test.
  Fixture now uses 12 KiB event plus increased second-page padding, preserving
  first-page publication and second-page refusal. Client also explicitly
  retains compatibility with prior 96 KiB bounded content.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint: **53 pass**.
- cargo check --offline --locked, cargo fmt --all --check, git diff --check
  and strict blueprint validator pass, 36 incomplete rows. Existing warnings.

SHA-256 source:

```text
src/remote/events.rs f5802a9b6b94b0bba2f36c9b8e100c5989af7a0fae5a28dbbb5b3d1aef74681f
src/remote/host_adapter.rs 0a5eb38ffb2cba4d5a97ff5d210699f230adfd60d404b47c33164d510244c466
tests/remote_events.rs c12adfbe92f76c091d5ab6fc30ecd37c273f3bda3f2864881ba161b9a8c41f6a
tests/remote_headless.rs 18259028af3c79b9c4b13fd9d9c9c2f0118d3b12d9297a6b87625963e057dd6c
tests/remote_subscription_tls.rs 6265894d103a4e56ff3ef5c10aff53b37df9ee8b343ff452bd65db15e4007989
HostClient.kt 460d38fb676dfcbefd97b2461bf1b79f3eb22330d489540eac1be1cd07219d13
ProjectionCacheCodec.kt 799d044bfd249aaaa69efed82235285e35cc0dd360b262a22467fb68128db32f
MainActivity.kt 9e18c24aba6d113c93d460d21c59c1768ca5d4256480c7a8debedfc3206ce26c
```

Compiled evidence under /private/tmp/zenpi-durable-tail-final2-20261002:

```text
main.jar e6baa8e93d6e2126ddc9f8948e7f32680042c05e5a27c179b9a1c2ee71f39058
unit.jar af989ab8abcdc4f99641fcbf71f2a5fe3008ad9b28472e8e64f66f651dccdf4d
instrumented.jar 88d1f7513c8699a2a9ec68c8d0bf5df5641ebc753dc8d17a551f272b2b04ab6e
```

No fresh APK/device/camera QR/same-LAN run. Full omitted-content paging,
provider delta replay/ACK, per-worktree scheduler, complete TUI/headless parity,
interactive PTY, API31/34/35 current-artifact gates and release/migration remain.
No agents, commit/push, permission bypass or user-change rollback.
