# Single Activity Pairing Owner

Provisional S2-030/S2-031/S2-037/S2-038/S2-072 progress. The previous goal
turn added current primary-workspace interoperability and was progress.
This turn reread execution skill/specification and current dirty integration,
pairing admission/handshake/storage/cancellation, Activity/Manifest, scanner
receipt and release policy. No nested agents, reset, commit, push or checkbox
promotion. Authoritative Blueprint remains 36 unfinished rows.

## Defect and Fix

MainActivity had an onNewIntent in-flight invitation fence, but the manifest
used default standard launch mode. A repeated external pairing link could
create another Activity/ViewModel instead of consulting the current connecting
registration owner. Per-model generations and storage mutex cannot fence that
new model. MainActivity now uses singleTask and documentLaunchMode=never;
onNewIntent records its new Intent before forwarding to existing admission.
No credential or one-use token retry semantics were relaxed.

The installed Android 35 SDK attrs_manifest.xml documents standard creating
instances, singleTask forwarding to an existing task's onNewIntent, and never
refusing new-document task behavior. Web documentation retrieval failed; local
SDK definition was inspected instead. This is configuration rationale, not
proof of connected system task-manager execution.

Added PairingActivityIntentTest against actual MainActivity. It sends an
ordinary browsable link with no CLEAR_TOP/SINGLE_TOP reuse flags, asserts one
Activity and the same ViewModel, then holds only the connecting admission
state and sends a replacement. State and generation must remain unchanged.
A NEW_TASK/NEW_DOCUMENT/MULTIPLE_TASK delivery must also retain that owner;
a later confirming-state link must replace the invitation normally. The
fixture requires an unpaired owned emulator, never creates a pairing success
or writes credentials, and restores its state. Instrumented source compiles;
the Activity test has not been connected-executed.

New validate_stage2_android_entry checks exact singleTask/never ownership and
rejects task/process/reparenting/multiprocess/no-history overrides. It runs in
the source checker and Gradle preBuild; APK source attribution inventories its
file. Merged release inspection independently calls the same structural policy
and accepts textual or numeric enum representations. Negative tests cover
missing/duplicate/weakened launch configuration and policy mutation invalidating
APK provenance. Scanner state suite now requires 25 exact named tests plus
external_pair_intent_single_owner_verified phase and receipt field; historical
reduced-suite receipts cannot prove this regression.

## Verification

Cwd /Users/mac/Github/zenpi, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Final observed UTC
2026-10-03T21:47:02Z. Existing dirty integration preserved.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-single-pairing-owner-20261004-final: exit 0 through
  pipefail/rg. Current QR fixtures, main/unit/instrumented Kotlin source
  compilation, 474 ordinary JVM tests, all current wire/write/proof/worktree/
  session/ACK/approval/PTY/primary-workspace lanes pass. Not Gradle/APK/device.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': 135 pass.
- Focused entry tests: 2 pass; release inspection tests: 15 pass; scanner
  receipt tests: 14 pass; host attribution tests: 8 pass.
- Lifecycle validator tests: 23 pass; backup validator tests: 5 pass;
  Blueprint validator tests: 5 pass; strict Blueprint pass, all 36 rows [ ].
- Rust fmt check and git diff --check pass.
- Shared APK builder change invalidated host inventory as expected; rebuilt
  actual offline/locked debug host after source lanes, then verify-only.
- APK verify-only remains a missing-provenance refusal. No unchanged blocked
  Gradle/ADB retry or permission bypass; no stale APK installation, physical
  camera/LAN, connected lifecycle/task-manager or release acceptance claim.

```text
AndroidManifest.xml dea2cbdc5e4adb710f17ee6c922b54fe31767ba7575b692672c3d3cd42c9d2e4
MainActivity.kt 948805073f125894b9416b1020100f0ad99f2956e17d41df7c648b5144e09a5e
PairingActivityIntentTest.kt 105334f803d8f036854962eaafb18fa4463b041381adc61020645d5e036695b2
validate_stage2_android_entry.py 418af891ea0e72932ab5a1bfe6631ab0406073d33432515f9c6d4a9175dc5a1c
stage2_android_build.py 9b11baf124f4a9b5e774f6437eca0bab1718a58216b2fc5aeb51f732963fcb1b
stage2_scanner_android_smoke.py 25af6ce9f50f79558b110702a3a64cecb23e776b26d3777b80233aaaab3ea680
main.jar 43f5ed339759a6a1c6c0acda8ae2f3f661fc062f5763c6c81cf9d5381fde9970
unit.jar 8f41468142c6dc64e1feb76c9ada37e3302a102d83d913c0a041be511dce97fc
instrumented.jar 91c2860683358c4a27d2114bc748e8ac929e6e72035fba4f67de0aaa7cb12551
debug-host-receipt 4a563bb20c337eabb4f170ae4cbbe8835a381c4d6d5db686b3efaf907cff7539
debug-host-inputs 0096b9661b31def50f4d3d448893d50397ad77893bca2656ffa9d5bb7e2e628e
debug-host-binary 677c02131aa670e18cd3813bb1a8016026b375a3155f9c010b0c62f29d287df4
```

Goal active and incomplete. Fresh source-bound APK, actual camera QR/direct
LAN two-way sync and recovery, API31/34/35 lifecycle, complete parity,
migration/reset/uninstall and signed reproducible release remain open gates.
