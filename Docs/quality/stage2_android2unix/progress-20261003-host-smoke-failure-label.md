# Host Smoke Failure Label

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The production host smoke receipt now reports a precise bounded failure label:
`invite not emitted; host_stderr=...`. This distinguishes a pre-invite host
initialization failure from a malformed or expired invite. The current sandbox
receipt remains `passed=false`, `blocked_by_sandbox=true`, with the Rust binary
reporting `Operation not permitted` before listener/QR emission.
