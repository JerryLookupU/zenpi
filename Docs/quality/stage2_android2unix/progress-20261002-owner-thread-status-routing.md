# Owner-Thread Paged Status Routing

Provisional S2-019/S2-020/S2-036 progress at 2026-10-02T12:20:35Z.
Current worktree, authoritative blueprint and execution skill inspected. Previous
turn made progress (paged status route). No nested agents, execution-worker
admission, commit/push/cleanup or checklist promotion.

## Implementation

The owner-thread admission allowlist and desktop TUI mailbox now route the
`workspace_list` view=`worker_status` request to `WorktreeHost::status_page`,
instead of rejecting it or falling back to the compact 16-entry listing. The
headless production adapter already uses the same route. This closes the
desktop production-path gap between Android's paged client and host lanes.

The response remains authenticated, current-session scoped and digest-fenced.
It merges live tracked and durable status, excludes output/stream/prompt data,
caps the snapshot at 4096 and pages 16 stable records. A stale digest, changed
task count, wrong offset, busy owner or unauthorized device fails closed.

## Verification

Cwd `/Users/mac/Github/zenpi`; `ZENPI_HOME=/private/tmp/zenpi-status-pages`.
Completed commands exit 0:

- `rtk cargo check --offline --locked`: zero errors, 20 existing warnings.
- `rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1`:
  15 passed, 426 filtered, 6.97 seconds.
- `rtk cargo test --offline --locked --test remote_workspace --test remote_worktree_runner
  --test remote_worker_owner --test core_session --test runtime --test remote_parity
  -- --test-threads=1`: 143 passed, six suites, 35.66 seconds. Production
  desktop owner-thread fixture remains green; paged route is included in compile
  and dispatch path.
- `/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-status-pages-final5`:
  251 JVM tests passed in the prior final source run; paged codec/UI compiled,
  not device-run.
- Python Stage 2 suite: 63 passed; strict Blueprint validation, cargo format and
  git diff checks pass; all 36 checklist rows remain unfinished.

No restricted TCP/Gradle/adb bypass, device/LAN/provider HTTP claim or duplicate
process restart. Evidence is provisional, not an acceptance receipt with captured
stdout/stderr hashes.

SHA256:

```text
src/remote/worktree_host.rs dd44a79ee70522fec893fb13ac0b8f05811f22ec328f4499126ce899f5ffe49b
src/remote/host_adapter.rs 88acd6a33b8f2d627adc35e090340630796d696d94c8ae2f5a0dc2fa310430cb
src/remote/headless_host.rs 30a86f800a949490dc8189496c64f128ba6cbff6ed0bf980b954447deca7ee00
src/tui.rs 350da76bde385a6a676c1dccae75834a0a90f28b34dcf9b906c4e5da1ac68ab6
android/app/src/main/java/zenpi/transport/WorkerStatusPageCodec.kt 96c96dccb429956ed8bd3da9019768755e1f232fb7e72aaba7417c01278ed688
android/app/src/main/java/zenpi/pairing/PairingViewModel.kt d9f92a8d8d043d11e7378739a30e3189f7dc1e87fd88c6013eb4fc80ea75329f
android/app/src/main/java/zenpi/MainActivity.kt 13c20710419df10d01176f44d8531cfee9337a9ebd98e9b15eab94e526063d9b
android/app/src/test/java/zenpi/WorkerControlTest.kt 054e32d249f12b19164a31b6125152db2ac6bc9f2b35133ea6db9a269d154b86
```

Real API31/34/35 QR scan-to-LAN synchronization, disconnect/replay/address-change
behavior, process lifecycle, signed release and physical/emulated device execution
remain unverified. Full goal active.
