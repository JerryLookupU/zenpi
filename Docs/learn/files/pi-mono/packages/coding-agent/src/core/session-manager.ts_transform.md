# Transform Note: `pi-mono/packages/coding-agent/src/core/session-manager.ts`

> **Stale (2026-10-15, ZE-406):** the recovery semantics described below contradict the v2 audit. zenpi now follows pi: a fresh journal per launch per cwd, `-c` reopens the newest one for the cwd, and dangling tool calls are answered with synthetic results on open (ZE-236, ZE-400). Treat the rest of this note as history.

source_path: `pi-mono/packages/coding-agent/src/core/session-manager.ts`
source_hash: `404b056c6b60125470e2e3d94b310f99419a93b52739a848b208256fc307dc73`

The source owns JSONL headers, version migrations, append operations, and
resume/tree reconstruction. zenpi narrows the shape to a single append-only
journal with a versioned header, monotonic sequence, signed handoff records,
and a valid-prefix recovery path. Compatibility conversion stays in
`session.rs`; a malformed tail never overwrites earlier bytes. Session tests
cover round-trip, sequence, malformed input, and partial final lines.
