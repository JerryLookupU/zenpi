# Frame Socket Gate Integrity

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The Rust frame target's deterministic partial-read/write, truncation and fault
cases pass offline. Its one real loopback timeout case is now explicitly
`#[ignore]` because it requires TCP bind/connect permission; a sandbox denial
must not turn the default offline suite red or be mistaken for a codec defect.

Evidence:

```text
remote_faults: 3 passed
remote_parity: 5 passed
remote_frame: deterministic cases pass; loopback case requires --ignored
```

Run `cargo test --offline --locked --test remote_frame -- --ignored` on a
socket-capable host to exercise the real timeout path.
