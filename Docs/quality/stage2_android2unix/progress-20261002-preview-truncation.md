# Bounded Remote Preview Truncation

Provisional S2-020/S2-036 work observed 2026-10-02T01:50:13Z. Previous
goal turn repaired cold foreground scanner admission and strengthened its
gates: progress. Full goal remains active, all 36 authoritative rows [ ].

## Implementation

Remote provider preview previously disappeared entirely on raw 4096-byte
overflow, while only 16 live slots served a headless host admitting up to 64
independent project lanes. LivePromptText slots now use MAX_PROJECT_TABS,
retaining bounded per-slot text and refusing slot N+1 without evicting active
work. It is not an implementation of configurable worktree quotas.

On overflow retain only the previously publishable prefix, discard incomplete
raw tail, mark truncated and ignore later text for this response. Never copy
or slice the oversized incoming chunk into a preview. A first oversized chunk
has empty text and explicit truncation. ResponseCreated resets the buffer;
terminal progress removes it. NUL remains fail-closed. Cumulative redaction
still applies to every projected prefix, including open private PEM blocks.

PromptProgress emits stream_text plus optional stream_truncated: true only
for the matching device/project/session/request active status. Kotlin checks
the exact allowed fields, true boolean, text presence, active state and 4096
UTF-8 byte bound. Compose renders a truncation indicator. Encrypted offline
cache strips both text and truncation; no transcript cursor, journal event or
ACK is advanced by preview. Subscription reducer accepts this metadata with
the same no-durable-cursor behavior. Desktop-local string-only preview retains
its conservative overflow withholding because it lacks truncation metadata.

Optional field compatibility is explicit: older exact-field clients refuse
this view rather than presenting a truncated prefix as complete. This does
not add unsolicited delta push, delta replay/ACK, paging or full parity.

## Verification

Cwd /Users/mac/Github/zenpi. Rust test environment
ZENPI_HOME=/private/tmp/zenpi-preview-cap-20261002.

- cargo test --offline --locked --test remote_live_text --test remote_workspace
  --test remote_subscription --test remote_subscription_tls: initial **52 pass**
  (9/23/10/10). After final cap-wide and UTF-8/wakeup coverage edits,
  cargo test --offline --locked --test remote_live_text --test remote_subscription:
  **20 pass** (10/10). Total distinct passing Rust tests **53**.
- Cap test checks every one of 64 slots at exact 4096 bytes, rejects slot 65,
  then admits it only after cleanup. Overflow tests cover safe TextDone prefix,
  unfinished split API key, oversized first chunk, response reset, full UTF-8
  prefix, and desktop overflow withholding. Existing private PEM all-split
  and unclosed-body tests still pass.
- New workspace projection test checks explicit flag, same-device scope,
  zero journal change and terminal cleanup. Wakeup regression checks one
  truncation change notification and no repeated wakeup from ignored chunks.
- TLS subscription 10-test suite uses real rustls over bounded in-memory
  encrypted pipes, not TCP/LAN. It passes existing security/owner/replay gates;
  no claim of a new truncation-specific real-socket or device gate.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-preview-cap-final-20261002: exit 0, main/unit/instrumented
  sources compile, **137 JVM unit tests pass** in 26 classes. New strict
  truncation parser test passes; cache/subscription tests check removal and
  cursor stability. Updated Compose indicator test is compiled ONLY.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint: **52 pass**.
- cargo check --offline --locked, cargo fmt --all --check, git diff --check
  and strict blueprint validator pass. Existing warnings remain.

SHA-256 source:

```text
src/remote/live_text.rs 459100ff4084ee0c7e703693b6d1d345bde8fec4f190bfdd9f83a8adab1cfd5e
src/remote/command.rs 3da73871cd484c153f858c585ed13afba1f185b9d358ca33be17053d150c4e99
tests/remote_live_text.rs 4970aef5cd93fce6c0e95002ed02b740afbe222b1b02af0778e9ccca23ae63ba
tests/remote_workspace.rs 64fa5775b87981016e04cd73644d364e00138f1a87fb62a5664057fa4568369e
tests/remote_subscription.rs 826d60ebfeac67d45174d8ecb71cda1975e97d7d9bd1b6d58b72cacfc5d838b3
PromptStatusCodec.kt 02f105c16becdfe1d93fd06a338fc74f5babe3c3f5648b33844b41c6adabd55d
MainActivity.kt bc57338663094c3b512b9112065e1cef6020271bd88509719198128c2442824b
PromptStatusTest.kt e581d2a83f97758c92b67b2e71f53a2fe941247bd0a84853f339d6470cb3bb20
SubscriptionTest.kt 252b96dea6147e13404b20030452b84bf3e2e44bd54ffd2147fd010e6248733c
ProjectionCacheTest.kt ad6cd6867baa3de3b9d1296b1073bf2d0d77454f68ace9cae9662b02e3ac864a
PromptScreenTest.kt 4aff75aca93dce3e82a2429c6aa0d81b86696eaf51360b3a3cce1ed0633529ea
```

Compiled evidence under /private/tmp/zenpi-preview-cap-final-20261002:

```text
main.jar 25d79cee51eccbf49861a1fdad1cea13cf209aaafff2cf443d856724cf0c7c4d
unit.jar f7e566a45c81bc3eb1fc95de771b0e173282f0ae40e45b830bb0ba0447661ea2
instrumented.jar 0dd80e287c2b91291162830fa6727ba98fe4eef2e1abe7ae0ea0841a61a37d52
```

No new APK/API31/34/35/physical QR/same-LAN run in this turn. Last goal
turn's Gradle wrapper permission blocker is not bypassed or relabeled as pass.
Per-worktree effective quotas, full TUI/headless parity, stream delta replay,
interactive PTY, fresh device verification and release/migration remain.
No agents/commit/push, user changes preserved, no acceptance promotion.
