# Idle Worktree Quota Control

Provisional S2-018/S2-019/S2-035 progress observed 2026-10-02T08:52Z.
Previous goal work made progress: worker live text and quota implementation;
the intervening RTK check confirmed the installed tool. Current worktree is
authoritative. HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b remains unchanged;
user changes preserved. All 36 Blueprint rows remain [ ]. No agent admission,
nested agents, commit, push, cleanup or acceptance promotion.

## Implementation And Scope

SetConcurrency now traverses the desktop owner mailbox and shared subtab
mutation path. Exactly subtab_id/subtab_revision/concurrency is accepted, with
integer quota 1..64, current Read/Control grants and owner/session/workspace/
subtab revisions. The atomic checkpoint and exact device/request fingerprint
receipt ledger retain configuration and retry identity. A succeeded six-field
receipt means idle configuration committed, not live scheduler application.
Master/arch pending/running/preparing work, runtime-owned independent worker
slots and desktop local admission fence writes. Actual next WorktreeRunner
startup leases and schedules the committed quota. No live resizing added.

Android strict projection decoding now accepts the grant only under live
workspace authority. Command construction/restoration validates exact fields
and integer bounds; receipt validation checks workspace/subtab increments and
owner/project identity. The existing durable pending-command path persists
before send and permits explicit exact-envelope retries. A selected-tab stepper
uses confirmed host values, not optimistic increments, and disables for
pending/revocation, disconnected phases or visible active workers. Cache v3
retains display counts but discards all grants. Another device's unseen worker
is still covered by the authoritative host busy guard.

The new Rust quota test proves durable CAS/ACL/malformed-input refusal, exact
retry/reused-ID conflict, no N+1 start at quota 3, busy checkpoint preservation,
completion then successful quota change, restart and old receipt without
rollback, mailbox routing and revoked retry refusal. The desktop worker route
test now changes quota through the actual ProjectRuntimeHost, checks the view
count, executes a separate Agent turn and queries the same quota receipt after
headless restart. This uses local Echo/provider events, not a LAN/provider HTTP
fixture. Three new JVM cases cover command/receipt/cache/foreground/retry
boundaries; an additional Compose case is compiled, not device-executed.

## Verification

Cwd /Users/mac/Github/zenpi. Rust commands use
ZENPI_HOME=/private/tmp/zenpi-quota-final unless noted. Final commands exit 0:

- rtk cargo test --offline --locked --test remote_workspace
  --test remote_worktree_runner --test remote_worker_owner --test remote_concurrency
  --test remote_parity -- --test-threads=1: 94 passed, five suites, 25.15 seconds.
- rtk cargo test --offline --locked --test remote_workspace -- --test-threads=1:
  final 57 passed, 19.62 seconds, including added mailbox/capability/view assertions.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  6 passed, 400 filtered, 0.25 seconds.
- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  3 passed, 403 filtered, 0.46 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host -- --test-threads=1:
  8 passed, 398 filtered, 0.70 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands --test remote_auth
  --test remote_approval_wakeup --test remote_live_text -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit --skip
  production_plain_host_routes_authenticated_tls_prompt_to_the_provider
  --test-threads=1: 31 passed, two TCP fixtures excluded, 3.56 seconds.
- rtk cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: 103 passed, one TCP fixture excluded, 6.08 seconds.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- /bin/zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-quota-control-android-final: main/unit/instrumented source
  compiled; 234 JVM tests passed, 1.193 seconds. Not Gradle/APK/device evidence.
- env PYTHONPATH=tools python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 passed.
- cargo fmt --all -- --check, git diff --check and
  python3 tools/validate_stage2_android2unix_blueprint.py --strict: pass;
  strict checker reports 36 incomplete items.

Unique successful Rust cases: 245; focused/repeated cases not added again.
After the initial 94-case run, an added mailbox assertion first failed compile
because the fixture registry was not Arc-owned. Unfiltered Cargo diagnosed
E0308; the fixture was corrected and all final 57 workspace cases passed.
All observed sessions terminal. Process inventory was denied; no inference
of process liveness was made from it, and no observation timeout restarted work.
No restricted TCP/Gradle/adb bypass attempted. These are provisional progress
results, not acceptance-grade stdout/stderr-hashed receipts.

SHA256:

```text
blueprint d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97
specification 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28
src/remote/subtab.rs 481ed7433c9c83b8888fdcf0586bf731ed0303ba362841bddfca4040ccb1af32
src/remote/host_adapter.rs f1e588caf47d0e2f4e07bbcfe324328725faa883162d6e3f6426b95f2a60fade
src/tui.rs 9001cc9db2ccb38eefe1b1ef1fd2066e3da1e94315ca92527b0e20eb2a9c5029
tests/remote_workspace.rs 5697aec23d932d63e8cf7c7262cd5a79e7afa683c55a92c086aad92e18d6ec3a
SubtabProjection.kt 7496ac303bce3618a39aa23cd8bf877be89b923fd3a6df54a104edf4620c177c
HostClient.kt d4697eae6e58161960e74a7c347e930556e82d37eeb66a9e1fe13ebf932f7c98
WorkspaceReceiptCodec.kt 093312756accafe78372ac83e8dd911afddb302348fc4ebb11f581fd3105838f
PairingViewModel.kt 268acb439551a182fdcd5bec75da9b668e6e0825f1c76de0fb7748be8d2befa0
MainActivity.kt 2ee08fcbe3451b6c1d554be2a07e81cd7cfdf2110d05e94d3bbde63a796cf794
ConcurrencyControlTest.kt 5921975cf241476ad416c4c686071b05728e3ec15fbd4cd19fdf6c0208fd1b91
WorkerScreenTest.kt 2f3a0338f1c86ae363241bb0aa253b92ed803d7a512289b34080f0db7bb67ea0
protocol 5b64e5428d1492bf7b1cc4fb11490e6fb9019f9c02047245685167eedbe1649f
main.jar e6fb40c5b6bacb8a4b96345326c5866143107206c08db98db1f7fac91e6bc5aa
unit.jar bc84ce8621ffd984f5a9a5a13096e44fd978b0da9be01c4c2b22507c8ff8ff4c
instrumented.jar c11159d20af9dbd128f4f26f6450fe5e74a6c1604dfa553a09073416f5c5e718
```

## Remaining Scope

Live quota resizing must reconcile independently leased owners with scheduler
application and effective receipts, including lowering while running/queued.
Local TUI worker task/status/approval controls, complete event/transcript parity,
settings and interactive PTY remain unfinished. Current APK API31/34/35 camera
QR, actual LAN desktop sync, disconnect/replay/process lifecycle and reproducible
signed release remain unverified. Full objective stays active; idle controls
do not replace any of these requirements or justify accepting S2-018.
