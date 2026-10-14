# Terminal Page Scope and Dialog Switching

Provisional S2-033/S2-036/S2-053/S2-054 progress at
2026-10-03T21:07:39+0800. Previous goal turn made verified global-workspace
publication progress. Current inspection found previous-terminal-page admission
checked only instance ID, navigation controls did not recheck displayed scope,
and Diff opening left other read panels open. No nested agents, worker launch,
checklist promotion, device launch, commit/push or cleanup.

## Changes

TerminalPageCodec requires the terminal and subtab projection to belong to the
selected project. Displayed history matches project/session, terminal instance,
active subtab and revision, not just instance ID. Previous-page chains require
continuous offsets and identical scope/digest/total; seven preceding pages are
retained. A nonzero initial page is refused. Both navigation controls use these
checks. Explicit refresh remains possible after error/foreign history only when
the current projection has a valid terminal.

PairingViewModel validates continuation before starting IO, verifies the full
scope before cached backwards navigation, handles broken retained history
without throwing into UI, and closes a stale successful read. History publication
validates its captured previous-page chain before updating state. Diff opening
uses the common close path to cancel and clear all existing read panels, including
session/terminal/task windows. Stale Diff success closes rather than remaining
loading. Neither operation adds shell execution or write authority.

Two new JVM tests cover full scope, project membership, unchanged output,
broken/reordered history, bounded retention and unanchored/skipped pages.
Existing narrow Compose history test adds foreign scope, revision, broken
backwards chain and missing-terminal fences without changing method count.
Dialog switching is source-inspected and compiled, not yet exercised through
a connected ViewModel/device workflow. Prepared desktop suite remains 66 tests,
or 67 with Doze. Session-suite documentation was corrected to its current eight
tests, including SessionCatalogScreenTest; harness behavior was not changed.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-terminal-page-scope-20261003: exit 0; main/unit/instrumented
  Kotlin compilation; 358 JVM tests pass. Existing warnings remain. Compose
  tests compile only, not connected execution.
- env ZENPI_HOME=/private/tmp/zenpi-terminal-page-scope-home cargo test
  --offline --locked --test remote_terminal -- --test-threads=1: exit 0,
  five pass, including real PTY projection/soft-wrap/redaction fixtures.
- With the same ZENPI_HOME, cargo test --offline --locked --test remote_workspace
  terminal_history_pages_reconstruct_retained_redacted_screen_and_refuse_stale_or_revoked_reads
  -- --exact --test-threads=1: exit 0, one pass. Real PTY history reconstruction
  and scope/refusal tests, not production TCP/Android/LAN acceptance.
- Python Stage 2 discovery: 93 pass; lifecycle verifier: 17 pass;
  instrumentation verifier: five pass; Blueprint verifier unit tests: five
  pass. Strict Blueprint validator, cargo fmt --all -- --check and
  git diff --check pass. All 36 authoritative rows remain unfinished.
- Release host --verify-only passes for unchanged attributed Rust inputs.
  Debug artifact attribution was invalidated by Cargo tests; a fresh attributed
  build completed after those terminal tests and verify-only passes. Android --verify-only still refuses
  missing APK provenance. No stale APK or unchanged blocked Gradle retry.

```text
main.jar ad960a2766149f3f8d4d7c4bacf107d18dccc80d0c5c27da685000897e810a14
unit.jar 749a7ec74e5d1b63e2041efc03484282bd8dd92a271987b0bb2f1e8dfd479904
instrumented.jar 83e46028920eee12c6d35fbcaae8d675c52147af0ae2caa7b1b96c24dfbd329c
TerminalPageCodec.kt 3476998a38e38f8ef2a506afbd750add5a4900e5fab02c4a0c7b16ce2a5fe98f
PairingViewModel.kt f727dc07c60dc0cbed0c82f5c7fa3975bdbfa21da92681e29e86d7c14047b4b3
MainActivity.kt a879db8862c4c9870107dce8f5f5706cc6baa41cbd7f0d692e0811089d169865
TerminalPageTest.kt 435e8e8ef06bfe26ec4edc7c8f0b5b366944374205a9f20c5fd766cfcfd285af
TerminalHistoryScreenTest.kt 5653a199bca3f1461d6de68c9322a50012447efb14e3ca2e65e27ea58e8748b2
debug build receipt 152ef91fd8f8046fbad2deb1451f4c0c5bfa6af6ef145ea14a49ca4823ee2871
debug binary 0146a6732f7d5ffb1be4ab532e3724dc6dcf75b90caadcc1234384fe9a6fa788
```

Goal stays active. Current-source APK, physical camera QR/direct LAN, production
desktop/Android TLS sync, API 31/34/35 connected lifecycle, full parity and signed
install/upgrade still require current behavioral evidence. No new device/network
acceptance or complete terminal-emulator fidelity is claimed.
