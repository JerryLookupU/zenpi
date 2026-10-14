# History Streaming Budget And Return Navigation

Provisional S2-036/S2-054 progress on 2026-10-04. Previous history-browser turn
changed source and verified behavior, so it was progress. Current execution
skill/Blueprint policy, dirty worktree, host history serializer, Android page
codec, read scope, dismissal and device verifier were inspected. No workers or
nested agents admitted. No authoritative checkbox promotion.

## Implementation

Current-session history previously cloned selected messages and buffered their
complete serialized JSON for every page. It also sized metadata using another
temporary string and counted unescaped content rather than the digest's actual
bytes. Host now borrows selected durable turns, resolves tree ancestry in order
and streams JSON directly into a SHA-256 writer. The writer refuses more than
64 MiB serialized history or expired deadline, including escaping and metadata.
It does not allocate a complete conversation clone or serialization buffer.
The digest is byte-for-byte compatible with SHA-256 of the prior JSON array.
Worker history uses the same streaming serializer without the new current-
session deadline. Noninitial empty terminal pages are refused; empty initial
sessions remain supported.

Android now validates retained history page shape, u64 journal revision,
counts/ranges, digest, message bounds, distinct IDs and adjacent same-scope
continuity before navigation, forward requests or historical message read scope.
Malformed retained pages cannot become read authority just because they share a
project/session. Closing an expanded historical message explicitly retains its
exact valid page, including after a transient message error, while changed
scope/journal/tool identity clears stale history. The prepared Compose case now
exercises the actual Close message button and returns to the history list.
The device harness requires an updated exact UI marker, so earlier UI evidence
cannot stand in for this behavior. This is prepared code, not device execution.

## Verification

- Final Rust transcript unit selection: 7 pass. Streaming regression compares
  exact JSON digest, admits exact writer capacity, refuses one byte less,
  expired deadline and escaped content exceeding the actual byte budget.
- Full owned-home serialized remote_workspace: 71 pass:
  env ZENPI_HOME=/private/tmp/stage2-history-bounds-home-20261004 rtk cargo test
  --offline --locked --test remote_workspace -- --test-threads=1.
- Final Android source command: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-history-bounds-final-20261004; exit 0, 430 JVM tests pass.
  Production/unit/instrumented compile; instrumented code not device-executed.
  Existing deprecation warnings remain.
- Python Stage 2 discovery: 125 pass; session harness: 8 pass;
  lifecycle verifier: 21 pass; Blueprint verifier: 5 pass.
- Strict Blueprint, Rust formatting and git diff --check pass. All 36 rows [ ].
- Real offline locked debug host build and current verify-only attribution pass.
- APK verify-only refuses missing provenance. No unchanged blocked Gradle retry,
  historical APK install, ADB camera/device or production LAN run occurred.

```text
transcript.rs 59046fac969f76713b4978b7661f1be390a190ffdab942e0a6f88d7b8055e777
SessionHistoryCodec.kt d7ba38f3845a738ead6d84d5063a66518b5cc8f197c406f3cb88b82364d95f37
SessionHistoryReadPolicy.kt 11c5a677858908b7900d4d8722e68ff8c2eb22312ba90337dfe0578e7d462902
PairingViewModel.kt a71bfd54569c861098d88fb6646eecdf3acd0a8e9ff9ced6877021e69915f940
SessionHistoryTest.kt 7ad71a6139e8795b0ee23a3b491d78c70e818748207aec33832a572e67cddf0e
SessionCatalogScreenTest.kt f349ddb90a80adc1c956dd045435c5179cf702f41368698b0f02162ff7ee31fd
stage2_session_android_smoke.py 257d707c38747597639c366029bf9f8e8759f68978349eee9fa018de6e3a92d8
main.jar aabad7ef0f3a3b1e6ed3a9c76e7d6865e66c6ccc1976049100e1110edf87f16e
unit.jar c2134c072271336b67dd65a8562205d1adc4830bd1666815a4765d752c9cd334
instrumented.jar 3ac8dca68d65537c09b5f32e174bd77b612c3fd2b29568f0395567c650aebf31
debug-host-receipt 032a79986023c31f77daf0add6629b476e86c7ac5e1a3379ee06b55b80d75db4
debug-host-inputs f89188d3442a910d1466f98e35214390e7bf3d3658a5f8d5e8a55709bbabd159
debug-host-binary bff1b3caadd45004f7d11336eedab8314c733ff576287da3091de7f8bffad166
```

Goal remains active. Current-source APK provenance, physical QR/direct LAN
bidirectional sync/recovery, connected API lifecycle matrix, complete desktop
parity and signed release remain incomplete or unverified. Source compilation
and fixture suite counts do not establish these gates or entire-item acceptance.
