# Host Smoke Failure Diagnostic

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The host smoke receipt now preserves a bounded stderr diagnostic and an
explicit `blocked_by_sandbox` marker when the production binary fails before
emitting its QR invite. The current run records:

```text
exit_code: 1
stdout: empty
stderr: zenpi: Operation not permitted (os error 1)
blocked_by_sandbox: true
passed: false
```

The marker is diagnostic metadata only; it never changes a failed host smoke
into a pass. This keeps the evidence useful for an operator rerun on a
socket-capable machine without weakening the release gate.
