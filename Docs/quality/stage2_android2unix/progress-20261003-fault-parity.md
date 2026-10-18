# Fault And Parity Gates

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

Fresh Rust acceptance targets passed in isolated homes:

```text
remote_faults: 3 passed
remote_parity: 5 passed
```

The fault target covers partial/timeout/reset/half-read framing and non-receipt
writer behavior. Parity covers request-scoped subtab execution, root restoration
after failure/unwind and safe changed-root handling. These remain host-side
evidence; Android connected and physical-LAN gates are still unavailable.
