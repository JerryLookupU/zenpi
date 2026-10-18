# Android Terminal Viewport And Editing Keys

Provisional S2-036/S2-054 progress on 2026-10-03. Previous goal turn repaired
scanner acceptance identity checks. This turn improves actual Android terminal
display and key access, without weakening existing instance/ACL/admission
boundaries. No nested agents, worker launch, checklist promotion, commit/push
or unrelated cleanup.

## Product Change

The connected terminal uses a bounded-width horizontal viewport with monospace
text and no soft wrapping. Host newline layout remains intact on narrow phones,
and long lines scroll horizontally instead of becoming ambiguous reflowed
screen rows. Same-instance output refresh retains horizontal position;
host/session/subtab revision/instance replacement creates a fresh scroll state.
Project/session and the subtab projection's project must match before rendering.
No raw ANSI stream, local shell or full terminal emulator is introduced.

The keyboard-icon dropdown adds fixed Backspace/Delete/Home/End/Page Up/Page
Down/Insert/EOF/Suspend/line-start/line-end/redraw bytes matching existing desktop
terminal mappings. Every key uses terminal_input and the existing durable
instance-bound request path. No new wire operation, capability or silent retry.
EOF/Suspend affect the current host program or shell. Receipt acceptance remains
byte admission, not proof of command execution. Pending revoke/reset and disabled
controls close the menu; offline/lost Control removes the menu. Input drafts and
geometry handling retain their existing scope fences.

## Verification

- New TerminalKeyTest checks every exact byte sequence, distinct labels,
  persistable envelope, foreground/network/offline dispatch refusal and terminal
  instance replacement fencing.
- Existing TerminalControlScreenTest expanded at 320 dp: checks two host lines
  are two layout lines despite a long first line, horizontal gesture/state,
  same-instance refresh retention and revision reset, every menu callback,
  revoke/reset closure and foreign terminal hiding. It remains in the exact
  desktop suite without changing its method count (69/70 with Doze).
- Final Kotlin source check at /private/tmp/stage2-terminal-ui-final2-20261003
  passed main/unit/instrumentation compile and 365 JVM tests after all additional
  Compose retention/reset assertions; its session was observed terminal exit 0. Compose
  assertions are compiled only, not device executed or screenshot acceptance.
- Stage2 Python discovery: 100 passed; lifecycle verifier: 18 passed; shared
  instrumentation/Blueprint tests: five each passed. Strict Blueprint and
  git diff --check pass; all 36 rows unfinished.
- Existing debug/release host verify-only passes; Rust source did not change.
  APK verify-only refuses missing current build provenance. No old APK install,
  unchanged Gradle/socket retry, actual camera scan or physical LAN run occurred.

```text
MainActivity.kt a6d172cbafa06d4f6f78f0b26fc8a244ee44a8f803acb5bd3c7c50e69841f7a2
TerminalKey.kt 92ab14de593eee6efeafe301ad1c576ef8c9b00e05fd583fb0a1dd1c2b8d95ef
TerminalKeyTest.kt 08d0b52b4b7318e3f1950686b1a4900954f6711b112d4cf922828ded3fb78fdd
TerminalControlScreenTest.kt 03ec23eb667d79baaa355de1e0fa49f6b1fbfff7d97d821610736698ee596bdf
main.jar 27ecbac2d4b284f5d870135125e2c643d9e05b6f9e3bef99ecd2589616e4cb13
unit.jar c26951522f2a7cf9ceae3da94d1892bf23c9f46c33b1ede02feb2e533f6b3c3b
instrumented.jar 04431a670b0dafa68f32799e2e7c125a6e24fb5f4ee2e7035b0ef37add76ac33
```

Goal remains active. Current-source APK, connected production desktop/Android
TLS synchronization, physical QR/direct LAN, API lifecycle matrix, full parity
and signed installation/upgrade still require behavioral acceptance.
