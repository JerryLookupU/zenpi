# Device Suite Coverage

Provisional S2-036/S2-053/S2-054/S2-055 acceptance-preparation progress at
2026-10-03T20:43:00+0800. Previous goal turn was verified composer-scope progress.
Current harness inspection found recent file/terminal/task boundary tests were
not included in the explicit desktop class list, and session catalog UI was
not included in the session workflow list. No nested agents, checklist promotion,
device launch, commit/push or cleanup.

## Change

Desktop instrumentation now includes WorkspaceFilesScreenTest,
TerminalControlScreenTest, TerminalHistoryScreenTest, WorkerScreenTest and
WorkerStatusPageScreenTest. The suite contains 20 base and 38 control tests:
58 with control, 59 with Doze. One suite function supplies both the command's
class list and expected count. Desktop receipts record that exact class list
and count, and the API lifecycle validator requires their identity as well as
the complete output count/checksum. A historical 44-test output no longer
satisfies the current matrix even when its saved checksum matches.

Session workflow instrumentation now includes SessionCatalogScreenTest and
requires eight tests rather than seven. Existing phase, desktop callback,
journal/receipt and APK/host provenance gates remain unchanged.

New Python tests check the explicit class sets, uniqueness and test counts
against current Kotlin source methods. Lifecycle negative tests refuse reduced
counts, reversed/missing class sets and mistyped count metadata. These are
validator/harness tests, not execution of Compose or product behavior on Android.

## Results

Commands ran from /Users/mac/Github/zenpi.

- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 88 pass.
  Expected invalid-LAN argparse negative fixtures emit diagnostic errors.
- Desktop harness tests: exit 0, 15 pass; session harness tests: exit 0, 7 pass;
  lifecycle validator tests: exit 0, 16 pass; Blueprint validator tests:
  exit 0, 5 pass.
- Desktop/session --serial source-check --help: exit 0, no adb invocation.
  Plain --help exits 2 at the existing serialized-wrapper serial pre-parser;
  it was not treated as successful help or device evidence.
- Strict Blueprint validator, cargo fmt --all -- --check and git diff --check
  pass. All 36 authoritative checklist rows remain unfinished.
- Rust release --verify-only passes. Android build --verify-only still refuses
  missing APK build provenance. No stale APK or repeated blocked build attempt.
- Android product/test Kotlin is unchanged this turn; prior 349-test JVM source
  gate remains the latest source result, not a newly executed device gate.

```text
stage2_desktop_android_smoke.py 7782fff4548efe24ea234dc56207a8f9b1a33e2ee0ec6656f7c920a0812954cd
stage2_session_android_smoke.py bf438d1af2df94215eb88b5bf291274bfae2f3ef51b9ad8d667e80347aaefb17
validate_stage2_android_lifecycle.py 35196be9d8c781d0c2f695c23bfc5ca5f5ff406a0daaff1953549df6f96e5ae6
test_stage2_desktop_android_smoke.py 1a9803103fab5534eafa00d115f0aa10c9369b50dc9745cccde69ccc57ee8d40
test_stage2_session_android_smoke.py 497578120542bc9ee1891e97deeeab57566cdc23c0d28231d24f70ae4f37f248
test_validate_stage2_android_lifecycle.py 89d963747d77d0245870a63e601e5614352412e8a6b2afa1afdaa4e9d3b3a54a
```

Goal remains active. Full current-source APK/device suite execution, production
desktop/Android TLS sync, physical QR/direct LAN, API 31/34/35 connected lifecycle
and signed install/upgrade remain required acceptance surfaces.
