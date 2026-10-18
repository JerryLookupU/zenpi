# Rust Desktop Release Build

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The production Rust desktop binary was built from the current worktree with
the locked offline dependency set:

```text
rtk cargo build --offline --locked --release
cargo build: 0 errors, 20 warnings (9 crates)
target/release/zenpi: 9,233,856 bytes
sha256: 532b2d0bece5f0d1d7427f0399c7b1e6e50d07760f8eff4b2908ec54266c024b
```

The warnings are existing dead-code/vendor warnings; no release build error
remains. Android packaging and connected LAN/QR execution are separate gates
and remain environment-limited.
