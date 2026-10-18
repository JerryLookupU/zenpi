# Desktop Session Switch And Current Owner Routing

Provisional S2-019/S2-020/S2-033/S2-036/S2-038/S2-054 progress at
2026-10-03T19:00:22+0800. Baseline HEAD is
1a8ff1acf72e0816fad503a05df2b1d18e07361b with existing user/generated changes.
Requirement digest: 9887f784c944db898610e136dca2774496abb2f80b3e5a8fdd6a0b32659e009d.
Specification digest: 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.
The preceding goal work made source/protocol progress; the intervening RTK
request was a tool-state check, not product acceptance. Current authoritative
state and old command handles were inspected before further work.
No nested agents, worker launches, checklist promotion, commit/push or cleanup.

## Changes

The prepared production session workflow extends Android rename/new/resume
with a desktop-originated `/new` while the phone is subscribed. The harness
observes ordered instrumentation phases and issues the real TUI command. The
phone does not invoke refresh in this phase: only initial post-grant empty
projection recovery permits that fixture action. Convergence requires the
encrypted cache to match current session, transcript, cursor and workspace
revision, preventing a UI callback from racing ahead of durable cache commit.

Host evidence requires a distinct empty local journal and exactly one local
transition; original journal bytes must be unchanged. Workspace revision
advances once, remote metadata/receipts/bindings remain unchanged, other
sessions and top-level checkpoint state remain unchanged. Two screenshots
cover the restored conversation and automatically switched empty view.
Malformed, extra, foreign, float-revision or duplicate evidence is rejected.
The prepared harness still refuses missing/stale source-attributed APKs.

Local TUI and both headless open/resume routes republish input and approval
handles after durable mapping commit and Agent replacement. New regression
coverage uses real asynchronous JSONL streams for both headless routes:
current-session slash queue enqueue/list succeeds, old-session enqueue fails,
an actual shell approval is visible and answerable, denying it writes no file,
and queued input exists only in the resumed journal. TUI coverage compares the
pool control port against the new Agent port and retains the old port identity.
Typed authenticated session_changed and Android gated recovery are recorded
in progress-20261003-session-switch-refresh.md and rerun below.

## Verification

Commands ran from /Users/mac/Github/zenpi. Rust used only the controlled
ZENPI_HOME=/private/tmp/zenpi-resource-reload-test-home override.

- rtk cargo test --offline --locked --test tui_project_workspace
  --test headless_project_workspace --test remote_parity
  --test remote_subscription --test remote_subscription_tls
  --test remote_protocol --test remote_auth -- --test-threads=1
  --skip busy_diff_uses_selected_project_through_new_replay_switch_and_cancel:
  exit 0, 136 passed, one explicit filtered test, seven suites, 20.51 seconds.
- rtk cargo test --offline --locked --test headless_project_workspace
  local_resume_refreshes_session_scoped_input_and_approval_routes
  -- --test-threads=1: exit 0, one passed (both entry points), 0.36 seconds.
- rtk cargo test --offline --locked --lib existing_terminal_projects_only
  -- --test-threads=1: exit 0, one passed, 442 filtered, 0.62 seconds.
- Prior three-suite run finished with exit 101: headless busy-diff TCP fixture
  failed at tests/headless_project_workspace.rs:312, TcpListener::bind,
  PermissionDenied / Operation not permitted. Its child was reaped; retained
  stderr is /private/var/folders/gf/yj6lvtfj65922j9bnkfc7ddm0000gn/T/.tmpEetW2W/stderr.log.
  This failure is not counted as a pass. Only this known socket-dependent
  fixture was excluded from the successful rerun; no product gate was waived.
- Prior rtk cargo check --offline --locked handle finished with exit 0,
  zero errors and 20 warnings.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-desktop-switch-cache-final-20261003: exit 0,
  production/unit/instrumented Kotlin compilation and 297 JVM tests pass.
  SessionHostTest compiled only, not executed on a device. An initial cache
  fence compile failed on nullable cached projection accesses; explicit
  cached != null guard repaired it. Failed compilation is not a pass.
- Python session harness: six passed; APK provenance: ten passed; desktop
  harness: fourteen passed; headless harness: nineteen passed, all exit 0.
- cargo fmt --all -- --check; git diff --check; strict blueprint validator:
  exit 0, all 36 items still unfinished.
- python3 tools/stage2_android_build.py --verify-only: exit 1,
  missing APK build provenance. No stale APK/device/adb invocation followed.

All command handles needed by this work are terminal. One older source-check
handle was missing; its jar files alone were not treated as completion. The
fresh final source check supplies the current exit status. No permission
bypass, unchanged Gradle/ADB/TCP retry or invented device pass occurred.

## Digests

```text
tui.rs 3e06a01f1ef7a23e84dbe1483c26b185d8c2662fe7ac0cbe677a41efb564bc61
headless.rs aa42890b53128df99df4beb869922d1a07f4581ebdfee20c285bcd3deb99f44f
headless_project_workspace.rs 9f86b0a8edf6e65227572f61dcfb4efc0cb8800ed776d27f5c4bd5c596bd92ef
SessionHostTest.kt 6dbda5b75749b6225145524f9132704b8003ee838cc991317543ee4ce7b08bee
stage2_session_android_smoke.py 7814c3b9756f5fbbc8310f2376de3d03e583fb9ca0d2dd91fc77bdd361942ab7
test_stage2_session_android_smoke.py 0ddad03aaf458e77e3a42904646b5ff0ce6583efeeb3258e9fe28b484c805611
main.jar 47c82440c3ee67695a4d5b3668332923cc2eba6c7341b6cd0c4b4ebf2a6d2f55
unit.jar 21c5a06d549ece9f0ed6ea17138ef5bac2adb58d31933c3ea492737d47975b06
instrumented.jar d9b60301ab6bd955249478c9439072ee457a1653f4331290cedb783b76b54efe
```

Goal remains active. Current source-attributed APK, connected lifecycle/Compose,
physical camera/direct LAN, full parity and signed reproducible release remain
incomplete or unverified. JVM/UnixStream/fragmented memory TLS checks cannot
substitute for those requirements; no whole item is accepted here.
