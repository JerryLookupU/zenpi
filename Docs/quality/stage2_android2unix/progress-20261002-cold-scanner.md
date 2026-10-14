# Cold Foreground Pairing Admission

Provisional S2-031/S2-038 work, observed 2026-10-02T01:44:25Z. Previous
goal turn changed host shutdown result retention and completed its regression
gates: classified progress. Full Android/native QR/LAN/Rust desktop objective
remains active; no authoritative checkbox is promoted.

## Production Defect And Repair

PairingViewModel loaded a saved host, then called refresh when already in
foreground. refresh increments generation. The following generation == 0
block therefore skipped metadataLoaded and queuedInvite consumption. Normal
cold Activity onStart before IO restoration could permanently prevent
beginScan, and an invitation arriving before that read could remain ignored.

Restore saved state, mark metadata loaded and consume the queued invitation
inside one current-generation block before optional automatic reconnect.
Invitations still require manual confirmation; saved pending workspace/revoke
continues to prevent re-pairing. Cancellation while restore is pending clears
the queued invitation without discarding saved host metadata. No TLS, ACL,
storage format, private-key or token persistence relaxation.

Four additional ScannerResultTest instrumented cases reproduce foreground
before IO continuation deterministically on the main thread: scanner readiness
after saved-host reconnect, queued deep-link confirmation before reconnect,
pending workspace fencing, and cancellation of the queued invitation. These
use real PairedHostStore/Keystore and production ViewModel; they are compiled
but NOT executed on a device in this turn.

Scanner harness state/UI suite count moves from 13 to 17, and both live
instrumentation and saved-receipt validation require the four corresponding
phase markers. Older receipts cannot prove the new checks. Two Python tests
reject missing individual phases and the old suite count. Android development
guide records the production ordering and device-verification limitation.

## Verification

Cwd /Users/mac/Github/zenpi, except Gradle cwd android/.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-cold-scanner-final-20261002: exit 0. Cached Kotlin 1.9.24,
  Compose compiler 1.5.14 and SDK35 compile all main/unit/instrumented sources.
  **136 JVM unit tests pass** in 26 classes. The four new device cases are
  instrumented-source compilation only, not part of that JVM pass count.
- ./gradlew :app:assembleDebug :app:assembleDebugAndroidTest --offline
  --no-daemon: exit 1, wrapper initialization FileNotFoundException on
  /Users/mac/.gradle/wrapper/dists/gradle-8.11.1-bin/bpt9gzteqjrbo1mjrsomdt32c/gradle-8.11.1-bin.zip.lck,
  Operation not permitted. Failed before Android compilation/DEX/packaging.
  No fresh APK, permissions bypass or replacement of historical artifacts.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint: **52 pass**, exit 0.
- python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0,
  36 incomplete rows. git diff --check passes.
- Historical scanner-api35-resultfenced-20261002 receipt tested against current
  validator: refused (already missing required nonblank permission phase).
  No historical scanner receipt promoted to new cold-restore coverage.

SHA-256 source:

```text
PairingViewModel.kt 26fcdd8ef7da26a1a7ab30005e0696861edcd570e87c0e125e5e83a9ada5be68
ScannerResultTest.kt be553107ba9c6653505d796c39adc432f2cbe7a9b00274448c91b31f8824e2f9
tools/stage2_scanner_android_smoke.py 132fe351184af05a2f9c5b0dd3bd36154825c225c7bf278fe73252816783118b
tools/test_stage2_scanner_android_smoke.py 247e3aaa244f040af5232992db6a7dea4252e889158f8e1e817cd5c4ca07c858
```

Compiled evidence under /private/tmp/zenpi-cold-scanner-final-20261002:

```text
main.jar bfb9bd9b7fc8c49970635b906d359dcf8233d01cf988427ca9954e9b336193f5
unit.jar 40e9b23048b9b318cd44c9290c2e317e15b88aa5c7564b5df7c8898fb0ccc5b7
instrumented.jar 8be888227b2d5e3ae32b0da82d53e4c4e3289b8b48ba0e80f35e42caf1b964c0
```

Physical camera QR, actual same-LAN phone sync, fresh API31/34/35 lifecycle,
effective per-worktree quotas, full display/control parity, stream delta replay,
interactive PTY and release/migration acceptance remain unfinished. No agents,
commit/push or unrelated user-change rollback; goal remains active.
