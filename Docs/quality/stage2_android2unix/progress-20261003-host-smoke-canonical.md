# Host Smoke Canonical Token Check

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The production host smoke parser now applies the same QR token rule as the
Rust and Android clients: exactly 32 decoded bytes and canonical unpadded
base64url re-encoding. It also rejects reserved endpoint addresses before a
receipt is recorded. The smoke unit tests use a real deterministic 32-byte
token fixture rather than an alphabet-only placeholder.

Evidence:

```text
tools.test_stage2_host_smoke: 4 passed
```

The harness remains a host-side parser/CLI smoke. It does not claim Android,
camera, or physical-LAN communication.
