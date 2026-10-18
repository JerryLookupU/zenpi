# Scanner Invocation Evidence Integrity

Provisional S2-031/S2-052/S2-055 progress on 2026-10-03. The preceding goal turn
bounded shared desktop PTY output. This turn inspected QR pairing cancellation,
one-use token handling and scanner acceptance, then repaired a count-only
scanner evidence gate. No nested agents, worker launch, checklist promotion,
commit/push or unrelated cleanup.

## Current Contract

PairingHandshake already checks invitation freshness before hello, after
welcome/challenge and after signing, sends one pair request and does not retry
the token. PairingClient uses the existing closeable socket lease and bounded
deadline. No pairing success or physical scan is inferred from this inspection.

The scanner harness formerly checked summary counts and phase substrings only.
It could not prove that the expected tests actually ran. It now uses the shared
instrumentation identity verifier for two separate invocations: one
ScannerActivityTest permission-denial case, then the 21 methods in
ScannerResultTest/PairingScreenTest/DiffScreenTest. Every expected class/method
must have indexed start/success pairs; skipped, duplicate, foreign, failed,
unfinished or summary-only output refuses acceptance.

stage2-scanner-evidence/v2 stores class order, integer count/exit status, each
invocation output/hash and exact completed identities. Saved output must equal
the two invocation outputs in order. Permission-denial and cold-restore phases
must each be exact unique instrumentation status lines. The receipt binds the
shared verifier source digest, checked during execution and saved validation.
File/output/receipt size bounds apply. Exact API guard, true narrow behavioral
gates, no failure and false physical-camera/LAN claims are mandatory.

Successful orchestration revalidates the saved receipt before reporting pass.
Cleanup failure now produces a failed receipt instead of escaping the finally
block without preserving results. These checks do not prove a real camera QR
scan or LAN communication, and v2 explicitly refuses those overclaims.

## Verification

- Scanner harness tests: 11 passed. Positive fixture has every current method
  identity; negative cases cover count-only output, skipped/foreign/duplicate
  methods, mismatched indexes/checksums/classes/completions, verifier changes,
  repeated phase, boolean counts/exit codes, scope overclaim and legacy receipt.
- Mocked main orchestration exercises both saved-evidence success and forced
  cleanup failure. Mocks are not Android execution evidence.
- Recomputed checksums around a foreign test still fail named-suite validation.
  Historical scanner-api35-20261002 and resultfenced receipts fail the new
  contract without being edited or promoted.
- Stage2 Python discovery: 100 passed; lifecycle verifier: 18 passed;
  shared instrumentation and Blueprint tests: five each passed. Strict Blueprint
  and git diff --check pass, all 36 rows unfinished.
- Existing debug/release Rust attribution verify-only passes. Rust and Kotlin
  product source were not changed this turn; previous 364-JVM Kotlin evidence
  is not claimed as a fresh device or APK result.
- Real scanner entry point refuses missing current APK provenance before any
  adb install/permission/listener operation. No stale APK, unchanged blocked
  Gradle/socket retry, camera scan or physical LAN run occurred.

```text
stage2_scanner_android_smoke.py fe080886cc18ddd4241987d528bb3036f6ff3fe48d808cc86899cce3b0dccc89
test_stage2_scanner_android_smoke.py 80a2c1ea0d44bff5630f22ca82d62acf7dc123b966a786fd0c77259d49dc104b
stage2_instrumentation.py e2bb3ffa6b6a310ce73e440b040c4633150bc4b9b6c48e0f58c90e4d3ffdc082
```

Goal remains active. Current-source APK, connected production desktop/Android
TLS sync, actual camera QR/direct LAN, required API lifecycle matrix, full parity
and signed installation/upgrade are still missing behavioral acceptance.
