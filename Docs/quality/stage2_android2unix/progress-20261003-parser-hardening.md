# Cross-Language Parser Hardening

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The Android parser now refuses malformed JSON object types at the remaining
metadata/authentication/pairing boundaries instead of relying on Gson's
`asJsonObject` exception. This keeps malformed persisted state and hostile
receipts on the common `WireException` fail-closed path:

- pairing result must be a JSON object;
- authentication result must be a JSON object;
- cached projection and pending workspace records must be objects before
  decoding.

Evidence:

```text
Android cached source/JUnit: OK (267 tests)
Rust cargo check --offline --locked: 0 errors
```

Gradle packaging, connected Android tests, QR camera scanning and physical LAN
remain separate environment-gated requirements.
