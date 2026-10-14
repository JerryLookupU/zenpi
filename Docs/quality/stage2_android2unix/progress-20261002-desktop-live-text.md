# Desktop Live Text Progress

Provisional S2-019/S2-020/S2-036/S2-054 implementation. Previous turn added and
tested remote-owned prompt preview. This turn wires local desktop model requests
into the Android-readable live projection rather than limiting synchronization
to requests originating on the phone. Full objective and all 36 unfinished
Blueprint rows remain intact. No worker launch, nested agent, commit or push.

## Actual Changes

TuiRequest captures an optional original project for locally submitted discussion
model jobs. Both immediate model submission and queued model submission set it.
A shared eligibility helper refuses arch, shell, resource-control and remote-owned
jobs; remote jobs retain their separate device-scoped prompt-status preview.
An initial edit matched the user-shell submission site rather than model site;
inspection caught it, removed it and added explicit eligibility tests before
final verification. The shell route never forwards provider events through this
preview and is now structurally excluded.

The production worker captures the actual session ID after acquiring its Agent.
A LiveDesktopText guard feeds the existing provider callback into the same
4096-byte/16-entry, complete-line, redacted transient buffer from the preceding
turn. Guard Drop removes it on return, error, cancellation or panic unwind.
The original project/session stays attached even if the desktop's selection
changes while execution is running. This is the existing single model runner,
not a new session writer, worker or scheduler.

After the existing WorkspaceList ACL checks, the TUI mailbox adds a nullable
desktop_stream_text to each visible project, reading only that project's current
session preview. The normal listener rechecks device currency before output.
This field has no request ID, state or control capability. Read permission to
inspect local output does not become cancellation, steer or approval permission.
Android strictly accepts null/absent or a string of at most 4096 UTF-8 bytes with
no NUL, shows the text separately in connected/error live states and never
serializes it into ProjectionCacheCodec or advances a cursor/ACK for it.

## Verification

- Rust preview/headless-command suites: seven passed, two socket-dependent
  tests explicitly filtered. Includes guard normal-return/unwind cleanup and
  foreign project/session absence, alongside existing privacy/cap/remote-owner
  tests and production non-TUI cancellation integration.
- A focused TUI library test passed: local model captures its original project;
  shell/resource/arch/remote inputs remain unbound. This checks production
  request preparation, not an actual terminal/HTTP/Android runtime.
- Parity/headless/workspace/subtab/auth regression: 45 passed across five suites,
  with a permitted temporary ZENPI_HOME, no user configuration writes.
- Direct Kotlin/Gson/JUnit desktop-preview codec test: one passed, checking
  compatibility, exact byte cap, one-over, multibyte overflow, NUL and wrong type.
- All 44 Stage 2 Python tests, strict Blueprint validation, cargo fmt --check
  and git diff --check passed after correcting only the new TUI formatting.
- The final protocol/event/fault/live-text regression passed 23 tests across
  four suites. Initial cargo check passed with 20
  existing warnings; the focused library/integration runs compile the TUI edits.

Added Compose coverage checks read-only desktop text, absent cancel/steer,
offline hiding and cache omission without cursor advancement. It is not executed
in this environment because Android build/device access remains restricted.
The desktop smoke harness expected count now includes both new preview UI tests;
old API matrix receipts are unchanged historical artifact evidence.

## Remaining Acceptance

Production terminal/HTTP/TLS and Android screenshots must show partial local
desktop output before completion, project selection while running, final replay
replacement, revoked/read-only devices and reconnect without persisting preview
text. The new Compose test and rebuilt APK lifecycle matrix remain pending.
Arch, user-shell and PTY projections, long-lived event-channel push, bounded bulk,
independent worktree execution scheduling, actual phone camera/LAN and release/
migration/security gates are still required. This polling preview is useful
intermediate synchronization, not the whole requested final state.
