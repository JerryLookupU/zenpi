# Historical Detail Journal Revision Fence

Provisional S2-036/S2-054 progress on 2026-10-04. Previous streaming/page-chain
turn changed source and verified behavior, so it was progress. Current execution
skill, Blueprint policy, dirty worktree, history read scope, detail codec, host
owner-lock dispatch and device harness were inspected. No workers/nested agents
admitted; no authoritative checkbox promotion.

## Implementation

History listings bound journal_revision, but opening a listed message reused an
ordinary ID/digest-only transcript read. A desktop journal change between listing
and opening could therefore deliver a detail outside the captured page revision
before subscription observed the change. Historical body/tool-argument reads
now carry the captured exact u64 journal_revision on initial and continuation
requests. Host checks it under the same owner lock before branch/message lookup
and echoes it on success. Stale revisions refuse revision_conflict even if the
immutable message still exists. Worker reads reject the field; existing live
transcript requests without it keep their prior wire semantics.

Android requires the exact numeric revision echo and stores it in transient
TranscriptPage identity. Page navigation cannot mix revision-bound and legacy
pages. Historical detail state checks it against its current validated history
page; missing/string/changed receipts cannot publish or navigate. This revision
is not cached or used as write authority. History UI now exposes detail for all
messages/tool calls, matching worker history rather than only truncated items.

Prepared production SessionHostTest opens the seeded historical message through
the real button, checks exact revision/text, unchanged live projection, closes
the detail and checks the same retained list. The verifier requires a new exact
host marker; previous read-only-list evidence is insufficient. No connected
execution is claimed.

## Verification

- Focused Rust current-conversation regression: 1 pass.
- Full owned-home serialized remote_workspace: 72 pass:
  env ZENPI_HOME=/private/tmp/stage2-history-detail-home-20261004 rtk cargo test
  --offline --locked --test remote_workspace -- --test-threads=1.
  Includes bounded detail redaction/read-only checks, malformed revisions,
  body/tool continuation success, stale refusal after unrelated journal event,
  legacy unchanged behavior and worker-field refusal.
- Android production/unit/instrumented source check: 431 JVM tests pass.
  zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-history-detail-current-20261004; exit 0.
  Final artifacts recorded below; instrumented sources compile
  only, not device-executed. Existing warnings remain.
- Python Stage 2 discovery: 125 pass; lifecycle verifier: 21 pass;
  Blueprint verifier: 5 pass. Strict Blueprint, Rust formatting and diff-check
  pass; all 36 authoritative rows remain [ ].
- Real offline locked debug host build and verify-only attribution pass.
- APK verify-only refuses missing provenance. No repeated blocked Gradle build,
  old APK install, ADB/camera/production LAN run or connected acceptance claimed.

```text
transcript.rs b937beabe04623a37058b295b1b91de0800fddba9c4b83e1b705c6bee3771e7e
TranscriptPageCodec.kt 716b6e2753b17c1fe5570813bad40304d8370bfc3ba8686d2b1461203aa471a8
PairingViewModel.kt 59c39b6558bd24c02f0f4beb7a9da7ec9b3e9e4ff2c4437c647350038e8edf5b
TranscriptReadPolicy.kt fe0ba0af28e45993d7cde9533e4764a4a2fa9f7e33db3cacf2182d84a55b550e
SessionHistoryTest.kt 1eeaf6e02926d12aee32b62048e921e89954d61276ea592c6b8b9b7d0964bcde
SessionHostTest.kt 9bcc9a53408ef4db6fc695a033977bdeb6294505e8e0b2020c1fc60b2adcdc62
remote_workspace.rs 3ad7bcd53846fdee6818ffe363d1a8aa31200194df1248ba5552c0a97ef303b3
stage2_session_android_smoke.py 5d8b51f7475b0db36df20a964a795d39b34cf799a4f2efc2efd9dc2d4ba7b3af
main.jar acd825f9d98cb3bbe19ec11f7d647fd34916d06e713b3c912ec72c226adf8c52
unit.jar 1fd211de30663715d22f5eccde3e82df66ddd7469fc6140c54867b9fb90eada1
instrumented.jar e78c1839d7879518040a0b4098a3cf5d68756c312c344b4fd7672739275f645d
debug-host-receipt 314e01f0c1df43e3c2197145ccbeb002d1892f8907f3fdd49c41989714b3c9a1
debug-host-inputs 4b124c828e6caedeb054e5a5b4612fbc418daa7bf8f08a83aa1d802f0c9f58a6
debug-host-binary d0ca082fc15718eac46410d5891b02241fc6d9e9512841ecac5fcfbb04f712d8
```

Goal remains active. Current-source APK, physical QR/direct LAN bidirectional
sync/recovery, connected API lifecycle matrix, complete desktop parity and signed
release remain incomplete or unverified. Source/fixture gates do not prove those
requirements or entire-item acceptance.
