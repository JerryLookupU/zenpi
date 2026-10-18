# Read-Only Terminal Projection

Provisional S2-019/S2-020/S2-036/S2-054 progress, not accepted items. Previous
goal turn added desktop live model preview with verified tests. This turn exposes
the real desktop-owned PTY screen to the Android display, without executing
anything on Android or converting a read grant into shell authority. Full QR/
LAN/desktop objective remains active; all 36 Blueprint rows are unfinished.

## Production Wiring

TuiState::remote_terminal reads an existing PtyShell only; it never calls spawn
or write_input. It requires the requested project to be the current TUI project,
the captured PTY project/session to match the authoritative pool context, and
both the visible tab and PTY cwd to match the host's current stable-ID tab root.
The authorized TUI mailbox adds nullable desktop_terminal after normal listing
ACL checks. Listener current-device checks remain in force before each output.
The screen includes only project/session/subtab/revision, sanitized text,
truncated and read_only=true. Neither cwd/PID nor a control operation is added.

ensure_shell now records project/session scope and replaces the local shell
when project/session changes even at the same cwd. This avoids attributing an
old shell's screen to a new session. This affects local shell lifecycle and is
covered by scope-negative tests, but interactive full-TUI regression remains a
required release gate. Current local screen reads are bounded to latest 64
lines and 16384 UTF-8 bytes after redaction. Known tab/project roots are replaced,
control characters removed except newline/tab, registered secrets redacted.
Lines containing conventional secret markers are wholly withheld.

Android strictly parses exact fields, identity, stable-ID tab, u64 revision,
boolean flags and read_only=true. It renders only connected matching active
tab/revision, with monospace selectable output and truncation notice. The cache
codec's explicit project field set omits terminal text, so it cannot survive
offline restore or advance transcript ACK. No Android shell, remote PTY input,
resize or command side effect exists. Headless host still has no PTY surface.

## Failure And Repair

The initial build caught a temporary subtabs Vec borrow and an incorrect session
field assumption on ProjectTabMetadata. Binding the Vec and using the existing
project_session_cursor resolved these before final test compilation.

The first real PTY privacy test failed: shell command echo contained PTY_SECRET
as a quoted argument separate from api_key=. Existing assignment redaction
alone did not cover that entire echoed line. The projection now withholds full
lines containing secret markers; the same real-shell test passes. Unknown
unregistered secrets, arbitrary private paths and values wrapped into another
line are not guaranteed detectable. This remains a security-review residual,
not a broad claim of secret-proof terminal output. No raw PTY failure output
was written into a LAN receipt and no device data was published.

## Validation

- remote_terminal: three tests pass, including a real local PTY command emitted
  as two printf arguments to distinguish actual output from command echo. Tests
  also cover exact fields/scope, root/secret/control filtering and byte/UTF-8 caps.
- A focused TUI library test passes through real TuiState, ProjectRuntimeHost,
  existing PTY spawn/pump and production remote_terminal: absent shell gives
  none; actual output appears; foreign project/session and cwd are refused.
  It uses Agent::with_echo only for idle owner setup, not provider execution or
  a claim of prompt/TLS interoperability. PTY children drop at test end.
- Protocol/fault/live-text/terminal regression passed 21 tests across four
  suites before adding the real PTY test; final repeat passes 22 across four
  suites. Both focused TUI preview/terminal library tests also pass together.
- Parity/headless/workspace/subtab/auth regression: 45 passed across five suites
  with a permitted temporary ZENPI_HOME. cargo check passed after the borrow/
  scope repairs, with 20 existing warnings.
- Direct Kotlin/Gson/JUnit terminal parser: two tests passed for compatibility,
  scope, strict fields, u64/type/flag/control/text failures. This is not a rebuilt
  APK or Compose run.
- 44 Stage 2 Python tests, strict Blueprint validation, whitespace checks and
  final formatting check pass. Initial formatting differences were confined to
  newly added TUI blocks and corrected before the final check.

New Compose coverage asserts matching online display, revision mismatch hiding,
offline absence and omission from cache with unchanged cursor. It remains
unbuilt/unexecuted because external Gradle locks, adb and socket listeners are
restricted. Desktop smoke counts were advanced for the new UI test, without
altering historical API matrix receipts. No nested agent, commit or push.

## Remaining Acceptance

Rebuild new artifacts and observe the terminal through actual authenticated TLS
and Android screenshots; verify active-root/session switches, revocation,
read-only grants, background/reconnect and no cached terminal text. Full remote
PTY interaction, bounded bulk/stream transport and complete TUI parity are not
implemented by this read-only screen. Live subscriptions, worktree scheduling,
actual phone camera/LAN, migration and signed release/security gates remain
part of the unchanged goal.
