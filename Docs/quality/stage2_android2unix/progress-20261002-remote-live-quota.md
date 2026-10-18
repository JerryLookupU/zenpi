# Remote Live Quota Routing

Provisional S2-018/S2-019/S2-035 progress observed 2026-10-02T09:39Z.
Previous goal turn made progress: host runtime quota settlement. HEAD remains
1a8ff1acf72e0816fad503a05df2b1d18e07361b, user worktree preserved. All 36
Blueprint items remain [ ]. No nested agents, execution-worker admission,
commit, push, cleanup or acceptance promotion.

## Implementation

WorktreeHost dispatch_quota routes an existing lane's SetConcurrency through
strict integer/field validation, current Read/Control grants, owner/session and
workspace/subtab CAS, idle master/arch and pending-change fencing. It holds the
registry revoke lock through reservation, ownership growth, scheduler submission
and immediate settlement. Unconfirmed application/commit returns null unknown,
not failed/no-effect or succeeded. Exact explicit retries poll only the original
runtime receipt and settle under the current device ACL/revoke lock; they never
reapply a restored disk intent. Changed content conflicts and another quota
request is busy. Worker event polling deliberately never settles quota without
the requesting device's lock. Revocation after authorized scheduling cannot
undo that effect but prevents remote commit; shutdown/local recovery remain.

HeadlessHost and ProjectRuntimeHost share this route; idle changes fall back to
the existing subtab mutation path. Committed changes reconcile the desktop view.
Pending quota prevents lane retirement even after tasks complete, preserving
its original receipt until explicit confirmation or shutdown. New task admission
is fenced while pending; cancel/approval remain independent. Original task
receipts survive revision advance. Android enables the existing confirmed-value
stepper during workers; immutable persisted commands and explicit retries are
unchanged. Unknown response retains pending state; no automatic quota retry or
displayed-count success inference added. Compose running-state assertion was
updated and compiled, not device-run.

Two new controller tests use held independent Agent/local jobs and bounded event
backpressure: unknown then exact success, malformed/busy/ACL refusal without
checkpoint change, reused-ID conflict, second request/new-task fencing, fresh
prompt under new revision, original prompt retry, durable receipt and revocation.
The second revokes after submission, verifies background polling cannot commit
and remote retry refuses, then performs explicit local recovery after shutdown.
The existing desktop worker integration test now submits live quota through the
actual desktop entry, confirms same-view count and byte-identical headless retry.
These are not LAN/provider HTTP or Android device acceptance tests.

## Verification And Interrupted Runs

Cwd /Users/mac/Github/zenpi; Rust ZENPI_HOME=/private/tmp/zenpi-remote-live-quota.
Completed checks:

- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  focused 5 passed, 415 filtered, 0.92 seconds, before final pending-lane retirement
  guard. Final direct-toolchain Cargo and direct final binary both pass all 5.
- /Users/mac/.rustup/toolchains/1.98.0-aarch64-apple-darwin/bin/cargo test
  --offline --locked --lib remote::worktree_host -- --test-threads=1:
  final exit 0, 5 passed, 415 filtered, 0.89 seconds.
- env ZENPI_HOME=... target/debug/deps/zenpi-39949e4c16357dce
  quota_intent_tests --test-threads=1: 11 passed, 409 filtered, 1.27 seconds.
- Same final binary remote::worker --test-threads=1: 7 passed, 413 filtered,
  0.35 seconds; remote::headless_host: 8 passed, 412 filtered, 0.74 seconds.
- env ZENPI_HOME=... target/debug/deps/remote_workspace-4290635085c1b3b1
  desktop_worker_route --test-threads=1: final 1 passed, 57 filtered, 0.28 seconds.
- Same final workspace binary remote_concurrency_is: 1 passed, 57 filtered,
  0.26 seconds; quota_crash_intent: 1 passed, 57 filtered, 0.30 seconds.
- /Users/mac/.rustup/toolchains/1.98.0-aarch64-apple-darwin/bin/cargo check
  --offline --locked: final exit 0, 20 existing warnings, 11.73 seconds.
- /bin/zsh tools/stage2_android_source_check.sh
  /private/tmp/zenpi-remote-live-quota-android: main/unit/instrumented compiled,
  234 JVM tests passed, 1.205 seconds. Not Gradle/APK/device evidence.
- /usr/bin/env PYTHONPATH=tools /usr/bin/python3 -m unittest
  tools.test_stage2_desktop_android_smoke tools.test_stage2_headless_android_smoke
  tools.test_stage2_scanner_android_smoke tools.test_validate_stage2_android_lifecycle
  tools.test_validate_stage2_android2unix_blueprint tools.test_validate_stage2_android_backup:
  final 63 passed, 0.097 seconds.
- Direct toolchain cargo fmt --all -- --check, /usr/bin/git diff --check and
  /usr/bin/python3 tools/validate_stage2_android2unix_blueprint.py --strict:
  final exit 0, 36 incomplete rows.

Broad multi-suite and several wrappers remained live without visible output.
Original handles were repeatedly polled, then explicitly interrupted (exit 130)
before any replacement run; no timeout alone was treated as terminal. Partial
RTK summaries (87 or 29 cases) are not counted as successful suite evidence.
The final direct Cargo workspace run was interrupted during the preexisting
post_git_checkpoint_conflict test after 23 earlier cases passed. It did not
finish all 58 tests; no full regression claim. Targeted final workspace cases
above were run separately after that process terminalized. Direct toolchain and
test-binary checks resolved observation issues for the critical changed paths.
All observed handles are terminal. No restricted TCP/Gradle/adb bypass.
Unique completed targeted Rust cases: 34; interrupted partial cases excluded.
Provisional evidence lacks acceptance-grade stdout/stderr-hashed receipts.

SHA256:

```text
src/remote/worktree_host.rs cbf34c8d8c6bab84704e12fa759e36757135ba7a1af8ed5fba01584df2eb5504
src/remote/worktree_runner.rs f9f67c44f883070bc7e11afbcf9e66bd2d29c3f70f359bef2852c0cc96c312af
src/remote/headless_host.rs 5ac46d2c0881955d7dc86f5ac0600e1bfa88d7682ed8a13f2c0153e0ef74a7f7
src/tui.rs 3585a951e4bceb1959bded3344cec16f874656a6b7f2617353808dd16a8b58e0
tests/remote_workspace.rs 17203237b9028886344ebaad7110e00c299e5449b1a1754c524021e64ba865c1
MainActivity.kt ff6335875177280bd81b0d3fc450494c3196feb2fa48daa5984d1c27321c5bbc
WorkerScreenTest.kt 04fce4d6a4153192c5124bf6ad6d798c26a376939b0b264ef8ebab3ddf1f5d1b
protocol 40c106e8c5d078def717a3627082d1f8980f23b2f930bb6c12a511f39af2078e
user-guide 30da4f5efe625d843b62e335eb6f14b54e00878151cfff512054778653f0d6c4
main.jar 190aa105034acc696349a8f71d9d42dbf4e0f0aaf4972ff31958e9f9ae271c26
unit.jar bc84ce8621ffd984f5a9a5a13096e44fd978b0da9be01c4c2b22507c8ff8ff4c
instrumented.jar 1c3f80d495e2a6f72d40beeebc8767ab5ba73338d2561d91bb616b5d596905ac
```

## Remaining Scope

Live remote routing now exists, but full N/queued/race/64-owner/device cross-entry
acceptance and complete final regression remain unproven. Effective wire status
does not distinguish retained capacity or live thread count. Local TUI worker
controls, full event/transcript parity, settings and interactive PTY remain
unfinished. Current APK API31/34/35 camera QR, actual LAN desktop sync,
disconnect/replay/process lifecycle and reproducible signed release remain
unverified. Full objective stays active, no Blueprint item accepted.
