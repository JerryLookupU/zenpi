# Cached Tool Summary Restore

Provisional S2-031/S2-034/S2-037/S2-051 progress on 2026-10-04. Previous turn
repaired transcript continuation admission and passed source tests, so it was
progress. Current cache, HostClient resume selection and lifecycle/transport
tests were inspected. No nested agents, worker admission or checkbox promotion.

## Repair

Offline cache intentionally omits tool summaries/arguments. An unchanged saved
event cursor previously selected incremental replay after cold start; no new
events meant old assistant messages permanently lacked their tool controls.

New cache writes record only tool_calls_omitted: true on assistant turns whose
summaries are withheld. Decode strictly rejects false/non-Boolean/non-assistant
markers and still rejects raw tool fields. RemoteTurn carries the local omission
state, and HostClient selects a fresh authenticated snapshot when it is present.
The snapshot restores the current bounded tool summaries; following reads may
replay normally. Plain cached transcripts retain their original cursor replay.
Cache encoding remains stable across restore/display-only roundtrips and no
tool ID/name/preview or mutation authority is newly persisted.

The first candidate used missing workspace authority to select snapshots. Review
of the current network/process tests showed this would unnecessarily replace
all ordinary cached replay. It was refined before delivery to the explicit
omission marker. Older unmarked v3 records remain readable but cannot identify
historical omissions; their missing summaries need an explicit fresh snapshot.
No unsupported claim of retroactive cache repair is made.

Two JVM regressions cover omission/cache roundtrip, snapshot restoration at the
same cursor, live/plain replay selection, scope/cursor fences and malformed
markers. The policy is exercised by the production HostClient branch; these
tests do not establish connected SSLSocket/Keystore/Activity behavior.

## Verification

- Final source check /private/tmp/stage2-transcript-restore-marker-final-20261004:
  exit 0, main/unit/instrumented sources compile and 394 JVM tests pass.
  Existing deprecation warnings and one local variable-shadowing warning remain.
  Earlier broad candidate source check passed but is not the final artifact.
- Stage2 Python discovery: 110 passed; lifecycle verifier: 21 passed;
  Blueprint verifier tests: 5 passed.
- Strict Blueprint, git diff --check and rtk cargo fmt --all -- --check pass.
  All 36 authoritative rows remain [ ]. Rust production code/binary are unchanged;
  debug host verify-only passes.
- APK verify-only: exit 1, missing current-source provenance. No stale APK
  install, unchanged blocked Gradle/ADB/TCP retry, physical scan or LAN run.

```text
HostClient.kt 41cf4cc0b38a395a795568898f703b1a162d8812ff14d1cee409c6e17878fc02
ProjectionCacheCodec.kt 3e926cc337bf5b64a6003e052be0dc8624128b0a51e92214175ef3dff9c86475
ToolCallProjectionTest.kt 0758e08ce6f1684277f028c842fffef52dfb8eca2aa201bf4efa333bf84acbd9
main.jar c0bc568ace5b912e1a11c49d4dda5ef68f74222c348e2fa7cdf3c8bec8759f16
unit.jar e5077ec4e9d9cb4b2b2ff0314bbccdbea501ea24998dad157ecd33cf637f0a7f
instrumented.jar 448dccae8273dfc0d0c556eb52289e0b82b2057cf85f4d5052a0285cfb155776
debug-host-receipt 5ccc40c62c23203f07e0b31dcda9167132c7cf12af0b28298ffeffa18f63114c
```

Goal remains active. Current-source APK, actual camera QR/direct physical LAN
bidirectional sync, full parity, connected API lifecycle and signed release
remain incomplete or unverified. This repair is not whole-item acceptance.
