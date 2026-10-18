# Physical QR Foreground Replay Preparation

Provisional S2-031/S2-038/S2-051/S2-053/S2-054 progress on 2026-10-04.
Previous turn repaired outstanding scanner result admission and verified source
gates, so it was progress. Current physical camera test, production onStart/
onStop, encrypted cache/cursor and shared host orchestration were inspected.
No nested agents, worker admission or authoritative checkbox promotion.

## Workflow Extension

The operator-assisted physical QR scenario previously covered a live Desktop
update followed by a phone prompt, but did not prove recovery after the phone
stopped its Activity while the Desktop continued. The prepared test now stops
the Activity at CREATED, cancels its scheduled WorkManager read opportunity to
isolate foreground recovery, verifies offline authority stripping and captures
its durable display/cursor. The host's background callback submits another real
Desktop TUI prompt through the existing local provider fixture.

A separate authenticated read confirms that the Desktop answer actually exists
while the Activity is stopped, without publishing it into the ViewModel/store.
The saved projection and stopped ViewModel cursor must remain unchanged and
must not contain that answer. Returning to RESUMED must automatically
reauthenticate/replay without Refresh, invitation/cursor injection or adb
reverse. The recovered answer is rendered once in the same session, with a
strictly advanced event cursor and equal encoded durable cache. Idle is required
before the subsequent production Android prompt is sent.

Receipt v5 requires all eleven ordered phases, three ordered provider calls and
answers, six unique ordered user/assistant turns, preserved original journal
prefix and the same one remote durable prompt admission/completion binding.
It also independently parses exact once-only background/recovered u64 cursor
observations and session identity from instrumentation output and compares them
to the saved receipt. Missing/duplicate/foreign/noncanonical/nonadvancing cursor
evidence fails closed. Older v4 receipts cannot establish this new scenario.
The 360-second observation allowance is bounded and does not extend invitation
expiry or turn the Desktop listener into a background Android keepalive.

This prepares a real direct-LAN/camera recovery gate, not a connected result.
WorkManager cancellation deliberately means this does not prove background
WorkManager execution, Doze, process death or every network-switch scenario.

## Verification

- Physical saved-evidence tests: 10 passed; Stage2 Python discovery: 115 passed.
  Lifecycle verifier tests: 21 passed; Blueprint verifier tests: 5 passed.
- Final command: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-physical-foreground-replay-verified-20261004; exit 0.
  Main/unit/instrumented sources compile and 402 JVM tests pass. Existing
  deprecation and unnecessary-safe-call warnings remain. Earlier runs predate
  the final idle fence and are not the final instrumented artifact.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 rows remain [ ]. Production Rust/Kotlin sources are unchanged this
  turn; Android instrumentation and Python orchestration changed.
  Debug host build provenance verify-only passes.
- APK verify-only exits 1: missing current-source APK build provenance.
  No stale APK install, unchanged blocked Gradle/ADB/TCP retry, camera scan
  or direct physical LAN execution was attempted.

```text
stage2_physical_qr_android_smoke.py f70c545607ee6b2ab1542bcde540d13922adaf41cd4482e3065c78f8ace07f7f
test_stage2_physical_qr_android_smoke.py a90d552de0d596c91dcf7026e37b204775f0c1367f6241f24c977a002075d2d0
PhysicalQrHostTest.kt 08c0a41a629ea960919d3b6506711850a0df53b2d4e8702309cceeebf675c4ac
main.jar 7418ef0f6564ef39673fbb3f2c5bd8643226a51c91d51cdf4e1c06685af5be1f
unit.jar 9d07f246c4ef29c39949033ef75a21c136269bef89ddfbca6dc6200d66e1112f
instrumented.jar 64c4cccd0ce0067cebb910f460419ab294bf1e3ed705fb0747504ebd79655dd6
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current-source APK, actual QR/direct physical LAN
bidirectional sync/recovery, full parity/API lifecycle and signed release
remain incomplete or unverified. No entire Blueprint item or final workflow
acceptance is inferred from prepared tests.
