# Shared Message Field And Channel Rules

Provisional S2-003/S2-030/S2-050 progress on 2026-10-04. Previous turn repaired
authentication reader authority fencing and passed source gates, so it was
progress. Current Blueprint, dirty worktree, reviewed contract, generator,
Rust message definitions and Android structural codec were inspected. No nested
agents, worker admission or authoritative checkbox promotion.

## Change

The shared contract now includes the thirteen message payload field sets,
channels and minimal structural fixtures. RemoteSchema.kt generates the maps,
and FrameCodec uses them instead of separate handwritten field/channel tables.
Rust retains independent typed definitions. A new contract-driven test in each
language decodes every fixture, rejects each missing required field, rejects an
extra payload field, and checks all three channel substitutions. Android also
asserts generated-map equality with the contract.

Generator validation checks safe message/field names, bounded nonempty unique
field lists, the type discriminator, exact fixture payload fields, valid
channels and no envelope overwrite of schema/channel/payload. Existing negative
generator tests now exercise malformed message rules as well. No production
wire acceptance/version change, credentials or network operation is added.
Public pair fixtures exercise structural key/token/signature shape only, not
valid curve points, signatures, device registration or admitted invitations.
Operation-specific argument/result types are not generated.

## Verification

- rtk cargo test --offline --locked --test remote_protocol: 16 tests pass.
- zsh tools/stage2_android_source_check.sh /private/tmp/stage2-message-schema-20261004:
  exit 0; production/unit/instrumented sources compile and 423 JVM tests pass.
  Existing warnings remain; instrumented tests compile only, not device-executed.
- Stage2 Python discovery: 125 pass; schema generator tests: 4 pass;
  lifecycle verifier tests: 21 pass; Blueprint verifier tests: 5 pass.
- Strict Blueprint, rtk cargo fmt --all -- --check and git diff --check pass.
  All 36 authoritative rows remain [ ].
- After Rust tests completed, the real offline locked host build refreshed
  current debug provenance. Rust source inventory and final binary are unchanged.
- APK verify-only refuses missing provenance. No unchanged restricted Gradle/
  ADB/TCP retry, historical APK install or physical QR/LAN run occurred.

```text
schema-contract.json 7cc311653c77426c76e7b23705170ee1edefc394c19488dc16165d416166d7da
generate_stage2_android_schema.py 75e4375aedc248cd6a9d85ba272239df9af6961cdb98dc9385cb3f6d1a48839b
RemoteSchema.kt e989491d75ae090819a050d59a046203ffe21b399bedc4d8f5892ca3bc268fa2
FrameCodec.kt b77ee445b228e0332ed83797430c994142c5245b7e9253acf8f22ef4640b16bc
remote_protocol.rs a4dbe9ec148ecbafff8e9cd4ec4464b08cd4a9d5e76f6310d51e5b71952d1c0c
RemoteProtocolCorpusTest.kt 9d4b218dbc81ecd25566e0d27848419ed90954b63d47416f110fb01ccaef20ee
main.jar 24a52bf5ebff436fec7aac4b9949d7040869b9622981b4b24c30150185adce04
unit.jar 4b14e83c18fdaa9e5f58c45ede06964001e87967878669262a65f89b31155089
instrumented.jar a2d7ab9d56a37c3a8506707a1fe769e8173bfcab35e66d130e7baf13dcd6ec75
debug-host-receipt 497fb8f2bf5148bba3e690c5a5066821e9ce56563d8c5bd2bc5efc3673fee230
```

Goal remains active. Source-bound APK, physical QR/direct LAN communication
and recovery, full shared-schema/parity/API lifecycle and signed release remain
incomplete or unverified. No entire Blueprint item acceptance is claimed.
