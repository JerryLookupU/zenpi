# Legacy Tool Summary Recovery

Provisional S2-033/S2-036/S2-038/S2-056 progress on 2026-10-04. The preceding
RTK check verified a local tool but did not advance the Android/Desktop goal.
Current cache, authenticated HostClient restore branch, background ResumeWorker,
encrypted-store migration and Blueprint were inspected before this repair.
No nested agents, worker admission or authoritative checkbox promotion.

## Repair

The previous omission-marker repair only helped new writes. Older v1/v2/v3
cache records lacked reliable information about withheld assistant tool
summaries. Incremental replay at an unchanged cursor could therefore keep old
invocation controls missing after an app upgrade or cold start.

New display writes use v4. Legacy v1/v2/v3 assistant turns conservatively acquire
the existing local toolCallsOmitted marker on decode; user/tool turns do not.
HostClient consequently chooses authenticated inspect/snapshot instead of tail
replay for these restored displays. Foreground and ResumeWorker share that
production branch. Normal v4 text-only caches retain incremental replay.
Earlier marked v3 records remain readable, while v1/v2 still reject that field.
Unknown versions, false/non-Boolean markers and cached authority remain refused.

Decode does not mutate input bytes, advance either cursor or add credentials,
arguments or mutation authority. A subsequent v4 cache write preserves the
recovery marker until a fresh host snapshot replaces it; migration alone does
not pretend to have recovered summaries. The encrypted migration test now
prepares v1/v2/v3 restoration, pending-request preservation and confirmed v4
commit, without write-on-read. It is compiled, not device-executed.

Three new JVM tests cover legacy unmarked assistant recovery and marker
roundtrips, earlier marked v3 compatibility/schema refusal, and user-only/empty
legacy replay. Existing cache migration assertions now describe v4 and the
conservative assistant marker. Protocol, Android and migration notes agree.

## Verification

- Final command: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-legacy-tool-restore-final-20261004; exit 0.
  Main/unit/instrumented source compilation succeeds; 397 JVM tests pass.
  Existing API deprecation and unnecessary-safe-call warnings remain.
  The earlier 396-test run predates the final user-only regression.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': 114 passed.
  Lifecycle verifier tests: 21 passed; Blueprint verifier tests: 5 passed.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 rows remain [ ]. Rust production/binary are unchanged, and the debug
  host build provenance verify-only check passes.
- APK verify-only still exits 1: missing current-source APK build provenance.
  No stale APK installation, unchanged blocked Gradle/ADB/socket retry,
  physical camera scan or direct LAN execution was attempted.

```text
ProjectionCacheCodec.kt 4fdd91ea6a0808daac258ee2d8618b6b2fc1d1b76d8632ec3da2a35ef54f2baa
ToolCallProjectionTest.kt f4f9e109c5047f2f7a98b83ec4c5abe96d0291ff55b1fd49c6bbbc4b84e4d9a0
ProjectionCacheTest.kt 76a1a6171c83003ceaed91204d1583d23d5e8e81e98d91d57c96e69ddeaf77f8
PairedHostMigrationTest.kt 1fb21779b5d52063241029fc027d51d0c080d57f2cee660fde88690d0160af28
main.jar 1662dd5e5bbf34204d8b06af1d39f5ce3c3124fa0dce9057ace37ce115574c79
unit.jar a796c7e4d35b383cc0ad5b22833f4b2be0af41b829104d35cfdd6f658dd2d571
instrumented.jar e81660547be5a7d39bd504a2e267e6f680f8729d1ab9a8100f2dbfcc98026fda
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current-source APK, real camera QR/direct physical LAN
bidirectional synchronization, full parity/API lifecycle and signed release
remain incomplete or unverified. These source tests do not accept an entire
Blueprint item or the final product workflow.
