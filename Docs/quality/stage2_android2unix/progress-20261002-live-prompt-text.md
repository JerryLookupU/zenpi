# Live Prompt Text Progress

Provisional S2-019/S2-020/S2-036 implementation, not accepted Blueprint items.
The previous turn repaired strict wire decoding and produced verified tests.
This turn moves the actual Android synchronization surface forward: remote-owned
provider output can now be inspected before the Agent unlocks at completion.
The full original QR/LAN/desktop goal is unchanged. No nested agent, commit,
push, external cache write or network-permission bypass.

## Production Flow

The existing desktop and non-TUI BackgroundRunner own the Agent/session while
processing a prompt, preventing inspect/replay from reading it midway. Existing
PromptProgress was already readable independently of that Agent lock but only
contained accepted/running/terminal status. It now has a private bounded
LivePromptText buffer keyed by device/project/session/original request.
Both production process_with_cancel_and_events callbacks feed provider events
to this buffer alongside their existing behavior. Authorized WorkspaceList
attaches stream_text only to the same device's active prompt status, after its
existing ACL/current-device checks. No parallel Agent/session/writer or execution
runner is created.

TextDelta appends; TextDone replaces accumulated text; ResponseCreated clears
the previous provider response, preventing tool-cycle text from concatenating
as one answer. Only complete lines or TextDone are published, with full retained
prefix redaction before every read. This prevents revealing a credential-bearing
line while the key/value spans provider chunks. Registered secrets and existing
assignment/header/Bearer/URL redaction are reused. Arbitrary unknown secrets are
not magically identified. Reasoning, tool arguments, warnings and provider event
metadata are not projected. At most 16 entries and 4096 raw UTF-8 bytes per entry
are retained; overflow/NUL withholds the whole preview. Terminal progress removes
it. A subsequent response may start a fresh bounded preview.

Android PromptStatusCodec accepts only exact old fields or those fields plus
stream_text. It requires accepted/running, exact owner/request identity, string
type, no NUL and 4096 UTF-8 bytes. The existing ViewModel listing callback
updates this transient status even if the later Agent-dependent replay is Busy.
Compose shows it separately below durable turns in connected/error live states.
The existing cache codec emits only status identity/state, never stream_text;
offline restore cannot pretend partial text was committed or advance its cursor.
The codec was moved, unchanged except for the optional field, to its own small
file so it can be exercised directly without the socket/Android dependencies.

This uses the existing authenticated foreground listing poll. It is not a
long-lived subscription, unsolicited event-channel push, per-delta ACK or full
stream parity for prompts initiated only on the local desktop. Those remain
explicit required work rather than being relabeled as satisfied by this preview.
Single-line answers may have no visible preview until TextDone: the line-boundary
privacy policy is deliberate, and final output remains the durable transcript.

## Verification

The production HeadlessHost cancellation test uses the real Agent, original
owner, runner, authenticated registry and mailbox with a controlled backend.
The backend emits a partial text line and blocks until cancelled. While the
Agent is held, the original device's listing exposes that partial line; another
authorized device has no statuses. Cancellation still runs exactly one backend
call, clears preview text, publishes cancelled and joins/releases the owner.
This is real domain/runner integration, not HTTP/TLS or physical phone evidence.

Four LivePromptText tests cover split key/value redaction and incomplete line
withholding; nonprojected reasoning/tool arguments; four scope mismatches;
TextDone replacement; overflow/no recovery until cleanup; exact byte/16-slot
caps with no eviction of another active preview; new-response reset; and NUL.
These tests plus the two non-socket headless command tests are rerun after the
final buffer change: six passed across two suites, with the two socket-dependent
headless tests explicitly filtered. Their production HTTP/TLS gates remain open.

Protocol/events/fault/live-text regression invocation passed 21 tests across four
suites before the final reset/NUL test was added. Broader parity/headless/
workspace/subtab/auth regression passed 45 tests across five suites using an
isolated permitted temporary ZENPI_HOME. Initial cargo check passed with 20
existing warnings. The first fmt check identified only new TUI callback layout;
that localized formatting was corrected.

Direct Kotlin 1.9.24/Gson 2.11.0/JUnit 4.13.2 compilation and execution passed
two PromptStatusTest tests, including optional text positives and wrong type,
terminal state, overflow and NUL negatives. An added Compose test checks actual
preview display, offline absence and cache text omission with unchanged cursor,
but is not built or executed here: external Gradle cache locks/device/socket
access remain prohibited. Desktop harness expected counts were advanced for
the new Compose test; old receipts/matrix remain bound to their old artifacts.
All 44 Python Stage 2 tools tests, strict Blueprint validation and diff whitespace
checks pass. All owned test/build command handles are awaited to completion.

## Remaining Acceptance

Rebuild app/test APKs and run production desktop/non-TUI TLS with a delayed real
HTTP provider, screenshot Android partial output before completion and confirm
replacement by final replay with no cursor change during preview. Run the new
Compose/offline test and supported-API lifecycle gates on the new artifacts.
Long-lived event transport, local desktop prompt streaming, bounded bulk/PTY,
per-worktree scheduling, actual phone camera/LAN and full security/release gates
remain required. All 36 authoritative Blueprint rows remain unfinished.
