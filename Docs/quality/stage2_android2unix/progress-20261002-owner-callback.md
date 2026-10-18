# Android Main-Owner Callback Publication

Provisional S2-033/S2-036/S2-038/S2-054 progress observed
2026-10-02T06:29:14Z. Previous goal turn made
progress (typed authority-loss fence and stale error-phase controls).
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty changes preserved.
All 36 authoritative rows remain [ ]; full goal active. No worker admission,
subagents, commit, push, cleanup or acceptance.

## Implementation

HostClient executes socket work inside Dispatchers.IO. Its subscription
projection, command confirmation and prompt-listing callbacks previously
updated PairingViewModel state directly on that IO caller. MutableStateFlow
alone does not make a read/modify/write transition plus page reconciliation
atomic relative to main-thread network/lifecycle/user actions.

These ViewModel callbacks now use ownerCallback to dispatch to Main.immediate
and check the current connection generation after owner dispatch. Encrypted
storage remains on IO under the existing mutex. Subscription and command
confirmation recheck generation after suspended persistence before changing
projection, resolving the UI pending request, reconciling transcript pages or
closing transient views. The final ordinary attach also rechecks after storage.
Prompt listing callback is now suspend so the IO reader waits for its owner
transition rather than scheduling an untracked fire-and-forget update.

Storage failure or callback cancellation propagates to the owned reader and
stops that exchange. Persist-before-publish, exact saved-request retry, scope/
revision checks and generation fences remain intact. This does not move socket
or filesystem IO onto the UI thread and does not create another connection.

Five new deterministic JVM tests queue the owner runnable while the caller is
on IO, then interleave local draft edits, background/new QR/selection/command/
authority generation changes, cancellation, persistence failure and a suspended
save with generation replacement. They prove old work cannot publish/close a
new view and storage failure cannot publish a confirmation. A device test uses
the real Android Main Looper and rejects a replaced generation; it is added to
the desktop smoke suite (35 control / 17 read-only tests, plus optional Doze).
That device test is compiled only until device execution is available.

## Verification

Cwd /Users/mac/Github/zenpi. Final commands exit 0:

- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-owner-callback-android-final2: **200 JVM tests**, main/unit/
  instrumented source compilation. Existing Android deprecation warnings remain.
  No Gradle build, APK or device execution is claimed.
- env ZENPI_HOME=/private/tmp/zenpi-owner-callback rtk cargo test --offline
  --locked --test remote_subscription_tls --test remote_parity --test remote_workspace:
  **81 tests**. Rust production owner/ACL routing and rustls encrypted
  fragmented-stream regression remain green; not physical LAN proof.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  **63 tests**.
- cargo fmt --all -- --check; git diff --check; python3
  tools/validate_stage2_android2unix_blueprint.py --strict: exit 0.

Final cached-source artifact SHA256:

```text
main.jar c89c1199a0b9129a227cb6d7d11b84071a2c6e20605117c89c57df045cb86a00
unit.jar 7b0916d987db99102baa4e61534102ff1e380e30f16cbbdd6a98c1d0856a9899
instrumented.jar c3df43871171aeeef3f9a58157a55f490737c232fbc962d5c27b9fb808ddecdb
```

Source/requirement SHA256:

```text
Docs/stage2_android2unix_blueprint.md d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
Docs/stage2_android2unix_spec.md 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
OwnerCallback.kt c81169885da47e665be133baa2a99c015f303a814d31a464d7701bb44ac35c7d
PairingViewModel.kt a5d4fc56ce35853739c6b16add3fd16081825313d4093737fc8652e28c7cfc3f
HostClient.kt 933de4d5e751e5da8a94411fdd77a029c00e9fa4cc4709d070740db7f5feecbc
OwnerCallbackTest.kt ec761816ca83796f8122f16141b35c1c55d433e513b8c11a0c2dba2abb71cb69
OwnerCallbackDeviceTest.kt dd855f3ad7024ec72e6afe3bad0d3eaa8ebc4709038214b5c0cfa72fb7e114a4
tools/stage2_desktop_android_smoke.py 3c0554cf07f57f2b753e0e73918ee13834b92714d441ab60dcc4b75ba98b9cbb
```

## Residual Scope

Need real Android ViewModel/Keystore/TLS/Compose race execution with lifecycle,
scan and local host updates. JVM tests exercise the production owner helper
with deterministic dispatch, not a whole Android emulator. Current APK, camera
QR/LAN/API 31/34/35, concurrency scheduler, interactive remote PTY, full settings
and reproducible signed packaging remain unfinished. No restricted network/
Gradle/adb bypass. This note is provisional, not an acceptance receipt.
