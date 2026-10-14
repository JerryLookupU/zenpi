# Consume External Pairing Intent Data

Recorded 2026-10-04. Partial S2-031/S2-038/S2-052/S2-072 evidence, not Master
acceptance. Previous goal turn was progress: successful registration retires
the invitation from screen state. Current execution skill, Blueprint/spec,
dirty worktree, Activity entry and recreation regression inspected before edits.
No nested agents, reset, permission bypass, acceptance marks or repeated denied
Gradle/ADB attempt. All launched validation handles terminal.

## Repair

MainActivity stored the incoming external QR URI in its task Intent even after
ViewModel admission. The URI includes a short-lived one-use credential. Its
local reference was unnecessary once admission had occurred, and keeping it
made the original invitation available to later Activity consumers/recreation.
This was not a demonstrated network-authentication bypass or token disclosure.

The Activity now takes the URI and installs a copied Intent with data removed
before super.onCreate/onNewIntent and ViewModel admission. Action, type, flags,
categories, component and extras remain untouched. Fresh creation admits once;
saved-state creation does not re-admit. onNewIntent observers receive the copy,
then the current owner handles the extracted URI under existing pending/reset/
in-flight registration checks. No raw URI logging or saved-state token added.

PairingActivityIntentTest no longer waits for retained secret data as proof of
delivery. A harmless unique fixture extra identifies each actual delivery;
the test requires one Activity/ViewModel, unchanged in-flight registration,
null Intent data and retained ACTION_VIEW. After cancelling confirmation it
recreates the Activity and requires the same ViewModel, null intent/invitation
and no generation advance. The named test count remains unchanged. Scanner
acceptance requires external_pair_intent_consumed_recreation_verified exactly
once; Python negatives reject missing or duplicate markers. Security/network
matrices record the scoped claim and remaining device gate.

This clears the local Intent data reference only, not arbitrary extras,
ClipData, sender/system task history or JVM memory. No guarantee is made about
Android's process-death replay of an external launch; connected lifecycle and
process-death evidence remain separate requirements.

## Verification

Commands from /Users/mac/Github/zenpi:

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-consumed-intent-interop-20261004: exit 0. Main/unit/
  instrumentation compile; 593 ordinary JVM tests and every explicit Rust/
  Kotlin lane pass, including current QR pixels, P256 pairing/challenge proof,
  workspace/worktree/session/ACK/approval/PTY/model/resource/file/diff/history/
  worker and real-owner prompt/cancel behavior. Not APK or device execution.
- Python Stage 2 tool tests 166 pass; scanner verifier tests 19, lifecycle
  validator fixtures 25, Blueprint fixtures five, controller fixtures 15.
- New actual Activity delivery/recreation assertions compiled only. No claim
  of runtime recreation, camera hardware, Keystore or physical LAN acceptance.
- Strict Blueprint, entry validator, cargo fmt --check and git diff --check
  pass. 36 [ ], no [_]/[x]. Controller validate launches nothing; status has
  zero active/live/authenticated claims, requests or queued handoffs.
- After all Cargo lanes ended, debug attribution rebuilt and verify-only pass;
  release verify-only and retained independent comparison with publication
  check pass. Rust production source unchanged by this turn.
- APK verify-only still refuses missing provenance. No user device answer;
  current signed APK/install/upgrade, physical camera/direct-LAN bidirectional
  synchronization/recovery, connected API31/34/35 and full unsuppressed Rust/
  network/supported-host gates remain required. Goal active/incomplete.

```text
MainActivity.kt 406862fd95675fdf765767e660a38b153556aa182d302f96d88d88e945b6b7e9
PairingActivityIntentTest.kt 53e18e9c7924c4be46bcd18644f2b78adbe1b42f3d960a74abf50dd442e4d791
scanner-harness ca2f63490cee389656c428bdc2db8d574395dff8da075f5f4baa68c2c5dc7f30
scanner-harness-tests 4b7ce15b38ce30a2384b9459d8fac74c8b259b92bd90a0bf99d8639a9d5b17eb
main.jar e378fd0e817aa28a7b599d7603b4a216c440e2a583d26047c24e61d90c7d4cf0
unit.jar 53579dbfbcf54a05b16f56427cf769a0276f01dda11463a5ed8d1aa958b6d946
instrumented.jar 36d6fac94ae6e8766dc47d173a097fdebab6344df0b5e7385546355fea3d961e
debug-receipt 174a6811ab5b22edee5c3713b5bdfe4563e929987f91964a3588aa25e220445d
```
