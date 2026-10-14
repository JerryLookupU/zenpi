# Running Provider Android Acceptance Gate

Provisional S2-020/S2-032/S2-033/S2-036/S2-038 progress. Previous goal turn
fixed Busy ACK read-chain interruption; the intervening RTK environment check
confirmed local installation, not product completion. Current worktree HEAD
is `1a8ff1acf72e0816fad503a05df2b1d18e07361b` with existing user/generated
changes preserved. All 36 authoritative Blueprint rows remain `[ ]`.

## Changes

HeadlessHostTest now requires a running provider preview in the actual
PairingViewModel/Compose screen before the assistant turn exists. It checks
the encrypted store's matching transcript/cursors, stripped stream text and
unknown-outcome prompt status, captures headless-stream.png, backgrounds the
client and verifies the retained display/cache, then foregrounds it and checks
the same request/session/journal/event cursor while the provider is still held.
Only then does it emit a harness release phase. Completion must replace the
preview with the durable assistant answer and clear stream text. Existing
distinct-process exact retry and ACK verification remain required.

The production headless Android harness sends TextDelta/TextDone over its
real HTTP fixture and withholds response.completed until those ordered
Android phases arrive. The lock covers TextDone flush and its evidence event
to avoid a fast Android observer racing the provider record. Shutdown release
cannot manufacture success. Receipt adds separate stream gate event evidence,
explicit verification boolean and screenshot/hash. Four verifier tests reject
missing/out-of-order/duplicate events and shutdown-only release. The listener
and instrumentation deadlines were expanded to cover the new bounded phase.

This is executable acceptance scaffolding for the existing production stream
path, not a new device pass and not full delta/backpressure implementation.

## Verification

- At 2026-10-02T00:53:51Z UTC: `zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-running-stream-20261002`, cwd repository:
  main/unit/instrumented source compilation exits 0, **113 JVM tests pass**.
  Instrumented code compiled only, not executed. Existing icon/safe-call and
  fixture warnings remain.
- `env ZENPI_HOME=/private/tmp/zenpi-running-stream-20261002 cargo test
  --offline --locked --test remote_live_text --test remote_subscription
  --test remote_subscription_tls --test remote_headless`: **25 pass**
  (5/10/9/1), exit 0. TLS is bounded encrypted memory streams, not TCP/LAN.
- `python3 -m unittest discover -s tools -p 'test_stage2_*smoke.py'`: 34 pass;
  Blueprint tests 5 pass; lifecycle validator tests 9 pass; **48 total**.
  Strict Blueprint check exits 0 and preserves 36 incomplete rows.
  `cargo fmt --all -- --check` and `git diff --check` exit 0.
- `./gradlew :app:assembleDebug :app:assembleDebugAndroidTest --offline
  --no-daemon`, cwd android, exits 1 before compilation: wrapper lock
  `/Users/mac/.gradle/wrapper/dists/gradle-8.11.1-bin/bpt9gzteqjrbo1mjrsomdt32c/gradle-8.11.1-bin.zip.lck`
  is denied with Operation not permitted. No new APK or new connected receipt.
  No permission bypass, agents, commit/push or historical receipt edits.

SHA-256:

```text
HeadlessHostTest.kt 52d39b37dddaf6492ff2dffc82ee25f93bf9d54969a19d14320732f32afdde7e
stage2_headless_android_smoke.py ee86409af465a4776f82f5b1424d1fef465c47c724031638ef42b04acf733422
test_stage2_headless_android_smoke.py a7668c23d5dd6634c0dc1eaec13afec2b5a7bfb57af857aa2b3fc9f22d9d827e
main.jar fd8c22cb367a0f2edf43118b8f37032b295b1594d46c7cf463c3d5645e60988b
unit.jar 9880a5b180a0415fcf1cf501e9dc7fe5170b456926e86a0ace07a43117dbf27a
instrumented.jar 48ae80a553e642e833660a2a2b1b316daa3c09c7bab0697a0ddfcdab9de475ff
```

Full goal remains active: fresh supported-API APK/Keystore/lifecycle execution,
physical camera QR and same-WiFi LAN desktop/headless synchronization,
interactive PTY/bulk, executable independent-worktree quotas 1..64, complete
stream delta replay/backpressure and signed release/migration remain unproven.
