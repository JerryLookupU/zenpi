# Paged Worker Status Routing

Provisional S2-019/S2-020/S2-036 progress at 2026-10-02T12:20Z.
The current dirty worktree, authoritative blueprint and execution skill were
inspected. Previous turn made progress (status-page codec/UI). No nested agents,
worker admission, commit/push/cleanup or checklist promotion.

## Implementation

The paged `workspace_list` view=`worker_status` route is now wired through the
Rust TUI owner-thread and headless host paths to the same WorktreeHost lanes and
master journal. It validates authenticated device/project/session scope, exact
`view`/offset/digest arguments, current master ownership and bounded task data.
The route merges live tracked and durable statuses, keeps runtime-active entries
first, deduplicates request IDs, removes output/stream fields and returns a
digest-fenced page of 16 stable status records. State changes invalidate a
continuation with revision conflict; stale pages cannot be concatenated.

Android calls this route through HostClient, decodes exact page fields and
strictly validates integer bounds, page continuity, digest/total, task count,
scope and absence of output fields. A memory-only task list dialog exposes
refresh, previous and next page controls. Opening a new page cancels the prior
generation; scope, authority, lifecycle, offline and pending-mutation fences
clear or disable it. Cached projection data does not retain task pages.

## Verification

Cwd `/Users/mac/Github/zenpi`; `ZENPI_HOME=/private/tmp/zenpi-status-pages`.
Completed commands exit 0:

- `rtk cargo check --offline --locked`: zero errors, 20 existing warnings.
- `rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1`:
  11 passed, 430 filtered, 0.84 seconds.
- `rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1`:
  15 passed, 426 filtered, 6.46 seconds.
- `/bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-status-pages-final5`:
  cached API35 production/unit/instrumented compilation; 251 JVM tests passed,
  1.153 seconds. New page codec test covers digest continuity, exact offsets,
  scope, output exclusion and mixed snapshot rejection. Compose task list UI
  compiled but was not run on a physical/emulated device.
- Python Stage 2 suite: 63 passed, 0.066 seconds.
- `cargo fmt --all --check`, `git diff --check` and strict Blueprint validator:
  pass; all 36 checklist rows remain unfinished.

No TCP/Gradle/adb bypass or real LAN/camera/provider HTTP claim. All observed
handles terminal. Evidence remains provisional rather than acceptance-grade
stdout/stderr-hash receipts.

## Remaining Scope

Real API31/34/35 QR scan-to-LAN desktop synchronization, disconnect/replay and
address changes, process lifecycle, signed APK/reproducible release, and device
execution remain unverified. Full local worker quota UX, omitted task/status
fields beyond the bounded page contract and interactive PTY remain unfinished.
