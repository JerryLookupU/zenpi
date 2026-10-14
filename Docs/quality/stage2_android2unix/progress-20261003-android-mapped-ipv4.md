# Android Mapped IPv4 Invite Boundary

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

Android QR parsing now applies Rust-equivalent address safety to IPv4-mapped
IPv6 literals. Mapped broadcast and mapped link-local addresses are normalized
to their IPv4 form before rejecting unspecified, multicast, link-local and
broadcast destinations. This prevents an unsafe endpoint from bypassing the
Android parser merely by using an IPv4-mapped IPv6 spelling.

Added paired malformed invite tests for mapped broadcast and mapped link-local
addresses. Physical camera and LAN execution remain separate gates.
