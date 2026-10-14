# Host Smoke Bounded-Termination Semantics

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The smoke harness now distinguishes a valid invite captured before the bounded
TTL from a host that failed before invite emission. A signal exit caused by the
harness terminating the process after capture is accepted only alongside a
valid invite; pre-invite exit remains a failure with its stderr diagnostic.

This avoids a false negative caused solely by waiting for the one-second
development TTL while keeping the production listener and sandbox failure
strict. Current sandbox execution still fails before invite emission with
`Operation not permitted`.
