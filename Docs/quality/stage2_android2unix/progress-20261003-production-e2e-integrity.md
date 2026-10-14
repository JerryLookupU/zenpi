# Production E2E Gate Integrity

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The production-binary host pairing tests now have an explicit live-network
boundary instead of silently passing when the sandbox refuses a listener bind:

- `tests/remote_host_e2e.rs` marks both production listener tests as
  `#[ignore]` and requires `cargo test --ignored` for execution.
- A missing invite is now a test failure with captured stderr; it is never
  converted into a successful skip.
- The tests use the real TLS listener path (no `--plain` shortcut), then pair,
  verify one-use consumption, and exercise the attached workspace grant/list
  path when the environment allows sockets.

Observed in this environment:

```text
cargo test --offline --locked --test remote_host_e2e
2 ignored (explicit live-listener gate)
cargo test --offline --locked --test remote_host_e2e -- --ignored
fails before invite emission: Operation not permitted (os error 1)
```

This is stronger evidence than a green auto-skipped E2E test, but it is not a
physical-LAN or Android connected-device receipt.
