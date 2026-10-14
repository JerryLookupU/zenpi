# Rust Release Refresh

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The current worktree was rebuilt with the locked offline release profile:

```text
rtk cargo build --offline --locked --release: 0 errors, 20 warnings
target/release/zenpi sha256:
2547bd43cb671b6382365ad8d4591e389221dbbb55719fa2712eab170d715a98
```

The production host smoke was run against this exact binary. It failed before
QR invite emission with `Operation not permitted`; the receipt records
`passed=false` and `blocked_by_sandbox=true`. No Android or physical-LAN claim
is made.
