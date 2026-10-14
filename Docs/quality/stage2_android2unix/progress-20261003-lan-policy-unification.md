# LAN Policy Unification

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The desktop Android smoke harness now shares the same RFC1918/ULA literal
policy and IPv6 canonicalization as the pair harness. Direct desktop, headless
and pair probes use matching IPv4/IPv6 socket families, reject scope IDs and
non-private addresses, and preserve the no-`adb reverse` invariant.

Validation:

```text
Ran 44 Android smoke route/receipt tests ... OK
git diff --check: OK
```

This remains source-level validation. The sandbox has no connected device and
denies network socket binds, so no LAN communication receipt is claimed.
