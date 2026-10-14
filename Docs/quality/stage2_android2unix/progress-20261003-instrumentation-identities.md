# Instrumentation Test Identities

Provisional S2-053/S2-054/S2-055 acceptance-preparation progress at 2026-10-03.
Previous goal turn was verified suite-coverage progress. Current output audit
found the full count plus declared class list still could not establish which
individual tests actually completed. No nested agents, checklist promotion,
device launch, commit/push or cleanup.

New stage2_instrumentation.py discovers the named current-source JUnit methods
for the explicit class suite and requires exactly one start/success pair per
method from AndroidJUnitRunner status blocks. It checks class/method identity,
current/numtests indexes, missing/duplicate fields, failures/skips and unfinished
tests. A sole matching OK summary is insufficient. Input is bounded to 4 MiB;
unsupported source annotation styles fail until discovery is updated.

Desktop and session harnesses call this validator before accepting their initial
suite. Desktop receipts save sorted completed identities; the API lifecycle
validator reparses the saved output and compares identities. Both harnesses bind
the verifier source hash across the run. Existing provider, journal, process,
phase and APK/host provenance gates remain. This is an evidence-consistency
check, not authentication of fabricated logs or proof of device execution.

## Results

- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 93 pass.
  Expected invalid-LAN argparse negative fixtures emit diagnostics.
- python3 tools/test_stage2_instrumentation.py: exit 0, 5 pass. Positive fixture
  uses named pairs and interleaved phase statuses; negatives cover skipped,
  failed, foreign, repeated, missing, unpaired, wrong index/count and oversize
  output. One historical scanner invocation checks actual runner wire format,
  not current APK or scanner acceptance; the historical file was not modified.
- Lifecycle validator: exit 0, 17 pass, including summary/checksum-only and
  completed-identity mismatch refusal. Blueprint validator unit tests were
  unchanged; strict validator and git diff --check pass.
- cargo fmt --all -- --check passes; Rust release --verify-only passes for
  unchanged sources. Android build --verify-only still refuses missing APK
  provenance. No stale APK or blocked Gradle/socket retry.
- Product Kotlin is unchanged; the latest JVM source result remains 349 tests,
  not a new device execution. All 36 authoritative checklist rows unfinished.

```text
stage2_instrumentation.py e2bb3ffa6b6a310ce73e440b040c4633150bc4b9b6c48e0f58c90e4d3ffdc082
test_stage2_instrumentation.py b33c87b64cc886343d38280f7d7b09798fd41c9903220f6e5d5fe269a89e0e08
stage2_desktop_android_smoke.py 3af025adc9422b2a096a316c327cdf0199b7acaa0650d17950e663458cff0b00
stage2_session_android_smoke.py 96e7e83d3d6725460258b2e86674f50a74ab6321ff94e5ad8a5d4ce4ff14aa3c
validate_stage2_android_lifecycle.py 26ba445274499d8989e88a9d15201ede0b69dd6aff8dd66307da767624fc7eec
```

Goal remains active. Current-source APK/device suite execution, production
desktop-to-Android TLS sync, physical QR/direct LAN, API 31/34/35 connected
lifecycle and signed install/upgrade remain required behavioral gates.
