# Host Smoke TTL Capture

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The host smoke harness now reads the production invite as soon as it is
emitted, validates it against the process start timestamp, and terminates the
bounded CLI immediately. It no longer waits for a one-second invite lifetime
to elapse before parsing the first line. The harness keeps stdout/stderr hashes
and redacts the one-use token from receipts.

Evidence:

```text
tools.test_stage2_host_smoke: 5 passed
```

This makes the smoke robust against short TTLs; it remains host-side evidence
and does not claim Android or physical-LAN success.
