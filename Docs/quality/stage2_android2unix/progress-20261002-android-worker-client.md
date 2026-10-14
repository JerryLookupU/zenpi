# Android Worktree Task Client

Provisional S2-018/S2-019/S2-035/S2-036/S2-038 progress, 2026-10-02T08:20Z.
Previous goal turn made progress: actual headless worktree lane integration.
HEAD 1a8ff1acf72e0816fad503a05df2b1d18e07361b; dirty worktree preserved.
All 36 Blueprint rows remain [ ]; full objective active. No nested agents,
controller worker admission, commit, push, cleanup or acceptance promotion.

## Implementation

HeadlessHost lists worker_control only for a current project Control grant and
durable workspace checkpoint. Its worker_statuses remain device-scoped and
separate from master prompt statuses. Android defaults absent fields to disabled/
empty, so desktop adapters without the worker scheduler cannot expose controls.

The new bounded WorkerStatusCodec requires exact request/project/master-session/
stable-subtab/state fields and optionally an allowlisted typed failure code.
It reuses the existing prompt state validator, rejects duplicate IDs, raw
diagnostics/arguments/stream fields and more than 16 statuses. RemoteProject
now carries live worker authority and statuses independently from master
prompt authority. Subscription view reduction consumes these fields without
inventing master transcript turns or advancing cursors; malformed updates are
atomic refusals.

WorkspaceCommand.workerPrompt captures the selected host subtab ID and lexical
u64 revision in an exact Prompt body. Saved command restore validates shape,
identity, limits, blank/shell/NUL input and numeric revision. Admission receipts
must match the requested subtab in addition to the original request, job and
master project/session. Accepted is admission, never execution success.
cancelWorker requires retained live worker authority and an accepted/running
device-scoped target; it sends the same one-target cancellation body already
routed by the headless controller. ERROR phase can cancel a retained live task
but cannot submit new work. Offline unknown decisions can only be explicitly
retried with the identical saved envelope; foreground/network/reset/revoke
fences and existing durable pending command storage remain shared.

The main Compose screen shows worker task state and a Stop control separately
from master messages/steering. A dedicated worktree composer identifies the
selected tab and sends through PairingViewModel's existing owner callback,
receipt and persistent retry path. Draft identity includes project/session/tab
so switching the target does not reuse another task draft. This is not worker
transcript or provider live-stream parity.

Cache v3 deliberately remains unchanged: encoding discards worker authority
and statuses, and decoding rejects added authority fields. AuthorityLossPolicy
already uses this display-only conversion. No offline snapshot can start or
cancel a worker. Existing master prompt, approval and cache behavior is retained.

Five JVM tests cover decoding/capacity/unsafe scope, exact selected command and
restore, cache/authority loss, worker cancellation/foreground/error/pending
retry, and receipt identity/state. A sixth subscription test checks unchanged
cursor/transcript and atomic malformed-worker refusal. One new Compose test
checks narrow task send/cancel controls, master separation, error-phase send
disable and offline/authority loss. It was compiled only, not run on a device.
The actual Rust mailbox regression also checks Control true versus read-only
false and absence of another device's worker statuses.

## Verification

Cwd /Users/mac/Github/zenpi. Rust uses
ZENPI_HOME=/private/tmp/zenpi-android-worker-client. Final commands exit 0:

- rtk cargo test --offline --locked --test remote_workspace
  -- --test-threads=1: 54 passed, 19.91 seconds.
- rtk cargo test --offline --locked --test remote_worker_owner
  --test remote_worktree_runner --test remote_concurrency --test remote_parity
  --test remote_auth --test remote_approval_wakeup -- --test-threads=1:
  50 passed, six suites, 6.13 seconds.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  5 passed, 398 filtered, 0.28 seconds.
- rtk cargo test --offline --locked --lib remote::worktree_host
  -- --test-threads=1: 1 passed, 402 filtered, 0.45 seconds.
- rtk cargo test --offline --locked --lib remote::headless_host
  -- --test-threads=1: 8 passed, 395 filtered, 0.92 seconds.
- rtk cargo test --offline --locked --test remote_headless_commands -- --skip
  headless_runner_executes_real_http_tool_turn_and_retries_never_resubmit
  --skip production_plain_host_routes_authenticated_tls_prompt_to_the_provider
  --test-threads=1: 8 passed, 2 restricted TCP fixtures excluded, 2.57 seconds.
- cargo test --offline --locked --test headless_project_workspace
  --test tui_project_workspace --test tui_composer -- --skip
  busy_diff_uses_selected_project_through_new_replay_switch_and_cancel
  --test-threads=1: 103 passed (19/28/56), one restricted TCP fixture excluded.
- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-android-worker-client-subscription-final: main/unit/
  instrumented source compiled, 229 JVM tests passed in 1.162 seconds.
  Earlier 223/228 test runs also passed, not added to the final count.
- env PYTHONPATH=tools python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  63 passed. cargo fmt --all -- --check, git diff --check and strict Blueprint
  validation pass; still 36 incomplete items. Unique successful Rust cases: 229.

No fresh APK, Gradle socket, device UI, adb or physical TCP/LAN validation was
attempted or claimed. Cached source compilation is weaker than G-ANDROID and
G-RELEASE. All live test handles were polled to terminal before final reporting.
No observation timeout caused automatic restart.

SHA256 at verification:

```text
src/remote/headless_host.rs 5f4f3c4ccd5ead928a3594d0ad1729596a8be1f5ab11e3c62dc5fb76c281bc9e
tests/remote_workspace.rs bc6de9806cc06b00b6506ea64b2e8e8332e923b027d0af8eff136f4b3e4bf498
android/app/src/main/java/zenpi/transport/WorkerStatusCodec.kt 5cde1f7731e363517f781938a9b52a7fae08d712f4398b128eafc341642deb21
android/app/src/main/java/zenpi/transport/HostClient.kt 65e0e23ac74fba000409d01890580c8fdadb821c6a315f14b89a7775fe333ece
android/app/src/main/java/zenpi/transport/WorkspaceReceiptCodec.kt 56a944f0524317d1fdce971835861ffeeb7a241cc58b1d43cf414707016b51e7
android/app/src/main/java/zenpi/pairing/WorkspaceDispatchPolicy.kt 592ebc4cc8605d76905c7e93a2d95c7ca389d7e2b742df1da8c79ece5fd52c6d
android/app/src/main/java/zenpi/pairing/PairingViewModel.kt 51182f91cffa0c2c119f2f1c1331d3f4e14191b5082efc7512614b7f34f12df6
android/app/src/main/java/zenpi/state/ProjectionCacheCodec.kt a1a4c1dfecda86f2308255b8116f4771ffe19ca780a67579647f05db388417fc
android/app/src/main/java/zenpi/MainActivity.kt 30e84763ff0d22a2b22e675ba7cae8d421c78806c4c299c9a48201f365c1ff66
android/app/src/test/java/zenpi/WorkerControlTest.kt af3cc1c78a96dcace0ef3616268f9f13855ffc15431dd6725d0fb24a0edc7fa1
android/app/src/test/java/zenpi/SubscriptionTest.kt 1931a7d72bcf46fbb88155a9e61867eacd37d48d02519841f95e8de4c3362c05
android/app/src/androidTest/java/zenpi/WorkerScreenTest.kt 8d68f80929eed186f35e9d8cafa698fcdc3562b0f853174518f1014e6bf33b02
Docs/stage2_android2unix_protocol.md 1eb554ae21f838b4a3b54dc6422f9539368386de1637aa561079a02bc874427b
main.jar dc13586089fccfe4758558cda53067afbfbc587924c2907174850d849009c664
unit.jar 8b6c5f5976a09ef19506dc8b19046893356cc5fe23aa57e3102aff783c7b603e
instrumented.jar 4592046dbcd17954315c160ae5ed1b61e0e6e348fd9a43aa19c1244c660644b3
```

## Remaining Scope

Desktop TUI worker scheduler parity, live effective quota receipt/stepper,
worker transcript/provider stream, full settings and interactive PTY remain
unfinished. Current APK API31/34/35 camera QR, actual same-LAN desktop sync,
disconnect/replay/process lifecycle and reproducible signed release are still
unverified for this state. This provisional note is not an acceptance receipt
with captured stdout/stderr hashes. Full objective remains active.
