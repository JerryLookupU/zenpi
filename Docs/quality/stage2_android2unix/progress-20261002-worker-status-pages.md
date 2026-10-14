# Paged Phone Worktree Status

Provisional S2-019/S2-020/S2-036 progress at 2026-10-02T12:10Z.
Current dirty worktree, blueprint and execution skill inspected. Previous turn
made progress (live-task prioritization). User changes preserved; no nested
agents, execution-worker admission, commit/push/cleanup or checklist promotion.

## Implementation

Phone worktree status now has an explicit `workspace_list` view=worker_status
route. The host builds a stable, bounded task projection for the authenticated
device and current project/session, including only request/project/session/
subtab/state/failure/runtime-active fields. It removes output and stream data,
deduplicates tracked and durable records, prioritizes live evidence, and caps
the total snapshot at 4096 tasks. Pages contain 16 tasks.

The full snapshot digest covers the stable task projection. Requests bind the
previous page's digest and exact next offset; changed task state, project,
session, total count, digest, malformed offsets or extra fields fail closed.
The endpoint reads the master journal and live lanes without mutation or worker
reconstruction. Existing 16-entry workspace listing remains unchanged for
compact projection; the explicit paged view is required for full browsing.

Android adds `WorkerStatusPageCodec`, a memory-only task list dialog with refresh,
previous and next controls, digest/offset continuity and scope checks. The list
shows IDs, state and runtime-active marker only; output remains in the normal
status projection. Opening a new list or scope loss cancels the prior request;
offline, pending mutation, authority loss and lifecycle fences disable or clear
the dialog. No status page is persisted in the projection cache.

## Verification

Cwd /Users/mac/Github/zenpi; ZENPI_HOME=/private/tmp/zenpi-status-pages.
Completed commands exit 0:

- rtk cargo check --offline --locked: zero errors, 20 existing warnings.
- rtk cargo test --offline --locked --lib remote::worker -- --test-threads=1:
  11 passed, 430 filtered, 0.91 seconds. Bounded live-priority and admission
  status fixtures remain green.
- rtk cargo test --offline --locked --lib remote::worktree_host -- --test-threads=1:
  15 passed, 426 filtered, 6.51 seconds. Cross-lane priority and current
  device/session behavior remain green.
- /bin/zsh tools/stage2_android_source_check.sh /private/tmp/zenpi-status-pages-final5:
  cached API35 production/unit/instrumented compilation passes; 251 JVM tests
  pass, 1.153 seconds. New page codec test covers digest, exact continuation,
  scope, bounds, output exclusion and mixed-snapshot refusal. Compose task-list
  UI compiles but is not device-run.
- env PYTHONPATH=tools /usr/bin/python3 -m unittest tools.test_stage2_desktop_android_smoke
  tools.test_stage2_headless_android_smoke tools.test_stage2_scanner_android_smoke
  tools.test_validate_stage2_android_lifecycle tools.test_validate_stage2_android2unix_blueprint
  tools.test_validate_stage2_android_backup: 63 pass, 0.074 seconds.
- cargo fmt --all --check; git diff --check; strict blueprint validator pass;
  all 36 checklist rows remain unfinished.

No TCP/Gradle/adb bypass or device/LAN/provider HTTP claim. All observed command
handles terminal. Provisional evidence is not an acceptance receipt with captured
stdout/stderr hashes.

Artifacts: `/private/tmp/zenpi-status-pages-final5`.

## Remaining Scope

Real API31/34/35 QR camera-to-LAN desktop synchronization, disconnect/replay and
address changes, process lifecycle, signed APK/reproducible release, full local
worker submission/quota UX and interactive PTY remain unverified or unfinished.
The full goal remains active.
