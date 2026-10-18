# Task Tool Arguments Paging

Provisional S2-019/S2-020/S2-036 progress at 2026-10-02T11:25:56Z.
Current dirty worktree inspected; user changes preserved. Previous goal turn
made implementation progress (metadata-bound tool previews). No nested agents,
new execution-worker admission, commit/push/cleanup or checklist promotion.

## Behavior

TranscriptRead accepts the exact request_id/turn_id/tool_call_id/offset/digest
shape for a completed task invocation. It checks current device Read authority,
master task identity, runtime-confirmed worker journal/session/slot, the complete
master-bound history manifest including metadata hashes, selected assistant
message membership, one matching call and bounded tool name. No client path
selection or raw host execution metadata is exposed.

Only the arguments value is structurally redacted and serialized. Both raw
encoded and redacted documents are limited to 64 KiB, with 8192-byte UTF-8
pages and a digest-bound continuation. Responses retain task/message scope and
add tool_call_id/tool_name. Ordinary content paging behavior remains unchanged.

Android opens a retained invocation through its OpenInFull icon, displays its
tool name and selectable monospace arguments, and uses existing next/previous
message controls. Codec and navigation bind call ID/name as well as task/turn.
Selected invocation state is memory-only, cleared by authority/lifecycle/scope
loss; dismissal returns only to validated history. Offline/pending controls are
disabled. Calls omitted beyond the eight-call preview remain a separate frontier.

## Verification

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-tool-arguments-home.
Completed commands exit 0:

- rtk cargo test --offline --locked --lib remote::transcript -- --test-threads=1:
  six passed, 429 filtered, 4.24 seconds. Full UTF-8 argument reconstruction,
  redaction before slicing, exact raw cap admitted/one more refused, redacted
  expansion cap, missing/duplicate/wrong-role call and wrong digest refusal.
- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  final twelve passed, 423 filtered, 6.02 seconds. Actual deterministic-backend
  read_file execution opens its arguments; foreign device/turn/call, mixed history
  shape, extra fields and metadata tampering fail. Pool restart reads the same
  arguments without creating workers or changing either journal; revoke refuses.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test core_session --test runtime --test remote_parity
  -- --test-threads=1: 142 passed, six suites, 34.55 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- /bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-tool-arguments-final:
  production/unit/instrumented Kotlin sources compile against cached API35;
  249 JVM tests pass, 1.189 seconds. Three new JVM cases cover call-bound wire
  paging, malformed/foreign/oversized input, previous-page identity, dismissal
  and authority cleanup. Narrow 320 dp Compose navigation/offline test compiled,
  not run on a device. Existing warnings remain.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 pass, 0.070 seconds.
- cargo fmt --all --check; git diff --check; python3
  tools/validate_stage2_android2unix_blueprint.py --strict: pass; all 36 unfinished.

First Kotlin compilation failed on an unqualified Alignment reference; corrected
it and reran through terminal success. Failed compilation is not passing evidence.
Final Android run includes the monospace rendering change. Final controller run
includes restart/revocation assertions. No TCP/Gradle/adb bypass or retry. Every
observed command handle is terminal. Provisional evidence is not an acceptance
receipt with stdout/stderr captures. No LAN/provider HTTP/device claim.

SHA256:

```text
src/remote/transcript.rs dea29ef5d347a65a95841f6dd10b4f54447ddcb20daa3999670c51bec0a7dfa8
src/remote/worktree_host.rs 798ef75978c425a5cb696418f1a08eb6868830c9bb1d32a9b90fbf72db88fd0b
android/app/src/main/java/zenpi/transport/TranscriptPageCodec.kt 553dbb23d31ed396bb558e5ced56248016a14b768ebf63354633dd6ce4a32389
android/app/src/main/java/zenpi/transport/TranscriptNavigation.kt 9cc8ae0aa01caf651dfe6642b8409b2706d93a8363a0db0506d77041781cf017
android/app/src/main/java/zenpi/transport/HostClient.kt 08707148675bb97c9914afc8fc4183530824f844fa04f91d0d32e1a01cec61ed
android/app/src/main/java/zenpi/pairing/PairingViewModel.kt 1146bafe5a0be94526c5061fe571c854e7b164bc4ec5d66c0f9857273e7b50cd
android/app/src/main/java/zenpi/pairing/WorkerHistoryNavigation.kt 57f82eeb00145e982fd5baafcbda46544f5cf57b408f1cef9a8538e65bdeb18c
android/app/src/main/java/zenpi/pairing/AuthorityLossPolicy.kt 967239a15c3eeb3f483d90f030b6691fc4d3f641fb17ab0e90eb3bc069ed6292
android/app/src/main/java/zenpi/MainActivity.kt 4af3c4659fe0d3c83e202dd26f9b57cb9dab589907235eab90a7654bbb8a7378
android/app/src/test/java/zenpi/ToolArgumentsTest.kt 34cf6b24631c84bbdae75dfcedb7d77de42fa546de5bded5de3df174278a1f73
android/app/src/androidTest/java/zenpi/WorkerScreenTest.kt 5d1657e53fa1bfe2e889de7aab829fdfaff9a443fa9a7aa67a4836a434bfa504
main.jar ac332b8125ad12ed9aecac8c4acf3323e208f7d2ed6f5449fddf45dc7a0931bc
unit.jar ee4287d9275782f5ee52ae0ef10ba0559e6fc641138324f1ce477b67d2cd6940
instrumented.jar 5ba68193f13c104821fc41d5cf93ff41a640a41e2714dbc1a03351a454555728
```

Artifacts: /private/tmp/zenpi-tool-arguments-final. Real API31/34/35 camera QR
pairing/LAN desktop sync, network and process lifecycle, signed APK/release,
omitted invocation paging, live/failed partial worker history, local TUI worker
controls, settings and interactive PTY parity remain unverified or unfinished.
The full goal stays active; no authoritative requirement is declared accepted.
