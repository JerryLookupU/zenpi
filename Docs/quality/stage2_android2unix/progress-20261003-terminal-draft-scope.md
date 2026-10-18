# Terminal Draft Scope

Provisional S2-036/S2-052/S2-054 progress at 2026-10-03. Previous goal turn
was verified background session-race implementation progress. Current-source
terminal inspection found drafts keyed only by host/session/terminal instance,
and write construction did not check the preview's project/session identity.
No nested agents, checklist promotion, worker launch, commit/push or cleanup.

## Changes

TerminalCommandCodec now resolves the target through TerminalPageCodec.terminal,
which requires the preview's project/session plus active tab/revision to match
the selected projection. Foreign preview scope cannot construct input or resize
even with matching terminal and tab IDs. Existing durable request identity,
owner/workspace CAS, receipt validation and host PTY checks are unchanged.

Compose input, Enter mode, rows and columns now reset on active-tab/revision
or workspace-owner replacement as well as host/session/terminal-instance change.
Ordinary output-text refresh retains unsent input. The existing narrow-width
instrumented test checks draft retention, revision/owner clearing, explicit
new-input dispatch and lost-control display. Its method count is unchanged.
One new JVM test verifies foreign scope cannot produce either input or resize
and valid envelopes retain the exact scoped identity.

## Verification

- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-terminal-draft-scope-final-20261003:
  exit 0, main/unit/instrumented Kotlin compiles, 346 JVM tests pass.
  Existing warnings remain; Compose instrumentation was compiled, not executed.
- ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home cargo test --offline
  --locked --test remote_terminal
  terminal_screen_has_exact_scope_and_removes_roots_secrets_and_controls
  -- --exact --test-threads=1: exit 0, 1 pass. This targeted projection/redaction
  test is not PTY input, device or TLS/LAN acceptance.
- Python Stage 2 tooling discovery: exit 0, 86 pass; Blueprint validator unit
  tests: exit 0, 5 pass. Strict Blueprint, cargo fmt --all -- --check and
  git diff --check pass. All 36 authoritative rows remain unfinished.
- Host release --verify-only passes for unchanged Rust source. Android build
  --verify-only refuses missing APK build provenance. No old APK installed or
  unchanged blocked Gradle/socket build repeated.

```text
main.jar 8f16ff5401f95be7d966cfafd8e8196c0107e71f5d3c7e69719fb10110cc1738
unit.jar ac0c07aaaefe7657b3280f7e0f6ad876a794c15269c7907fa3cab098bfb266f2
instrumented.jar d8ef1386511836e57e15e3318a2395904264bb69b2662edbc2c50a710a3d484e
MainActivity.kt 4a91b8c66e275deb5323ea532ab0fdc6ce7c8afb399e1c85c7f7529bfe2b18c2
TerminalCommandCodec.kt 27ab7038ded19dccf2c4b57cf5603fdbf98acef3d76ce4ff4a9c1549d8f09f48
TerminalControlTest.kt 208fdf789c6a4643149b8d2d39dbbedc0701f162aec56f3a73fe2ec20e96ab83
TerminalControlScreenTest.kt b3077412ef74a66185100c554f1095f4049b258943b0badcef09d0f4cd79673b
```

Goal remains active. Current-source APK, production desktop-to-Android TLS sync,
physical QR/direct LAN, API 31/34/35 connected lifecycle and signed installation/
upgrade still require their own behavioral acceptance.
