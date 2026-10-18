# Worker Approval Shared Route And Android Client

Provisional S2-018/S2-019/S2-036/S2-037 progress observed 2026-10-02T07:54Z.
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b, dirty worktree preserved.
Previous goal turn yielded verification evidence (RTK, Cargo check and completed
workspace test handles); this turn changes the authoritative source. All 36
Blueprint rows remain [ ]; full objective active. No controller worker
admission, nested agents, commit, push, cleanup or acceptance promotion.

## Implementation

The preceding host-domain worker approval implementation retains originating
master-session identity, projects redacted summaries without the worker Agent
mutex, and answers an exact worker-session/digest target through the existing
ApprovalCoordinator. Unknown checkpoint reservation precedes the response;
durable receipts survive restart and refuse revoked devices. Master approval
arguments and behavior remain unchanged.

This turn connects that behavior to the actual shared HostAdapter. Workspace
listing includes a combined master/worker projection capped at 16 per project.
Approve routes by worker_session_id presence; malformed/null worker targets do
not fall back to the master coordinator. The existing desktop remote route and
HeadlessHost mailbox use this adapter. Worker runtime creation/admission is
still not connected to those production controllers.

Android strictly accepts a master summary or the exact worker extension with
both worker_session_id and stable subtab_id. Duplicate request strings are
allowed across independent worker sessions but rejected within one session.
Command creation, saved request restore and explicit pending retries retain
the chosen worker session. Error-phase authority compares it as well as ID
and digest. Offline cache still discards all approval targets/authority.
Cards, confirmation and accessible decision controls identify the worker;
master control descriptions are unchanged. Correlated receipts retain the
existing six-field master-session routing rather than becoming execution
proof or silently changing the protocol receipt schema.

New real HeadlessHost mailbox integration uses two retained worker owners with
the same approval ID. It checks listing, read-only visibility, foreign-project
absence, raw-argument exclusion, invalid/null/master-target refusal without
checkpoint changes, Control-versus-Approval isolation, responding while the
worker Agent mutex is held, exact retry, other-worker isolation, changed-target
request conflict and revoked receipt denial. Coordinators use deterministic
local fixtures, not an actual provider or network socket. Earlier independent
worker tests cover allow/deny, stale origin/session replacement, failed CAS,
restart receipts, unsafe extras and actor worker-session audit.

Four new JVM tests cover worker scoped identity, strict partial/invalid fields,
saved command restoration, error authority and correlated receipt compatibility.
One new Compose instrumentation test covers distinct duplicate-ID buttons and
disabled confirmation after worker-session replacement. It was compiled only,
not executed on a device.

## Verification

Cwd /Users/mac/Github/zenpi. Rust environment
ZENPI_HOME=/private/tmp/zenpi-worker-mobile-route-final. Final commands exit 0:

- rtk cargo test --offline --locked --test remote_workspace
  --test remote_worker_owner --test remote_worktree_runner
  --test remote_approval_wakeup --test remote_auth --test remote_parity
  -- --test-threads=1: 91 tests, six suites, 24.49 seconds after final refusal
  coverage. An earlier 91-test run passed before the added refusal assertions.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit
  --skip production_plain_host_routes_authenticated_tls_prompt_to_the_provider
  --test-threads=1: 8 passed, two restricted TCP fixtures excluded, 2.77 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  4 passed, 397 filtered, 0.24 seconds.
- cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: 103 passed (19/28/56), one restricted TCP fixture excluded.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-worker-mobile-route-android-ui-final: main/unit/instrumented
  source compiled; 223 JVM tests passed, 1.149 seconds. No Gradle/APK/device run.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  63 passed. cargo fmt --all -- --check, git diff --check and strict Blueprint
  validator pass. Validator still reports 36 incomplete items.

Unique Rust tests in the listed suites: 206. Repeated runs are not added.
The first new mailbox test failed because its fixture omitted required request
ID; supplying the proper envelope fixed it, without relaxing host validation.
Final related regression suites pass. Every live test handle was observed to
completion; observation timeouts did not cause automatic restart.

Source SHA256 at verification:

```text
src/remote/approval.rs 67911af7b7a47c534432aa5ace27d6cd5a52cf61b98548787ea7de488548c4ae
src/remote/host_adapter.rs aab416cc7580cf01e7357ebe3c24342695c3814ec1e7001fb5e357ab0aa6bee9
src/remote/worker.rs 0ea3c0f5f4aca9be8a0d219f4089c9fc2b8a833009bffaeb7624478d09aaedce
tests/remote_workspace.rs 5a1ff18a820a4d2d2dcafba3be3a1b7d9843159326b0658b6d0a520529a4e1f0
android/app/src/main/java/zenpi/transport/ApprovalProjection.kt b9aae4ad0288d771c0996fd042eb9995bfc381df3ec866cd699802dc0bf453a7
android/app/src/main/java/zenpi/transport/HostClient.kt 235bf6becdb1c6d7141a79c5e0a5c9e1347cb4281ed93fd16571b53c98e736ce
android/app/src/main/java/zenpi/pairing/WorkspaceDispatchPolicy.kt 3a44f10a3645952eb3cfa06734ace49ea5ed4f56ad3b946075556bf197aa21e5
android/app/src/main/java/zenpi/MainActivity.kt 28bb4573bea6358d12226497e3756b08d725092dc2b7152f5e25c7f956360e24
android/app/src/androidTest/java/zenpi/ApprovalScreenTest.kt 6567af910bfe1a2a910fbfadebc4f0db6081156f7504db65c21c9f80dcfadf33
Docs/stage2_android2unix_protocol.md 0f3694da6cef306118d7a495e26a3e144ea2aed31ecc6c349bb0a1e63d550556
Docs/stage2_android2unix_blueprint.md d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
Docs/stage2_android2unix_spec.md 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
main.jar 2e320daabd864afe51dae9932a213b0411dcfd789921423f463fd54408cae15b
unit.jar fdd9dc3404e7da0d8ecf8b43be400b8d0691e2176f3dd91e0fb4747985d185cd
instrumented.jar 58ae7a8ee621a40239bf849d9accaca2611d3f735fe561d888f2fe33b47fc39e
```

## Remaining Scope

Production worker admission/progress/cancel and lifecycle ownership, transcript/
live-event routing, effective live quotas and Android concurrency controls are
unfinished. No worker controller was started merely to make approval tests pass.
Current APK/API31/34/35 camera QR, real same-LAN desktop communication,
disconnect/replay/lifecycle, full settings/interactive PTY parity and signed
reproducible release remain incomplete or unverified. Restricted TCP/Gradle/adb
constraints were not bypassed. This provisional note lacks acceptance-grade
captured stdout/stderr receipts and is not G-HOST/G-NETWORK/G-ANDROID acceptance.
The full user objective remains active.
