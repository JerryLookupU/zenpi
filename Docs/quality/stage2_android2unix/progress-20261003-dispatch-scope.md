# Current-Scope Android Write Admission

Provisional S2-033/S2-034/S2-035/S2-036/S2-052/S2-054 progress at
2026-10-03T19:49:05+0800. Previous goal turn repaired atomic projection
publication. This turn inspected the current pre-persistence write path and
found Connected admission did not recheck captured scope/capabilities, unlike
Error admission. No nested agents, worker launch, checklist promotion,
commit/push, signing or unrelated cleanup.

## Repair

WorkspaceDispatchPolicy now checks owner/revision and the listed target
project/session for every new write before executeWorkspace saves pending
intent or starts transport. Non-workspace operations must use the currently
selected project/session. Workspace open/select/close can target a different
listed project, preserving primary-tab selection, but cannot borrow a previous
session or revoked workspace capability.

Operation-specific capability checks apply in Connected as well as Error.
Steer/cancel/approval revalidate live target identity, state and approval digest/
worker-session binding. Secondary mutations recheck their stable target,
revision, directory/worktree/concurrency permissions and move destination.
Worker prompt and terminal writes bind the active secondary tab; terminal
writes also require the current terminal instance. Session operations require
the main tab and session capability; rename binds its captured journal revision.
Unknown/unsupported operations are refused. Host ACL/CAS/owner validation is
still authoritative and cannot be bypassed by these early client checks.

An already unresolved pending command is intentionally different: foreground,
network/reset/revoke/phase gates still apply, but only the identical saved
envelope is eligible for explicit retry. It is never retargeted to the current
projection or assigned a new ID. Network recovery does not auto-send it.

## Verification

- Final current-source command:
  zsh tools/stage2_android_source_check.sh /private/tmp/stage2-dispatch-scope-final-20261003
  exited 0. Production, unit and instrumented Kotlin compile; 319 JVM tests
  pass. Existing compiler deprecation/nullability warnings remain.
- Eight new JVM tests cover Connected old-scope/owner/revision/capability
  refusal, cross-project primary selection, exact pending retry, terminal
  instance, active worker tab, quota permission, session/main-tab/journal and
  secondary CRUD/move/individual capabilities. Existing positive/Error/offline
  and unknown-write tests remain passing. Model/reasoning positive fixtures now
  explicitly carry their actual operation capabilities rather than relying on
  the previous blanket Connected admission.
- SessionHostTest now asserts a captured rename from the prior desktop session
  fails admission while the current-session rename is eligible, with no pending
  write created. This device test is compiled only, not run on the current APK.
- Python Stage 2 tool discovery: 86 tests pass, exit 0. Backup/lifecycle/
  Blueprint validator unit tests: 5/15/5 pass. Strict Blueprint, cargo fmt
  --all -- --check and git diff --check pass. All 36 items remain unfinished.
- Current release-host source attribution verify-only passes with binary
  9f1ff46c2fd82ccad171534cbde89b3f7c6b49babaef8fd0f7686ad3019caaa8.
- Current APK attribution verify-only exits 1: missing successful build
  provenance. No unchanged failed Gradle retry or historical APK install.

The earlier 314-test compilation ran before subsequent tests/device assertion
and final policy formatting changes; it is not the final source evidence.
The final fresh command above owns the following jar digests. All required
command handles from this turn are terminal.

```text
WorkspaceDispatchPolicy.kt 4976d5eba11332421fbf61e661f6b8861e32b64e46e7b174103c073dd16ff31a
WorkspaceDispatchPolicyTest.kt 1a2326655aa77ed9c43f2b47f0e73e425a787a6b1ea82aa8dc7c8e31592537e1
TerminalControlTest.kt dacaa1d5741f8a070bc566c87bc23ebf2ebd8fb498c783a0c6cf1b533758391a
WorkerControlTest.kt 1c93908e4319cdb341c7f9439ede83e1d8c45eb6d6af14a43ad3c8a36056dc72
ConcurrencyControlTest.kt 7a47c2f2454b174758d3e25ae37ed1283ea9dad6118ff1d4ac5e2a13cf302e39
SessionCommandTest.kt f2812611f625e820132443603154de7cdd2db2fa44b459a700064154b66e71ff
WorkspaceCommandTest.kt 2e71940a167b48704e5bc492cea0adc2982031e002fd572f8a0d820ea31385d6
SessionHostTest.kt e438e1dc1c7a5b55c90904ac1ab0c9e1b12166e70f4fd4b0fda0eb4ebbc70314
main.jar b6b7efd2654e8597701ef3f9ff05a561214fcdf1e03c1904daf6dac38f1c9f13
unit.jar f19ec239505a0140ebc9de46c62259736946c53960cdf8de7563ba2ee669f398
instrumented.jar b6db64c87f0d7b384bbbee742fa9d79b90c2ee5f5b6819f26f946415ac9e284b
```

Goal remains active. Current APK packaging, connected parity/lifecycle,
physical-camera QR/direct LAN and signed install/upgrade gates remain open.
