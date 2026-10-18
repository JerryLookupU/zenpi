# Session-Scoped Android Intent And Confirmation

Provisional S2-034/S2-035/S2-036/S2-038/S2-054 progress at
2026-10-03T19:06:01+0800. Previous goal turn was progress: current owner
control routing and prepared desktop-origin switch verification changed
authoritative source and tests. This turn inspected the current UI/ViewModel,
recovery policies and acceptance commands before extending the implementation.
No nested agents, worker launch, checklist promotion, commit/push or cleanup.

## Findings And Repair

The message composer was keyed only to project ID. A direct same-project
session replacement could carry a draft into the new conversation. Session
and approval confirmations used unscoped remember state; changing session
disabled some confirmations, but returning to the original session could
make an old confirmation actionable again. Worktree removal/open-workspace
confirmations also lacked the complete pending/revoke/offline/capability UI
fence, even though backend dispatch still validates them.

MainActivity now binds message/task/terminal drafts and session/approval
confirmations to pinned host fingerprint/device and selected project/session.
Workspace and subtab confirmations reset on that boundary, with subtabs also
resetting on active-tab/revision replacement. Ordinary same-session event
updates preserve drafts. Workspace/subtab confirmations disable for offline,
unknown write, pending revoke, reset or missing relevant host capability.
Terminal drafts cannot migrate even if a terminal identity is reused.

SessionScopeScreenTest prepares four Compose scenarios: same-session draft
retention versus host/session reset; approval confirmation cannot resurrect
after switching away/back; reused terminal identity resets its unsent input;
worktree removal becomes disabled on revoke and disappears on session switch.
Existing new/rename tests now require dialog removal rather than a stale
disabled dialog. The production SessionHostTest leaves an unsent rename
dialog open before desktop `/new` and requires both dialog and draft to
disappear after the automatic snapshot/cache replacement.

The session device harness now selects the production SessionHostTest plus
SessionScopeScreenTest, SessionNewScreenTest and SessionRenameScreenTest:
exactly seven instrumented tests and five ordered production phases are
required. Python negatives reject the former one-test result and surplus
counts. This is prepared validation, not evidence of a device run.

## Verification

All commands ran from /Users/mac/Github/zenpi.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-session-ui-scope-final-20261003: exit 0, production,
  unit and instrumented Kotlin compile; 297 JVM tests pass, 1.208 seconds.
  The four new Compose scenarios and modified connected workflow compile
  only; no Compose runtime, rendered screenshot or device/LAN pass is claimed.
  An earlier source check finished successfully before the final UI additions;
  only this final run is used as current-source evidence.
- python3 tools/test_stage2_session_android_smoke.py: six passed, exit 0.
- python3 tools/test_stage2_android_build.py: ten passed, exit 0.
- python3 tools/test_stage2_desktop_android_smoke.py: fourteen passed, exit 0.
- python3 tools/test_stage2_headless_android_smoke.py: nineteen passed, exit 0.
- python3 tools/test_validate_stage2_android_lifecycle.py: fifteen passed,
  exit 0; test_validate_stage2_android_backup.py: five passed, exit 0.
- python3 tools/validate_stage2_android2unix_blueprint.py --strict: exit 0,
  all 36 items remain unfinished. git diff --check: exit 0.
- python3 tools/stage2_android_build.py --verify-only: exit 1, missing
  current APK provenance; no adb/device invocation followed. Previous Gradle
  local-socket restrictions were not bypassed or retried unchanged.

No Rust source changed this turn, so the prior Rust session/owner gates are
historical supporting evidence, not described as a new run. All required
command handles are terminal.

## Digests

```text
MainActivity.kt 7ef1955d6cddea11b3df6ff8f6ad62de3654ef3d1df76f7d9e697c6217f73060
SessionScopeScreenTest.kt 72f7f391cb2a2867a3bc8e3291a5938bfac9489aa3f2404079bd59dfe31007c6
SessionHostTest.kt 6471a5ee6621bd17178e84e0f0cdc8c4a02e0602ad250529fd7b5d6c3a5ac983
stage2_session_android_smoke.py 61da3a869e93d4df492941d01a14319beb4a1adb280a28dccbb77cd5514201ae
test_stage2_session_android_smoke.py c6a1cc374737f6d360acaa3b4f175f2111d317266ae31fb4fc8aa7eb81389f28
main.jar 11af84b177b70e2a4cdebc97d19059daa9e7beec2d12497ecef9ddf5d56737fd
unit.jar 21c5a06d549ece9f0ed6ea17138ef5bac2adb58d31933c3ea492737d47975b06
instrumented.jar 5d10b461d92977cbea0359948239e90e5e20998bd289f615d5c8a517ab1614ec
```

Goal remains active. Current APK, real camera/direct LAN, connected lifecycle
and UI, complete cross-client parity and signed reproducible release remain
incomplete or unverified. Source compilation does not waive those gates.
