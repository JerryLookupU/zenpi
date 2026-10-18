# Stage 2 Cancellation Fences Captured Tool Approval

Recorded 2026-10-04. Related S2-019 routing, S2-036 approval controls,
S2-052 security and S2-054 parity. Partial evidence; all 36 Blueprint rows [ ].
Cwd /Users/mac/Github/zenpi, dirty-worktree HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b. Existing changes preserved; no nested
agents, reset, push, permission bypass or stale APK installation.
Blueprint SHA-256 d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
specification SHA-256 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

## New Evidence

The explicit Kotlin/headless owner lane extends the actual Always-policy tool
loop. After one allowed write and one denied write, the second controlled
backend completion requests a third production write_file call. Its live
pending preview feeds Kotlin approval construction. The generated Allow is
captured but not sent; Kotlin instead cancels the original running prompt.
Once the host reports cancelled, the pending approval disappears. Sending the
captured Allow returns InvalidRequest and android-cancelled.txt never exists.
The session contains no approval_consumed record for the third tool call.

Fresh disk recovery uses a distinct Agent and a backend that must never run.
The original captured decision returns OwnerMismatch under the new owner.
A new request with the current required_owner and workspace_revision but the
same cancelled approval target/digest also returns InvalidRequest. The latter
check establishes target invalidation rather than merely a stale-owner fence.
Recovery leaves the session journal unchanged, preserves the one allowed file
effect, and executes zero provider calls. Kotlin validates actual correlated
refusals and empty pending approvals alongside prior prompt/steering/cancel
and approved/denied decision receipt assertions.

This proves cancellation completed before a previously unsent Allow, not every
simultaneous Allow/cancel ordering. The controlled backend drives production
Agent/coordinator/tools/filesystem code; this is not external-provider,
Android Keystore, production TCP/TLS, physical camera or direct-LAN evidence.
No production runtime logic or permission policy changed for this extension.

A separate deterministic coordinator regression was added on continuation.
The waiter is held inside its cancellation predicate, outside the coordinator
lock. A digest-fenced Allow is accepted and duplicate response refused. The
test then sets cancellation before releasing the waiter: the wait returns
Cancelled, bound-session subscribers are notified, accepted/pending state is
cleared and late mark_persisted/re-answer both return UnknownRequest. This
uses Agent::with_echo only to obtain the production session-bound coordinator;
no provider or tool execution is claimed for this narrower test.

## Verification

- Initial checker /private/tmp/stage2-approval-cancel-race-20261004 exited 101:
  cancellation and first late refusal passed; fresh recovery returned the
  correct OwnerMismatch rather than the fixture's expected InvalidRequest.
  The failed log is retained and not counted as a pass.
- Corrected checker /private/tmp/stage2-approval-cancel-final-20261004 exited 0;
  this was intermediate evidence before the current-owner refusal assertion.
- Final command: zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-approval-cancel-current-20261004. Exit 0: 529 ordinary JVM
  tests, main/unit/instrumented source compilation, production QR corpus,
  explicit interop lanes, six-test desktop target and real tool-loop lane.
  Extended owner lane: one Rust test and seven Kotlin phases passed.
  Instrumented compilation is not connected-device execution. No test/source
  edits to that lane or Android sources after this final checker. A separate
  Rust coordinator test was subsequently added and verified below.
- python3 -m unittest discover -s tools -p 'test_stage2_*.py': exit 0, 137 pass;
  log /private/tmp/stage2-approval-cancel-python-20261004.log.
- python3 tools/test_validate_stage2_android_lifecycle.py: exit 0, 25 pass.
- cargo fmt --check: exit 0 (completed session 92964).
- Continuation's first remote_approval_wakeup run: exit 101, fixture omitted
  attachment workspace so Agent had no coordinator. Fixed fixture setup,
  without runtime changes. Initial fmt check also failed import formatting;
  corrected and reran cargo fmt --check, exit 0.
- rtk cargo test --locked --offline --test remote_approval_wakeup: exit 0,
  two tests pass. Uncompressed repeat with cargo test and identical arguments:
  exit 0, two pass; /private/tmp/stage2-approval-cancel-wakeup-20261004.log.
- git diff --check and strict Blueprint validation: exit 0; 36 unfinished,
  zero worker-self-tested and zero Master-accepted rows.
- python3 tools/stage2_host_build.py: exit 0 after Cargo lanes; --verify-only
  revalidated current debug binary and build receipt on continuation. Cargo's
  later coordinator-test builds invalidated the attributed binary temporarily;
  verify-only correctly exited 1. Rebuilt through the same helper and verified
  again after all Cargo runs; exit 0, original binary/input hashes restored.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing current
  APK provenance. Unchanged Gradle/ADB permission failures were not retried.

Final log: /private/tmp/stage2-approval-cancel-current-20261004.log.
Wire results: /private/tmp/stage2-approval-cancel-current-20261004/android-prompt-results.json.

```text
remote_headless_commands.rs fba4546a2d9b2a8e5903897dce12e0fb6a38e9cdabedc649f5f203df1f457050
AndroidRustPromptTest.kt 40d355a0d2ea1c9325a98476bd7ca65c9681631a39b2ef2bafcfdc8583ac314a
final-log 8682304ef462f08ed87a9de73f9a04b4df4c6111a1aca68c85d2c12f65f506ab
prompt-results 2a07b066f693e602c1a0ad3b43c5e5ba0322ee2db9098ff0ffb82eb267ae06e2
python-log 40cb7d5442f1a5a0af005fb124bd4df97598089a44b9e73a046b79e5c021a9c5
unit.jar 2a477399d300b25382cd798da53d4597838a4a438c01797ac44927b2d6fec18c
initial-debug-host-receipt a11ca022113329f72aa5f5ffbe0a02657fa27c8a0eea3b593aeedfeeadad099d
final-debug-host-receipt 51507e1b2992be1de0a16723d64c9abe977069e546a96988bf30029250f1ded3
debug-host-inputs 55eedcd61b52e995fbe4ae2fb6531d3d8337eafd5dcc36ce8102ba085d2ec3ed
debug-host-binary c9b9af949a6d94a3659e58e16496ff770f26baae12cc95d78178dacc87183a22
remote_approval_wakeup.rs e84de71fa486f95e4a2bbc32823cb1ea0134e24cd1bf559443a9926f3e23d764
wakeup-log c02bac7915053497257a1f63a8a5d999b9f96a93a61771bb5f28810851c664d2
```

Goal remains active and incomplete. Current APK, physical QR/direct-LAN
bidirectional sync and recovery, connected API31/34/35 lifecycle, complete
desktop parity and signed reproducible release remain required. These focused
cancellation assertions do not replace any of those acceptance gates.
