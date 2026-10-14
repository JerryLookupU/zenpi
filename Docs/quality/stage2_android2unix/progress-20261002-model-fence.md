# Android Model Catalog Continuation Fence

Provisional Stage 2 progress at 2026-10-02. No blueprint item was promoted.

Model catalog requests now require the requested offset to be either the
current page's `nextOffset` or its adjacent previous offset. This mirrors the
Rust snapshot protocol and prevents arbitrary jumps that could accidentally
mix pages from different catalog generations. The bounded local page history
continues to service previous-page navigation without a network round trip.

The isolated Android source check completed with `OK (258 tests)`. Rust
listener tests were also attempted under an isolated `ZENPI_HOME`, but all
network cases fail at socket bind with the sandbox's `Operation not permitted`;
no LAN receipt is claimed from that run.
