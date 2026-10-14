# Rust TLS / QR Pairing Progress

Canonical Master progress, 2026-10-01 20:34..20:44 +08:00. This is not a final
acceptance receipt. Blueprint checkboxes remain unchanged; the full Android
application, LAN camera scan, persistent device authorization, shared desktop
state and synchronization objective remains active.

## Implemented

- `src/remote/pairing.rs`: persistent private TLS identity, strict invite URI,
  certificate DER SHA-256 fingerprint, six-digit code, 32-byte random token,
  1..300 second expiry with monotonic and wall-clock enforcement, atomic one-use
  admission, refresh/cancel invalidation, redacted Debug output.
- `src/remote/listener.rs`: opt-in TLS 1.3 / ALPN zenpi/1 TCP listener,
  connection cap including incomplete handshakes, absolute deadline socket,
  bounded handshake timeout, tracked shutdown sockets and joined worker threads.
- `src/host_pair.rs`, `src/core.rs`, `src/lib.rs`: real `zenpi host pair` CLI,
  interface/advertise selection, full-screen terminal QR, safety code, fingerprint,
  countdown, refresh/cancel and narrow-terminal refusal. Non-TTY/`--plain` has
  a sensitive one-time invite output for controlled integration clients.
- `Cargo.toml`/`Cargo.lock`: rustls promoted to production dependency; rcgen and
  qrcode generate identity/QR using existing libraries. quircs is test-only and
  independently decodes the rendered terminal module pixels. Resolved rcgen and
  time dependencies declare Rust 1.88 compatibility, matching this repository.

The production process now performs TLS, hello/welcome and one-use pair
admission. Its response explicitly says `control_available=false` and
`device_registered=false`, then closes. This is transport acceptance, not an
end-state substitute: S2-013 device registry/ACL, S2-014 real host adapter,
S2-019 common routing and S2-020 durable event replay are still required.
No provider, tool or shell is initialized by this pairing entry point.

## Observed Verification

Working directory `/Users/mac/Github/zenpi`; inherited local environment;
loopback sockets and temporary identities only, no provider network calls.

| Command | Exit | Evidence |
|---|---:|---|
| `cargo test --locked --test remote_tui_pairing --test remote_listener --test remote_pairing --test remote_protocol --test remote_frame --test headless_protocol` | 0 | 3 + 4 + 6 + 8 + 9 + 45 = 75 passing tests |
| `cargo check --locked` | 0 | Integrated host entry and production dependency typecheck |
| `cargo fmt --check` | 0 | Formatting |
| `git diff --check` | 0 | Whitespace |
| `python3 tools/validate_stage2_android2unix_blueprint.py --strict` | 0 | 36 unfinished checklist items; metadata freshness |

Tests independently decode IPv4 and IPv6 terminal QR pixels to the exact URI.
A spawned production binary negotiates TLS, consumes the token once, refuses
the duplicate and releases its listener on exit. TLS tests cover wrong
certificate, incomplete handshake cap/timeout, absolute deadline and shutdown.
Identity tests cover private permissions, persistence, malformed/public/symlink
refusal. Eight simultaneous token consumers produce exactly one winner.

An initial TLS test exposed accepted sockets inheriting nonblocking mode on
macOS; explicitly restoring blocking mode fixed the failure. The latest full
targeted run above passed after that repair. Earlier failing results are not
represented as acceptance.

A task-scoped product tmux session (not an agent worker) ran the real desktop
TUI at 110x70: QR/address/code/fingerprint/countdown appeared. Sending `r`
refreshed the invite; resize to 80x24 displayed the required-size fallback;
Escape terminated the product session. Plain text capture does not prove
terminal color fidelity or real camera scanning. Independent pixel decoding
proves QR geometry, but a screenshot/camera/emulator roundtrip remains needed.

No Android code, APK, emulator execution, complete Rust suite or release build
was verified in this window. User explicitly confirms computer use and the
local Android emulator may be used for the forthcoming validation loop.
No real execution worker or cron was launched, and no commit/push was made.

## Next Frontier

Build the native Android Kotlin/Compose project (S2-030), strict invite parser,
camera scanning and human safety confirmation (S2-031), certificate-pinned TLS
and bounded codec (S2-032), then run it against this production host in the
emulator. Implement device key/ACL/revocation and connect to the existing
host-owned project/session runtime rather than a demo echo server. Preserve
the full Blueprint's workspace/worktree, concurrency, approval, replay,
lifecycle, security, packaging and parity requirements.
