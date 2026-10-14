# QR Token Canonicalization

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

Rust and Android now enforce the same unpadded URL-safe base64 canonical form
for the 32-byte QR token. Rust previously accepted alternate encodings that
decoded to the same bytes while Android rejected them; the host parser now
re-encodes and compares the token before admission. Added paired Rust/Android
tests for padding refusal.

Evidence:

```text
remote_pairing: 7 passed
Android cached source/JUnit: OK (270 tests)
```

This improves QR interoperability and fail-closed parsing but does not replace
physical camera or direct-LAN execution evidence.
