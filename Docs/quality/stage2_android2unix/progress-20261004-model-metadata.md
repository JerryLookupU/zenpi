# Stage 2 Equal-Selection Model Metadata Synchronization

Recorded 2026-10-04T08:25:02Z. Partial S2-036/S2-054 evidence, not acceptance.
Previous goal turn progressed pairing handshake identity-read cancellation.
Canonical cwd /Users/mac/Github/zenpi; dirty HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push or permission bypass. Blueprint SHA-256
d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.
All 36 authoritative rows remain [ ]; zero [_]/[x].

## Finding And Change

ModelStatusCodec.newest retained the entire displayed status when the selection
revision and catalog snapshot were unchanged. Rust Agent.set_context_budget can
change the effective budget without changing either selection or catalog, so
an Android model panel could keep showing the prior budget despite receiving
a fresh status. Catalog navigation and status metadata have separate lifetimes.

Equal-selection/snapshot merging now adopts the incoming status and retains
only the user's current catalog page. The shared retainsPage predicate binds
history preservation to project/session, selection revision and complete page,
not equality of all status fields. Both subscription publications and paged
read publication use it. Back navigation merges current status metadata into
the retained page so returning cannot restore the old budget. New revisions
and changed snapshots still replace pages; regressing revisions and malformed
incoming status retain the prior validated display. No wire/cache schema or
remote write behavior changed.

The new JVM regression covers metadata merge, complete page retention, foreign
scope, newer/lower revisions, changed catalog and back-page budget preservation.
The actual Rust desktop model lane changes Agent budget to 32000/2048, requires
unchanged selection/catalog and exact journal bytes, then Kotlin decodes both
real statuses and checks the merged result across existing model/reasoning
CAS/restart phases. Extended ModelStatusScreenTest asserts updated budget text,
absence of the old text and unchanged catalog; compiled only, not device-run.
Protocol, Android and network/parity documentation record those boundaries.

## Verification

Final command after all production/test edits:
zsh tools/stage2_android_source_check.sh /private/tmp/stage2-model-metadata-final-20261004.
Canonical cwd; exit 0. Current main/unit/instrumented sources compile, 575
ordinary JVM tests and every explicit current Rust/Kotlin lane pass. Includes
six-test real desktop target, actual budget metadata, regenerated current Rust
terminal QR fixtures, software-P256 pairing, session/ACK/restart and actual
Agent prompt/tool/running-worker cancellation. Per-lane homes and OUT isolated;
credentials unchanged. Instrumented assertions compiled only, not executed.
The earlier source run also exited 0, but additional back-navigation/Compose
edits required this final rerun. No source edits after final launch; silent
observations were polled using the same live handle through authoritative exit.

- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 tests;
  /private/tmp/stage2-model-metadata-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 verifier
  fixtures, not connected API31/34/35 lifecycle acceptance.
- python3 tools/test_validate_stage2_android2unix_blueprint.py: exit 0, five tests.
- Source entry and backup validators: exit 0, source policy only.
- cargo fmt --check, git diff --check and strict Blueprint: exit 0; 36 [ ],
  zero [_]/[x]. Metadata validity is not product completion.
- python3 tools/stage2_host_build.py and --verify-only: exit 0 after all final
  Cargo lanes; current debug binary reattributed, Rust production inputs unchanged.
  Not release reproducibility acceptance.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing APK
  provenance. No unchanged Gradle/socket/ADB retry, old APK installation or
  permission bypass. Current APK/physical evidence remains absent.

```text
ModelStatusCodec.kt dbec1a3a502a501847ab5ea7d5449943a8b6459a145e8fd78d485b99931b6e2e
PairingViewModel.kt bca758ae6211e57aad3b8c024fa881dfa11c52640bbc4955ddbe02d90e181d01
ModelStatusTest.kt 9ae80b7dcf5a908dbd6eb6fd0988cf4e84ac83a2e8cf2f75f7639e0270abde00
AndroidRustModelTest.kt 61397bc3b22bad1a2f6b445d3fde3b5d60723e1a35875f117261f6fa7a893387
ModelStatusScreenTest.kt 429fdf117505cf6f945dd7ef26d3185d0ee2dd93b8b462078b1249fd1e17f7d4
remote_android_model_interop.rs 318d3c5f5ee0f3fc752057526d84583d481c9163e381b0d4686cc1f0dcf9b204
source-log 817ff1448049cebea982a25000e1a59d1c32d20497e90036ba5bda4d44abe7d2
python-log 4b836ea9e5ecea63d6456b5a985831e3dae11c257c0d2653d148155744cc19e4
model-results 8c14bf6a5f0528cd342adb136a8944164cdd48d107d81e5eadeafa0ab40da9d6
main.jar f9c6cc8f427202a03037dc6f84e004d3bed44614bde395fdfa565cd1eaf2f27b
unit.jar 42a61b28f5433545f5311393798726645d15810ea67719415e5c60adbfd70939
instrumented.jar 8e803880dd04f03ad420e12d119b9cc49d3b584ef2669dc0aec78bae659de1d0
host-receipt dc7dad2d5abc763ae98576de83c26e9737401f4f6995d8e052b7110379224865
host-inputs 9971547e0a69a38c9a30512ee37340ab10553825f20ae10d253c5c600e7faedd
host-binary 8c8dfc56ed6956ace031c8682d713b4c917262faba1ec6a60375e3f219532b89
```

Goal active/incomplete. Fresh APK provenance, physical camera QR/direct-LAN
bidirectional synchronization/recovery, connected API31/34/35 lifecycle,
complete desktop parity and signed reproducible release remain required.
The current implementation/interop fix is progress; neither completion nor
blocked status is justified by the remaining external runtime gates alone.
