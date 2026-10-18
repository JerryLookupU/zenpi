# LAN-First QR Address Selection

Provisional S2-011/S2-012/S2-021 progress. Previous goal turn repaired private
PEM projection leakage. This turn improves the requested desktop QR to LAN
entry: the previous wildcard-bind path relied exclusively on connecting an
unbound UDP socket to a documentation-prefix address. That can fail without
a default route or choose a public/CGNAT VPN route instead of local Wi-Fi.
Full native Android/Rust desktop synchronization goal remains active; all
36 authoritative Blueprint rows remain `[ ]`.

## Implementation

New src/remote/lan.rs reads getifaddrs directly, with an owning freeifaddrs
guard and existing libc dependency. It retains enabled, nonloopback,
non-point-to-point IPv4/IPv6 interfaces, excluding unusable and scoped IPv6
addresses. IPv4 bytes are decoded in memory order (network octets), avoiding
host-endian inversion. Selection is deterministic/deduplicated: unique
same-family private LAN wins even without a route; several private LANs need
a matching route or explicit --bind/--advertise. No private candidate retains
the prior valid route fallback. No packets or address scanning are sent.

host_pair::advertise_address now calls this production path only for wildcard
binding without explicit advertisement. Operator exact bind and advertisement
retain existing behavior and family/interface checks; no global routing or
firewall changes. The production QR issuer/renderer is unchanged.

No claim that an interface is phone-reachable: multiple physical networks,
virtual bridges, non-point-to-point VPN interfaces and firewall rules remain
operator/environment constraints. Multiple private addresses intentionally
fail instead of choosing by enumeration order; explicit selection is the
escape hatch. Public-route fallback is compatibility, not NAT traversal.

## Verification

Cwd /Users/mac/Github/zenpi, observed 2026-10-02T01:03:45Z UTC.
Test environment ZENPI_HOME=/private/tmp/zenpi-lan-selection-20261002.

- `cargo test --offline --locked --test remote_lan --test remote_pairing`:
  **13 pass**, exit 0 (7/6). Tests cover offline route failure, public/CGNAT
  route precedence, IPv4/IPv6 family, multiple candidates, duplicate/order,
  invalid address classes, actual local interface enumeration, and selected
  addresses passed through actual invite URI/SAS/QR rendering.
- `cargo test --offline --locked --lib host_pair::lan_selection_tests`:
  **2 pass**, exit 0. Confirms exact bind/explicit advertisement bypass
  discovery and cannot cross exact interface/family. Initial fixture compile
  failed because HostPairOptions has no Default; fixed tests to use existing
  argument parser, without adding a production default API.
- `cargo test --offline --locked --test remote_lan --test remote_pairing
  --test remote_tui_pairing terminal_pixels_decode_to_the_exact_invite_with_quiet_zone`:
  **1 pass**, exit 0; the first two suites have zero matches under this filter,
  not extra passes. The QR pixels decode to exact IPv4 and IPv6 invites.
- Stage2 Python smoke 34, Blueprint tests 5, lifecycle tests 9: **48 pass**.
  Strict Blueprint validation exits 0 with all rows incomplete. Cargo fmt,
  git diff check and cargo check --offline --locked pass. Existing warnings
  remain. No new Android APK/device/TCP/LAN receipt, no permission bypass,
  agents, commit/push, or historical receipt edits.

SHA-256:

```text
src/remote/lan.rs 4ce6a8c647e58ca43887754c3cfa7d61ed31f2bc97af29bfb86c52ae1f075080
src/remote/mod.rs c477ad1e1c26f7680a488f2ad9e2ceabb1d3839dcd54fcc1829d584c72393451
src/host_pair.rs 11b3caf437b6848864a9c37c9a38f690fa88215c1fd4707c1b28479756da6438
tests/remote_lan.rs 0c1a0edc5ddd626dfd9aa87e97db51454388cc90b40eea2d19d8dfda5eca8f5e
```

Fresh supported-API APK/Keystore/lifecycle, physical camera QR and same-WiFi
LAN desktop/headless synchronization remain unverified. Also required:
interactive PTY/bulk, full stream delta replay/backpressure, independent
worktree scheduler quotas 1..64, release/migration. This change addresses a
real entry-path limitation, not substitute acceptance for that full scope.
