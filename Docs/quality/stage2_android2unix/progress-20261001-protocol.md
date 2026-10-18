# Stage 2 Protocol Progress: 2026-10-01

This is a canonical Master progress note, not a worker handoff or final
acceptance receipt. No Blueprint checkbox was advanced. The full Android / LAN
QR / Rust desktop synchronization objective remains active and unverified.

## Current Identity

- Repository: `/Users/mac/Github/zenpi`
- Branch/revision: `main`, `1a8ff1acf72e0816fad503a05df2b1d18e07361b`
- Baseline: current dirty worktree, preserved without reset/stash/checkout
- Requirement digest: `9887f784c944db898610e136dca2774496abb2f80b3e5a8fdd6a0b32659e009d`
- Specification digest: `403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28`
- Implementation window: 2026-10-01 20:26..20:33 +08:00

## Canonical Changes

S2-003 protocol primitives and draft compatibility contract now exist at
`src/remote/protocol.rs`, `tests/remote_protocol.rs` and
`Docs/stage2_android2unix_protocol.md`. S2-010 framing primitives now exist at
`src/remote/frame.rs` and `tests/remote_frame.rs`. `src/remote/mod.rs` and the
`src/lib.rs` export make the primitives testable in the real library; they do
not start a listener or change the existing TUI/headless entry modes.

Frame limits are checked before allocation, output is bounded before emitting
the prefix, unknown schema/message fields are refused, writes structurally
require expected owner/revision, and transport cursors are session-bound.
Read/write timeout ownership and terminal partial-frame handling are documented.
Host authentication, owner validation, durable deduplication and dispatch are
not implemented by the codec.

## Execution-State Correction

The existing canonical ledger claimed an `authenticated_live` S2-001 mock
worker, with PID `67787001`. Direct `tmux has-session` reported its socket
missing and `ps` rejected the PID. There was no result manifest. Reconciliation
retired that record and released its request lease; no replacement was launched.

The controller now refuses canonical mock transport and probes process
identity before reporting live/authenticated workers. The selector's stale
specification digest was reconciled and the Gantt regenerated from current
authority/state. Current status: zero live transports, zero authenticated goals,
zero outstanding requests, no pending handoff. No Stage 2 cron is installed.

The ignored `.ops/stage2_android2unix/` controller is local runtime, not a
versioned deliverable. Its 15 passing fixture tests do not prove full real-TUI
execution safety. Atomic submission leases, continuation request accounting,
ownership-conflict admission, selective build bootstrap, handoff integrity and
completion receipts still need a full audit before automated real admissions.
In particular, the fixture integration test tolerates missing fixture Gantt
generation; that passing test must not be treated as proof of Gantt acceptance.

## Verification

All commands ran in `/Users/mac/Github/zenpi`, using the inherited development
environment without provider credentials added to the remote protocol. The
loopback test binds only ephemeral localhost and sets a 50 ms socket read
timeout. These are observed command outcomes, not full release gates:

| Command | Exit | Observed criteria |
|---|---:|---|
| `cargo test --locked --test remote_protocol --test remote_frame` | 0 | 8 protocol + 9 frame tests pass |
| `cargo test --locked --test headless_protocol --test remote_protocol --test remote_frame` | 0 | 45 existing headless + 17 new remote tests pass |
| `cargo check --locked` | 0 | Integrated library/binary typecheck succeeds |
| `cargo fmt --check` | 0 | Formatting matches repository configuration |
| `git diff --check` | 0 | No patch whitespace errors |
| `python3 tools/validate_stage2_android2unix_blueprint.py --strict` | 0 | 36 unfinished items; current selector/Gantt digests agree |
| `python3 tools/test_validate_stage2_android2unix_blueprint.py` | 0 | 5 structural tests pass |
| `python3 .ops/stage2_android2unix/tests/test_controller.py` | 0 | 15 fixture tests pass, including two new state-isolation/liveness tests |

Existing vendor unused-parentheses and host dead-code warnings remain. They
were not changed as part of this work. The complete Rust test suite and release
build were not run, and no Android build/instrumentation test was run.

## Next Required Work

Authority/validator acceptance S2-001/S2-002 remains open. Complete the execution
controller audit rather than enabling an unsafe cron. Complete protocol/frame
Master receipts before accepting S2-003/S2-010. Then implement S2-011 TLS/TCP
listener and S2-012 host identity/pinned-key one-use invite, followed by S2-021
full-screen scannable desktop QR. Build the Android client and wire it into the
real host-owned command/replay domain; do not substitute a mock-only demo.

The machine has Java 17 and Android SDK platforms 34/35/36 plus emulator/NDK
tools. `adb devices -l` reported no connected device. Two AVD names exist, but
neither was verified running. No repository `android/` project exists yet;
API 31 and the full 31/34/35 lifecycle matrix are not verified.

## Source Digests

| Path | SHA-256 |
|---|---|
| `src/remote/protocol.rs` | `726c92e9c44229cf43fea188bae02802615b2f367a4261dd8c3c9c8ccc580f6e` |
| `src/remote/frame.rs` | `27464d8244fc3f2f6a7425580f710d087ee3931c36060bed435016b908c03746` |
| `tests/remote_protocol.rs` | `c7eb97ce1e964308a0b14ea86ae3ba40297fda3e3e112441a9e77e46587392cc` |
| `tests/remote_frame.rs` | `8657203d296c306e017828a4751e4a2a35cc333bc62c82bd1171907497090540` |
| `Docs/stage2_android2unix_protocol.md` | `72b0ac806603b565d957a27c50691e795de6158c46e4e8d24931c5cedbad40b7` |
