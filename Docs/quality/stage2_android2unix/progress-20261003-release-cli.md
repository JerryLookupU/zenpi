# Release CLI Smoke Boundary

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The current release binary was inspected against the host-pair entry point. Its
argument/help surface is present in the Rust core usage contract, but invoking
the command in this sandbox stops at pairing storage initialization with
`Operation not permitted`; no identity or listener is created. This preserves
the fail-closed behavior required for a release binary when its configured
storage cannot be opened.
