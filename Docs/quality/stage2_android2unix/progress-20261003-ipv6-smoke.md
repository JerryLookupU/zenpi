# Direct IPv6 Smoke Harness Support

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The desktop, headless and shared-pair Android smoke harnesses now select `AF_INET6` for a
literal IPv6 `--lan-ip` at both the preflight bind and listener-close probe;
IPv4 continues to use `AF_INET`. The desktop harness's address validator now
accepts the same RFC1918/ULA literal set as the shared pair harness. This keeps the harness consistent with the
Rust invite/parser IPv6 path and avoids rejecting valid ULA direct-LAN runs due
to an IPv4-only Python probe.

Validation:

```text
py_compile: OK
Ran 33 desktop/headless smoke-validator tests ... OK
git diff --check: OK
```

The current sandbox still cannot bind sockets or start adb, so this change does
not claim an actual IPv6 phone/LAN connection.
