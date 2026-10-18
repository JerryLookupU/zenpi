# Physical QR Bidirectional Sync Gate

Provisional S2-031/S2-034/S2-037/S2-051/S2-054 progress on 2026-10-04.
The preceding turn repaired read outcome consistency, so it was progress.
This turn inspected current physical camera test/harness, provider text parser,
saved receipt validator and production Desktop terminal command helper.
No nested agents, worker admission, checklist promotion or unrelated cleanup.

## Workflow

The physical workflow previously proved an initial desktop transcript read and
prepared one Android prompt/desktop answer roundtrip. It did not require a new
locally submitted Desktop turn to reach an already connected phone without
manual refresh. That distinction matters for the requested communication/sync.

After the existing Read-only unchanged-journal gate, the fixture grants Control
and uses the production Desktop TUI command helper to submit a distinct local
prompt. The phone waits for the local provider answer without refreshing. It
must display it in the same session, advance its event cursor and find that
answer in the production encrypted store. It waits for idle before entering
the existing Android prompt/answer test through the real message controls.
No URI/token/SAS injection, process-wide route binding or adb reverse is added.

The local provider fixture now handles exactly the two distinct prompts. Receipt
v4 requires eight ordered phases, including desktop live update, and two ordered
provider requests/answers. The append-only journal must contain four distinct-ID
turns in local user/assistant then remote user/assistant order. Only the remote
prompt owns an admission binding and running/succeeded receipts. An old v3
single-roundtrip receipt, missing local turns/phase, reversed provider calls or
duplicate turn IDs cannot satisfy this gate. Existing camera/manual confirmation,
grant, cleanup, current build and transitive harness attribution checks remain.

These are executable acceptance requirements, not a claim that a physical run
occurred. The temporary provider tests the production runner path without
spending on an external provider. It does not prove arbitrary provider/NAT,
full desktop parity, disconnect/recovery matrix or release behavior.

## Verification

- Physical verifier: 9 tests pass. Stage2 Python discovery: 110 passed.
  Lifecycle verifier: 21 passed; Blueprint verifier tests: 5 passed.
- Source check /private/tmp/stage2-physical-qr-bidirectional-final-20261004:
  main/unit/instrumented compilation and 389 JVM tests. The physical class is
  compiled only; no connected execution. Existing deprecation warnings remain.
- Strict Blueprint, git diff --check, rtk cargo fmt --all -- --check and debug
  Rust source attribution pass. Rust production code is unchanged; all 36
  authoritative rows remain [ ].
- APK verify-only still exits 1 for missing current-source provenance. No
  stale APK installation, unchanged blocked Gradle/ADB/TCP retry, camera scan
  or direct physical LAN run occurred.

```text
stage2_physical_qr_android_smoke.py 5fe502bc8398fad0e4c9230ae3c6570062cfef68e202661eddeee8bea5feb8a5
test_stage2_physical_qr_android_smoke.py d4f6cc7f85c82f902d1647c52ff55b17a5a4030906852c88ecd5bd95289a49f6
PhysicalQrHostTest.kt 6b29526b3a08300237240e00a2486cf4e03edef4a0eea08f0bf16306145ccdeb
main.jar 43c3b51e71a7ec0b4f2cda155802b874bfe69fba0cbe58abc2284cf400ccaa82
unit.jar 4e9bb778b316e91f6a88101fc7edb64f563029b4651d4a827054327a06500feb
instrumented.jar 566b5a93e89392f810324d7595271826d36ab4a2b16ce045c499583653ab4334
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current-source APK and actual physical QR/direct LAN
bidirectional run, supported API lifecycle, complete parity and signed release
remain incomplete or unverified. No whole-item acceptance is published.
