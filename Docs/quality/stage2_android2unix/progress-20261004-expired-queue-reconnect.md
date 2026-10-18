# Expired Invitation Must Not Suppress Saved-Host Resume

Recorded 2026-10-04. Partial S2-031/S2-038/S2-052/S2-053 evidence, not
Master acceptance. Previous goal turn fixed build-status typing and classified
the full Rust gate. Current skill, Blueprint/controller, dirty checkout and
foreground/cold/reconciliation paths inspected. No nested agents, permission
bypass, resets or checklist acceptance changes.

## Repair

The validated cold queue exposed a reconnect branch: after restoring a saved
host, an expired queued invitation was correctly refused, but its mere presence
prevented the otherwise valid automatic foreground refresh. This left the
previous desktop offline until a later lifecycle or explicit refresh action.

Cold restore and interrupted-pair storage reconciliation now decide refresh
from the resulting state after invitation admission. A valid replacement clears
host and remains confirming, so the old host does not reconnect. A refused
expired invitation retains saved host and can reconnect. Pending workspace/
revocation, foreground/network/reset/scanner gates and explicit recovery cancel
remain enforced. There is no token retry or new mutation dispatch.

The existing expired-queue instrumented regression now requires generation to
advance for saved-host reconnect, then stops foreground and checks offline,
retired invitation, generic expiry scan error and unchanged encrypted store.
It compiles but has not executed on a current device. The exact named scanner
test count remains 28 and its expiry phase remains mandatory.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-expired-queue-resume-20261004: exit 0; main/unit/device-test
  source compilation and every explicit Rust/Kotlin interop lane pass. Ordinary
  JVM suite independently rerun from those jars: 593 pass; not APK/device execution.
- Seven Rust targets: remote_concurrency 12, remote_events nine, remote_frame
  ten plus one ignored real-socket test, remote_migration six, remote_parity
  five, remote_protocol 18 and remote_security 13 pass (73 ordinary tests).
- Next command: remote_auth 12, remote_faults three, remote_headless one,
  remote_pairing eight and remote_subtab 11 pass. remote_tui_pairing fails:
  four pass, two fail. The delivery test reports listener EPERM; production
  child emitted no valid invite (InvalidInvite). Its stderr was not retained
  by this test, so that second failure is not asserted to be independently
  diagnosed as EPERM. Later targets in that invocation did not run.
- Separate tail command passes remote_worker_owner 12, remote_workspace 72,
  remote_worktree six and remote_worktree_runner 13. Total across 17 selected
  targets: 215 pass, two fail, one ignored. Each host command used a distinct
  mktemp ZENPI_HOME. These are local domain/frame/Git/PTY gates, not full Rust,
  physical LAN or connected Android acceptance.
- Stage 2 Python suite 169 pass; scanner verifier 20 and lifecycle fixtures 25
  pass. Strict Blueprint, cargo fmt --check and git diff --check pass.
- After Cargo lanes, debug attribution rebuilt and release verify-only passes.
  APK verify-only refuses missing provenance. Existing Gradle daemon permission
  diagnostic was not retried. No phone camera/direct-LAN run occurred.

All launched handles ended. The Blueprint remains 36 [ ], zero [_]/[x]. Current
signed APK/install/upgrade, connected API31/34/35, physical camera/LAN two-way
synchronization/recovery and unrestricted full host gates remain required.

```text
PairingViewModel.kt 91675c007a2391dc2ba3bc3d9161533d0a26e4d58a01d7552d295e04f0ccff94
ScannerResultTest.kt cfd86f1ebf94f8955b77cd07de7211136820a0d340483833fc3e109f0e14811f
main.jar b716c9d0b0a2bf328866c042ccac671d202f766c595e3037f7d60298b3d99098
unit.jar 53579dbfbcf54a05b16f56427cf769a0276f01dda11463a5ed8d1aa958b6d946
instrumented.jar 98b46e4b82497b841306210f800a5477e555622977e4098ed795a3007ed90c84
debug-receipt 302490f34f1f9968b257133047364a7052fd40432861a349191a998fc4d142e2
release-receipt eaea698082564e190c5de4577c0c80a9e52fe9783203462762a7e825ab4131f9
```
