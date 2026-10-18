# Desktop Direct-LAN ULA Validation

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The desktop Android smoke harness now accepts the same literal private address
policy as the pair harness: RFC1918 IPv4 and RFC4193 ULA IPv6, with scope IDs,
wildcards, link-local, multicast, documentation and public addresses rejected.
IPv6 values are canonicalized before listener advertisement comparison, and
all direct probes use the matching socket family.

Validation:

```text
Ran 38 desktop/pair/headless route tests ... OK
git diff --check: OK
```

No actual ULA device/LAN result is claimed because this sandbox denies socket
bind and has no adb-connected phone.
