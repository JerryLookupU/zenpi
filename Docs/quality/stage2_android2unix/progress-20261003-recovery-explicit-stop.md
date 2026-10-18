# Explicit Stop During Pairing Recovery

Provisional S2-031/S2-037/S2-038 progress on 2026-10-03. Previous turn changed
credential admission freshness and produced JVM evidence, so it was progress.
Current worktree/specification were inspected again. No nested agents,
checkbox promotion, commit/push or unrelated cleanup.

## Repair

During background/network pairing recovery, metadataLoaded is false while a
storage read waits for an owned commit. cancel previously only removed a queued
link in this state; the late read could still automatically reconnect after
foreground/LAN restoration. It could also republish an interrupted invitation
after the user had explicitly cancelled it.

A recovery-local cancellation flag now suppresses automatic refresh without
discarding the storage read. A committed record restores offline credentials;
an absent record returns empty. Starting a new reconciliation resets the flag,
and an explicitly supplied new link still follows the normal queued-link path.
The flag is accessed on the main state owner, not from the IO storage callback.

The existing background/network PairingCancelTest methods now test explicit
cancel during both absent and completed saves, restore foreground/LAN before
releasing storage, and verify no late auto-connect or old invite resurrection.
These cases run before their existing named phase, so the suite remains 80/81
methods without/with Doze. These are held-storage regression fixtures, not
network outcome evidence.

## Verification

- A fresh adb devices -l attempt failed at server listener startup with
  Operation not permitted, before device enumeration. Its command exited;
  no connected phone/emulator or live ADB handle is inferred.
- APK verify-only still refuses missing current build provenance. No stale
  install or unchanged blocked Gradle retry occurred.
- Stage2 Python discovery: 107 passed; lifecycle verifier: 21 passed.
- Physical verifier six and instrumentation/Blueprint tests five each pass.
- Kotlin source gate /private/tmp/stage2-recovery-explicit-stop-20261003 exits
  zero: main/unit/instrumented sources compile and 383 JVM tests pass. Expanded
  Android recovery cases compiled only, not device-executed.
- Rust debug/release attribution, formatting, whitespace and strict Blueprint
  pass; all 36 authoritative checklist rows remain unfinished.

```text
PairingViewModel.kt 6c79a895e6cc8fa4a0cbbcc5d8edae02b4bcdc35dd058315defcb9b528a8596d
PairingCancelTest.kt 6a861e1f8f05f26612a57d13f025017cd3aa2c74870861c44fc42b0ce4f51e5a
main.jar 40f7588ac61436f96bc29240b607ead3b1b9a21bc218bf40e7854ea921cfc106
unit.jar ea58fb6025947826eb5bf8e1f650ec3a0adf4e9194dfb5fe1b67805ee35e8541
instrumented.jar de9d431d793e95eca7d393d0a145acb521a0769ac72b814ca69ce11fb0ca981a
```

Goal remains active. Current-source APK, real physical QR/direct LAN desktop
sync, complete parity, supported API lifecycle and signed release remain
unaccepted. Device availability was asked previously; no reply is assumed.
