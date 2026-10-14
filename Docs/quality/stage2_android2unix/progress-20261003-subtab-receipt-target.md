# Subtab Receipt Target Semantics

Provisional S2-031/S2-037/S2-051/S2-054 progress on 2026-10-03. Previous turn
repaired in-flight deep-link admission and compiled its lifecycle regression,
so it was progress. This turn inspected current authentication, response reader,
workspace/session/terminal receipt codecs and Rust subtab mutation sources.
No nested agents, worker admission, checklist promotion or unrelated cleanup.

## Repair

WorkspaceReceiptCodec previously accepted any 64-hex active_subtab_id for a
successful subtab operation after owner/project/revision checks. For select,
this allowed a receipt claiming another tab; for close, it allowed the removed
tab to remain active. HostClient calls onWorkspaceConfirmed after receipt
validation and before fetching replacement projections, so the contradiction
could clear the encrypted pending request even if later reads failed.

The codec now requires select's active ID to equal the requested subtab and
close's active ID to differ from the removed subtab. These are properties
recoverable from the exact saved request, so they work on restart/retry without
relying on a potentially newer projection. Rename/move/concurrency are not
incorrectly required to activate their target. Add/close successor selection is
still host-owned; no index-derived identity is introduced.

Two JVM regressions check valid select/close/nonselected rename receipts,
wrong but well-formed IDs and a fragmented response. The wrong-target response
fails before its confirmation callback and leaves the request identity intact.
Rust producer tests now explicitly assert that select returns the target ID
and close returns the committed successor, never the removed ID. Existing
durable retry, restart, non-destructive close and ACL tests remain in place.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-subtab-receipt-target-20261003: exit 0; main/unit/device
  sources compile and 387 JVM tests pass. Existing deprecation warnings remain.
- Stage2 Python discovery: 110 passed. Lifecycle verifier: 21 passed;
  Blueprint tests: 5 passed. These do not execute a connected phone.
- With ZENPI_HOME=/private/tmp/zenpi-subtab-receipt-host.uhSP2S,
  rtk cargo test --offline --locked --test remote_subtab: 10 passed.
  remote_workspace subtab filter: 3 passed, 65 filtered; directory containment/
  close/exact-retry filter: 1 passed, 67 filtered. The latter two filters were
  rerun successfully after adding the producer assertions. Local/Echo ownership
  tests do not establish Android TCP/LAN behavior.
- Strict Blueprint, git diff --check and Cargo formatting pass. All 36 rows
  remain [ ].
- Cargo tests replaced the debug binary and invalidated its prior attribution.
  The first verify-only correctly refused it. stage2_host_build.py then rebuilt
  the production binary and published fresh attribution; final verify-only passes.
  Rust production source inventory is unchanged.
- APK verify-only still refuses missing current-source provenance. No stale
  APK install, unchanged blocked Gradle/ADB/TCP attempt or physical QR/LAN run.

```text
WorkspaceReceiptCodec.kt b59d4821fbf8c9207fab9fc8b0239c239f113aa840e35e44f07f1f8f92d6d99b
WorkspaceReceiptTest.kt a577e0f4ba7493d451e79e53ef56f37521b51f5f247f9c7b73b8f8001b3e21ed
remote_workspace.rs 18cd73533f4261d89ff5830b1c9ebef9c5ee0290f866dcd029d3582bb4e30f13
main.jar f70f55d5453927bb0c8b114676d2f93afd39eb0a403f14137ad4fd8d67163886
unit.jar 0b498b0d1061beaf9cc2a6da6a40de081f143415baa0bcdcbec5933dd0e8063a
instrumented.jar 9c01be541d722e853a403fed9a45163d9a8b57855425f856a510334d2142f2c5
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
debug-host-binary 57b52b80beda551642c47cd1f404f2fd1400b7a16699f91401402cb77db227a8
```

Goal remains active. Current APK, actual physical QR/direct LAN message
roundtrip, complete desktop parity, supported API lifecycle and signed release
are incomplete or unverified. This repair is not whole-item acceptance.
