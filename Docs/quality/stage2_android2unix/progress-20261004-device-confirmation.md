# Stage 2 Device Confirmation Scope

Recorded 2026-10-04; partial S2-031/S2-037/S2-038/S2-072 evidence only.
Previous turn was progress: checkpoint file admission and macOS release link
policy were repaired and verified. Dirty worktree preserved, no nested agents
or denied Gradle/production-listener retries.

## Repair

Revoke and local-forget confirmation state used unkeyed Compose remember,
so an open confirmation could survive a replacement host. Both now reset on
the full PairedHost value, including endpoint, fingerprint and device ID.
Forget confirmation also resets when resetRequired changes. A shared
DeviceActionPolicy gates revoke, status query and forget buttons. Revoke and
status dispatch use that same policy before any panel closure/storage/network
work. Scanner ownership, reset and pending intents retain their distinct
fail-closed semantics. Explicit local reset is still not host revocation.

Two new JVM tests cover scanner/reset/phase and independent offline/reset
semantics. A new Compose test exercises open revoke/forget dialogs, scanner
disablement, endpoint/fingerprint replacement and reset-state replacement,
asserting no stale callbacks. It emits device_confirmation_scope_verified.
This Compose test compiled but has not executed on a device.

Desktop suite now expects 93 tests with workspace and Doze enabled; scanner
state suite expects 26. Both require the new exact, unique named phase.
The API31/34/35 lifecycle matrix requires its receipt boolean and exact phase.
Scanner negative fixtures reject absent, duplicate or summary-only markers;
lifecycle negatives reject missing boolean/phase. Older receipts cannot claim
the new coverage. No historical receipt was rewritten.

## Verification

zsh tools/stage2_android_source_check.sh
/private/tmp/stage2-device-confirmation-source-20261004 ran to terminal exit 0.
Main/unit/instrumented Kotlin source compilation passed, followed by 590
ordinary JVM tests and every explicitly invoked Rust/Kotlin interoperability
lane, including signed device proofs, worktree, session, ACK, approval,
terminal, workspace, model/resource/file/diff/history/task reads, real owner
prompt and worker cancellation. This is cached-source/JVM evidence, not
Gradle APK packaging, production TCP or connected Compose execution.

Full Stage 2 Python tools: 159 pass; lifecycle fixtures: 25 pass; Blueprint
fixtures: five pass. Scanner harness tests: 16; desktop harness tests: 17.
cargo fmt --check, git diff --check and strict Blueprint pass. All 36 checklist
items remain [ ], zero [_]/[x].

The source checker rebuilt the debug binary, correctly invalidating its old
attribution. The debug builder refreshed it after the checker finished;
release attribution remains valid and unchanged. Android verify-only still
refuses missing APK build provenance.

```text
MainActivity.kt 6d08a9ebf05f3ec56251ec8b4d394fdcb4c0fb3bd51af0cd2d2c595e83fbe185
PairingViewModel.kt 73a83a67123c48990176e74e92f2fe4e4be4ff98267226820b7a078cea95295d
DeviceActionPolicy.kt 124cf919b344220f038356c77fa9efdd0bc32edc9b7d94754f81b59c7276096d
PairingScreenTest.kt 5ba373d06a3101f3052fa97ff80f1ad82ee4274cc8f08aadd9382afece030fcb
main.jar abfe729defd0c3cd22d3d281a9e28c4bb24c2b19cd6856c08ebf8a5f50cc56c0
unit.jar 8e6bb1e0c33e7b40573c883e579c16a49dc5829843b2e56ce7c6dc9942d0654a
instrumented.jar b1f668eb5d59377c9047dea40ee4c187f4b67d9d40c99c9bbc796b655bd2d65c
debug-receipt 10e38bce5fe8c78e9893738d8747cf1d27506a9a230b6bbe00915f73fa98cb16
```

Goal remains active/incomplete. Current APK and actual confirmation test
execution, physical camera QR/direct-LAN sync/recovery, connected API31/34/35,
signed install/upgrade and full-library gate closure remain required.
