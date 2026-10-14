# Android Smoke Marker Hardening

Provisional Stage 2 progress at 2026-10-03. No checklist item was promoted.

The scanner, headless and desktop Android smoke validators now match
instrumentation success markers as complete output lines. A substring such as
`OK (120 tests)` can no longer satisfy a requested `OK (12 tests)` count, and
the single-test recovery checks use the same exact-line rule. This keeps a
successful QR/LAN receipt tied to the exact selected test surface.

Validation:

```text
py_compile: OK
Ran 39 smoke-validator tests ... OK
```

Real adb, camera and physical-LAN execution remain environment-gated and are
not claimed by this source-level change.
