# Foreground Workspace Write Admission

Provisional S2-036/S2-038 progress. Previous goal turn improved LAN-first QR
address selection. Current source inspection found cancellation could start
a network write from OFFLINE/background state when the retained projection
still held a live prompt and networkAvailable remained true. The UI Stop
button also used phase != CONNECTING instead of live/error phase. Full
Android QR/LAN/Rust desktop goal stays active; all 36 Blueprint rows `[ ]`.

## Changes

WorkspaceDispatchPolicy is used at the shared executeWorkspace entry before
state changes, pending intent persistence or connection replacement. It
requires foreground, available network, no reset/revocation and a valid phase.
With a pending command, only the identical complete envelope may explicitly
retry. Without pending intent, CONNECTED permits existing typed commands;
ERROR permits only steer/cancel/approve. OFFLINE cannot admit a fresh write.
Existing command constructors/host ACL/CAS validation are unchanged.

Public refresh(command) also refuses background/reset and nonmatching saved
pending envelopes, so it cannot bypass durable admission. Pairing confirm,
new revoke and revoke-status callbacks require foreground. Local reset still
works independently. Stop button is disabled offline with retained live
authority. Four JVM tests cover all phases, network/foreground fences, exact
retry identity, error controls and reset/revoke. Instrumented running-provider
scenario invokes background callbacks and requires unchanged state and
encrypted bytes before release/reattach. Existing revoke fixtures now mark
foreground explicitly and check initial background revoke is a no-op.

The headless provider gate now requires a separate background-write-fence
phase. Lifecycle matrix validator requires both new behavioral flags, exact
ordered stream evidence and mandatory stream screenshot hash/signature.
Two new validator tests reject boolean-only/old/missing/out-of-order stream
evidence and missing/tampered screenshot. Historical receipts unchanged.

## Verification

Cwd /Users/mac/Github/zenpi; observed 2026-10-02T01:10:05Z UTC.

- `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-foreground-write-20261002`: main/unit/instrumented
  Kotlin source compilation exits 0, **117 JVM tests across 25 classes pass**.
  Instrumented scenarios source-compiled only, not executed on Android.
  Existing icon/safe-call/fixture warnings remain.
- `env ZENPI_HOME=/private/tmp/zenpi-foreground-write-20261002 cargo test
  --offline --locked --test remote_headless --test remote_subscription
  --test remote_subscription_tls`: **21 pass**, exit 0 (1/10/10).
  Rust source unchanged this turn; TLS uses bounded encrypted memory streams,
  not TCP/LAN. Existing Rust/vendor warnings remain.
- Python smoke tests 34, Blueprint tests 5, lifecycle validator tests 11:
  **50 pass**. Strict Blueprint, cargo fmt and git diff checks pass.
- `./gradlew :app:assembleDebug :app:assembleDebugAndroidTest --offline
  --no-daemon`, cwd android: exit 1 before compilation, Operation not
  permitted opening wrapper ZIP lock under /Users/mac/.gradle. No new APK,
  device receipt, agents, permission bypass, commit/push or checkbox updates.

SHA-256:

```text
WorkspaceDispatchPolicy.kt a06438cce08a0f68243b4151e6f017d5fa984af0132868d96882f6441ad71085
PairingViewModel.kt 33d001edb597b78ea789c862bba96d3ef6c3d2f2cc66b676248b844809ba1ed4
WorkspaceDispatchPolicyTest.kt 035846cc50f3cd7e03c5d56635c1a90d358c94a98d10451cda8794311c037a2e
HeadlessHostTest.kt b3deb7180ff928f335ad361af78dde0d4fc8053c9d5d78a672ad968e27adbab4
stage2_headless_android_smoke.py 8dbde13c2770d6513fcf1871fc3121a3a04231393a2747597b1ab363f29e88fe
validate_stage2_android_lifecycle.py 397a8b5b075549396253f064f118d2a5ec3a5f8e5f06570b7ba984fa1ffb12c8
```

This fences new background writes; it cannot roll back a host side effect
already admitted before lifecycle cancellation. Such unknown outcomes retain
their original pending identity. Fresh supported-API APK/Keystore/lifecycle,
physical camera QR/same-WiFi LAN, independent-worktree 1..64 scheduling,
interactive PTY/bulk, full stream delta/backpressure and release/migration
remain required. The goal is not completed or at a no-progress impasse.
