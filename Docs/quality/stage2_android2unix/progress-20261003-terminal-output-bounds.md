# Bounded Desktop PTY Synchronization

Provisional S2-019/S2-020/S2-036/S2-050/S2-054 progress on 2026-10-03. Previous
goal turn added actual terminal geometry sync. This turn fixes unbounded raw
PTY output accumulation and refresh work in the shared desktop terminal path.
No nested agents, worker launch, checklist promotion, commit/push or unrelated
cleanup. No blocked Gradle/socket retry or stale APK installation.

## Product Change

The PTY reader now uses a 32-chunk sync FIFO, each read at most 8192 bytes.
Queued payload is at most 256 KiB, with one reader chunk pending on backpressure
and the fixed read buffer separately bounded. Full queues retain the same bytes
and retry in order without further slave reads. The existing atomic stop signal
cancels that wait at shutdown; no blocking send can deadlock reader join.
Disconnect also terminates the producer.

Each pump processes at most eight chunks, 64 KiB, instead of draining a queue
that can be replenished forever. Remaining FIFO data is processed on later
ticks. This is a byte budget, not a hard latency/CPU deadline. Existing local
input handling, host instance/cwd authority, 4000 retained lines, remote
redaction and 16 KiB screen/8192-byte history pages remain unchanged.

Unterminated CSI/OSC could previously grow the parser's pending vector without
limit despite bounded output chunks. It now retains at most 4096 incomplete
bytes and enters a small discard state for an oversized unfinished control.
The terminator is recognized across chunks, including OSC ESC/backslash split,
before ordinary parsing resumes. Discarded control payload is not displayed
as text. Ordinary FIFO output is never dropped; oversized incomplete controls
are deliberately ignored. This is not full VT emulation or raw-output storage.

## Verification

- Final shared pty_shell group: 13 passed, covering full queue refusal,
  FIFO preservation, stop cancellation, deterministic eight-chunk pump budget,
  oversized/split escape termination and existing shell/parser/redaction cases.
- Final real remote_terminal suite: eight passed. A real shell emits 3000
  ordered UTF-8 rows, over 600 KiB and beyond queue capacity, with no missing,
  duplicated or corrupt rows. Screen cap/truncation and tail marker remain
  intact. A separate large-output case drops the shell without pumping and
  completes shutdown within the fixture's three-second deadline. Existing
  geometry, sensitive soft-wrap and scope/control tests pass.
- TUI local_remote_preview_tests: two passed, including existing authenticated
  resize/input, exact retry and matching project/session/root projection.
- Stage2 Python discovery: 95 passed; lifecycle verifier: 18 passed;
  instrumentation and Blueprint tests: five each passed. Strict Blueprint
  cargo fmt --all -- --check and git diff --check pass, all 36 rows unfinished.
- This turn changed Rust only; Android sources retain the prior final 364-JVM
  source verification. No fresh APK or device run is inferred from it.
- A release build started before the final source additions finished with
  source-inventory mismatch and published no provenance. The same handle was
  observed terminal before a fresh build. Final debug/release attributed builds
  and verify-only checks pass. Final attribution is recorded below.
- APK verify-only still refuses missing current provenance before device use.

```text
src/pty_shell.rs 024bc854a872c4fd461059c739bb4ef123d9fdb62dda8565004c5d8d5769f8a9
tests/remote_terminal.rs 1931bce54985fe8b8ef40fbbc485324f6f553fcf5c7489982503916bd2eeb8d3
host inputs 5a988b2f25bbdc3fd5d124b629108fde71d9da4bc22cefca8af5ca795010238a
debug receipt 842d32f0b39975891422e4e0afa03c1f5c51308f50ef8159cea30c4e3ba9c6f3
debug binary 888d1bdc01edce47f6d47b19175ffe29398de17260345cf6b4a35b7981be7db4
release receipt 9b176abbef394821c08733ee9b52c5d044643cf0b152dc03575724b19ecebb26
release binary 2ca7c7f0204f799003dae4365e1b219998230062a97ee4a042da447135557f79
```

Goal remains active. Production Android/desktop TLS sync, physical camera
QR/direct LAN, current-source APK, API lifecycle matrix, full parity and signed
install/upgrade still require actual behavioral acceptance.
