# Physical Failure Observation

Provisional S2-051/S2-054 progress on 2026-10-04. The previous turn changed
cache omission recovery and verified source gates, so it was progress. This
turn inspected current physical/manual-camera orchestration, shared process
runner and invitation/listener deadline sources. No nested agents, worker
admission, checkbox promotion or unrelated cleanup.

## Repair

Shared run_instrumentation accumulated phase output but threw on timeout or
callback failure without returning it. The physical harness only assigned its
output after a normal return, so failures during the operator-assisted path
could publish an empty instrumentation artifact and lose the observed stages.

InstrumentationFailure now carries a CompletedProcess observation after owned
adb process cleanup and reader joins. Timeout and tick/callback failures remain
failures; output is not interpreted as success. Error messages are generic and
do not echo arbitrary callback text, which could contain invitation data. The
physical harness catches this typed result, retains stdout/exit code/phases,
then proceeds through its existing failed-receipt and fixture cleanup paths.
It still redacts decoded invite/token from the saved output. Remote Android
instrumentation is not assumed stopped merely because adb was reaped; existing
force-stop cleanup remains required. No listener or invitation TTL is extended.

Four tests launch real short Python subprocesses without adb/TCP: timeout with
partial phase output, callback failure with private exception text, normal
stdout/stderr completion and tick failure. They require the owned process to be
terminal and verify errors do not expose private callback diagnostics. This is
process-runner evidence, not physical camera/network execution.

## Verification

- Pair harness tests: 9 passed; physical saved-evidence tests: 9 passed.
  Final Stage2 Python discovery: 114 passed. Earlier discovery before the tick
  regression reported 113; it is not the final count.
- Lifecycle verifier: 21 passed; Blueprint verifier tests: 5 passed.
- Strict Blueprint and git diff --check pass. All 36 rows remain [ ].
  Rust and Kotlin production sources are unchanged this turn; no Kotlin build
  or device execution is claimed. Current debug host verify-only passes.
- APK verify-only: exit 1, missing current-source provenance. No unchanged
  blocked Gradle/ADB/TCP retry or stale APK installation was attempted.

```text
stage2_android_pair_smoke.py 97ef0b6d956138b2507913f3dceeba17a37718208e4bf2f9052247488fe75172
test_stage2_android_pair_smoke.py 5db5ec6f9d08d9d6bf2b68d856601d21fa2b9b505ac1d12e0f7248d9f83ea53c
stage2_physical_qr_android_smoke.py 8eecbe3f92196ff3218fdc950210c77ba24d352b2c1d3b085d1a22b1bb44b133
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current-source APK, actual QR/direct physical LAN
bidirectional sync, full parity/API lifecycle and signed release acceptance
remain incomplete or unverified. Prepared failure observation does not accept
the product workflow or an entire Blueprint item.
