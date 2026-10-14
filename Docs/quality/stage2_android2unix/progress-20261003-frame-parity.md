# Android/Rust Frame Boundary Parity

Provisional Stage 2 progress at 2026-10-03. No blueprint item was promoted.

The Android frame codec and Rust bounded frame codec are now covered by their
respective existing test surfaces: network-order length prefixes, control vs
bulk limits, strict JSON/UTF-8, duplicate fields, truncation, partial reads and
partial writes. The cached Android source gate remains green at 258 tests, and
the Rust frame/fault targets are exercised offline. No connected socket claim
is made because the environment denies binds.
