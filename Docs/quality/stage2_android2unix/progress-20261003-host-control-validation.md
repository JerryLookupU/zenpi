# Host Control, PTY And Worker Validation

Provisional S2-015/S2-016/S2-017/S2-018/S2-019/S2-036/S2-054 progress on
2026-10-03. Previous turn repaired terminal/subtab coherence and produced JVM
evidence, so it was progress. Current worktree, execution skill, test bodies and
parity/runbook surfaces were inspected. No nested agents, checkbox promotion,
commit/push or unrelated cleanup.

## Validation

The initial command used the ambient configuration path. remote_concurrency
passed 12 tests; remote_subtab passed eight and failed two when opening another
owner with configuration I/O: Operation not permitted. Cargo exited 101 and
did not execute the remaining targets. This failed attempt is not a pass.

Source inspection confirmed new workspace owner preparation resolves production
configuration/resources even with an explicit Echo fixture backend. The same
command was rerun with a newly created writable temporary ZENPI_HOME, without
modifying user config, changing production validation, skipping assertions or
disabling tests:

```text
env ZENPI_HOME=/private/tmp/zenpi-control-validation.vaBUfR cargo test --offline --locked --test remote_workspace --test remote_worktree --test remote_concurrency --test remote_terminal --test remote_worktree_runner --test remote_worker_owner --test remote_subtab
```

The isolated command exited zero, 130 passed, zero failed/ignored:

| Target | Passed | Scope |
| --- | ---: | --- |
| remote_concurrency | 12 | Exact quotas, FIFO, isolated cancellation and real local tool writers |
| remote_subtab | 10 | Stable identity/migration, checkpoint CAS and wakeup/publication |
| remote_terminal | 9 | Real PTY input/output/resize, exit, secret filtering, backpressure and shutdown |
| remote_worker_owner | 12 | Independent journals/leases, busy and failed-commit fencing, 64-owner bound |
| remote_workspace | 68 | Signed ACL/CAS control, receipts/restart, session/catalog, approval, worktree and worker routing |
| remote_worktree | 6 | Real Git dirty/staged/untracked/ignored/locked/path refusal and TUI preservation |
| remote_worktree_runner | 13 | Durable quota, 64 slots plus queued 65th, runtime claims, restart and selected-root execution |

These tests exercise real filesystem, Git and PTY boundaries plus controlled
owner/backend fixtures. Some owners are Echo, so this is not a provider HTTP,
production TCP listener, current Android or physical LAN receipt. Parity matrix
now identifies these target boundaries. Runbook documents isolated validation
and rebuilding source attribution after Cargo changes the host binary.

Stage2 Python 107, lifecycle verifier 21, physical verifier six,
instrumentation/Blueprint tests five each pass. Formatting, diff and strict
Blueprint pass; all 36 checklist rows remain unfinished. Current APK provenance
is missing. No unchanged blocked Gradle/ADB retry or stale APK install occurred.

Cargo test changed the debug binary and the old attribution was refused. The
owned build tool exited zero and regenerated the receipt (no manually edited
digest); final verify-only passes. Debug receipt sha256:
7020b95d66d5de27c6e83e403914087f6731954002d9fe0b2f56c19f5b81367a,
binary 57b52b80beda551642c47cd1f404f2fd1400b7a16699f91401402cb77db227a8,
inputs 2fa078489a749991bf55b24c8d412bdf554bf9ba0cf37e78cf83f44683aecb4f.
Release verify-only remained current. No Rust production source was edited.

## Source Identity

```text
remote_workspace.rs d48a00f880c7f8a8ac21cc5bd6622da6163d5a42d82394f5b8a520b7ae03810e
remote_worktree.rs fbe71a7354ae492e58a37965f5f8dd560ed0a6f6998b88d1bd9f42b151d6f5d5
remote_concurrency.rs 0128b9c37387bf30935c0d0de0668b8a3f1ca1cccaa1d22cda1194ebc9e13da1
remote_terminal.rs b0e4307aae41ee0bcc5b330738432ee0fecd050c41d3d17f12b7be2aa5cbc88b
remote_worktree_runner.rs 70027500a12713f2d024343873f8dda9df94154c28bc0a1da2a2b4df55f1d533
remote_worker_owner.rs dbee9f5e38e3a9ffde9cade553e65921d03988aaefdd606e22f34435d8000c17
remote_subtab.rs 9e368e86c3ce00415c0d9bc1b3ca051545beb88df5441be56517687ab238c94e
```

Goal remains active. Current APK/device, physical QR/direct LAN desktop sync,
complete parity, supported API lifecycle and signed release remain unaccepted.
