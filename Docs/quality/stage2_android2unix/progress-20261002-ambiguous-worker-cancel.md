# Phone Cancellation With Ambiguous Progress

Provisional S2-019/S2-020/S2-036 progress at 2026-10-02T11:50Z.
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b. Current worktree and skill
inspected; previous turn made progress (desktop status). User changes preserved;
no nested agents, execution-worker admission, commit, push, cleanup or acceptance.

## Implementation

Progress append failure already retained actual runtime ownership while showing
unknown_outcome, but phone cancellation routing and UI required nonterminal
progress. Such a task could keep running with no phone cancellation control.
Live worker wire status now optionally includes runtime_active=true derived
from tracked Pending.runtime_done, never from stored events. This flag is
removed after runtime completion and absent in durable/restarted projections.

Host target lookup and cancellation candidates now use runtime completion,
not the ambiguous persistence outcome. The original device/request/session/
runner generation still binds the job, and normal Control/revoke/owner/revision
checks and durable cancel receipts remain unchanged. No prompt resubmission.

Android validates this optional boolean strictly: true only with accepted,
running or unknown_outcome; false/string/number/terminal flags are refused.
RemoteWorkerStatus.canCancel centralizes command, error-state dispatch and UI
gating. Unknown plus current runtime evidence shows an active indicator and
cancel action; unknown alone does not. Cache and authority loss continue dropping
all worker statuses, including the new evidence. Offline controls remain disabled.

## Verification

Cwd /Users/mac/Github/zenpi; ZENPI_HOME=/private/tmp/zenpi-ambiguous-worker-cancel.
Completed exit-0 commands:

- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  final 14 passed, 425 filtered, 6.33 seconds. New controlled live job replaces
  the master journal path to force real append refusal while preserving its inode.
  Unknown status retains runtime_active; foreign phone cannot read/cancel it;
  restored original path permits original-phone cancel and exact receipt retry.
  Runtime exit retires owners, removes the flag and prevents targeting the job.
  A forged runtime_active field in durable progress is never projected as live.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  ten passed, 429 filtered, 0.48 seconds. Ambiguous-progress fixture confirms
  the flag before runtime terminal and its absence afterward.
- rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test core_session --test runtime --test remote_parity
  -- --test-threads=1: 143 passed, six suites, 33.65 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text --test headless_project_workspace
  --test tui_project_workspace --test tui_composer --test remote_concurrency --test remote_migration
  --test remote_subtab --test remote_worktree -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel --test-threads=1:
  168 passed, three restricted TCP fixtures excluded, eleven suites, 17.39 seconds.
- /bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-ambiguous-worker-cancel:
  production/unit/instrumented cached API35 source compilation passes; 250 JVM
  tests pass, 1.219 seconds. New JVM test covers strict flags, unknown cancel
  command/error-state policy, inactive refusal and offline/authority clearing.
  New 320 dp Compose cancel/indicator/offline/removal test compiled, not device-run.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed, 0.061 seconds.
- cargo fmt --all --check; git diff --check; python3
  tools/validate_stage2_android2unix_blueprint.py --strict pass; all 36 unfinished.

No restricted TCP/Gradle/adb retry or bypass. Tests are controlled runtime and
production-domain fixtures, not LAN/provider HTTP/camera acceptance. All observed
command handles terminal; no timeout caused duplicate execution. Provisional
evidence is not an acceptance receipt with stdout/stderr hashes.

SHA256:

```text
src/remote/worker_progress.rs 097a4bed376aa7e8021f83ce0ca08c9da2ba586489d17e349424d5baf4d453b0
src/remote/worktree_host.rs 54bed69f435c00d05f52c3576ccaf65b2035920b1b7b56619e884ea329063294
android/app/src/main/java/zenpi/transport/WorkerStatusCodec.kt d9029e2ac86e4742d0069e9ea2af927e21be49be0f8ce37488e3b32965a42b31
android/app/src/main/java/zenpi/transport/HostClient.kt c9d2d1898e5ceb70bf3cfbf53a66963130226c5de59b33a7ece53dfb99d246b1
android/app/src/main/java/zenpi/pairing/WorkspaceDispatchPolicy.kt f681347c6deadf2fd2302db4959280914f490915501c2f44eb447be35c4ecc5a
android/app/src/main/java/zenpi/MainActivity.kt 5a49d85d72c8e57cd6cced6e0aa9a1de5d4592dc35dd6244429109e49dabbfce
android/app/src/test/java/zenpi/WorkerControlTest.kt 7a30d75b805b10cc41dc84c3a8cc4c65ff14c95d1ea9b305da33942d63dbb4bb
android/app/src/androidTest/java/zenpi/WorkerScreenTest.kt 386052206a1f6cd49bd84f3779c5fbd7e3f5b0c0eefc7efd72e991618176f4c6
main.jar 147f0f70dd5b48885e5d654f533312f4da9abed7d0bdac9e23ff4059948dc8ba
unit.jar 5ca33e3b3683e1aa89ff7533f5ad531092a8cfe85591f8ca03e5b61e8c0da511
instrumented.jar 9ab093d5224831dd5ce267020c02b201db4ead3c668e0bf72382b0d80254b63f
```

Artifacts under /private/tmp/zenpi-ambiguous-worker-cancel. Real API31/34/35
camera QR/LAN desktop sync, disconnect/replay/address changes, process lifecycle,
signed APK/reproducible release, local task submission/quota UX, live/failed
partial history, omitted call/status paging and interactive PTY remain unverified
or unfinished. Full goal remains active, no Blueprint item promoted.
