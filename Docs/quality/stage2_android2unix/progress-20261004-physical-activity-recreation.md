# Physical QR Activity Recreation Acceptance Preparation

Recorded 2026-10-04. Partial S2-038/S2-051/S2-053/S2-055 preparation,
not Master acceptance. Previous goal turn repaired cold cancel recovery.
Current execution skill, Blueprint/controller, dirty checkout, production
Activity and physical harness inspected. No nested workers, resets, permission
bypass, device operation or checklist acceptance changes.

## Added Gate

PhysicalQrHostTest now recreates the real MainActivity after manual camera/SAS
pairing and first authenticated desktop transcript read. It requires a new
Activity with the same ViewModel, cleared intent data, automatic CONNECTED
recovery without another scan or Refresh click, unchanged paired host and
Keystore device identity, no invite/scanner ownership, same session/transcript
and nonregressing event cursor. It then continues the existing local desktop
update, background replay and Android prompt workflows, so successful test
completion includes synchronization after recreation, not a standalone marker.

The physical receipt contract is v6. The ordered
physical_qr_activity_recreation_resume_verified phase and
activity_recreation_resume_verified boolean are mandatory; failed/final-invalid
runs clear this gate along with all others. v5 receipts cannot prove the new
workflow. New verifier tests reject missing/duplicate/reordered phases and
false gate values, and explicitly reject the previous receipt schema. User
guide and lifecycle/network matrices record the new requirement. This is
Activity recreation, not Android process death or API-matrix acceptance.

## Verification

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-physical-recreation-20261004: exit 0. Main/unit/device-test
  source compiles; 593 ordinary JVM tests and every explicit current Rust/Kotlin
  QR/proof/workspace/worktree/session/ACK/approval/PTY/model/resource/file/diff/
  history/worker/prompt/cancel lane pass. Existing warnings remain.
- Physical verifier fixtures 11 pass; full Stage 2 Python tool suite 170 pass.
  Lifecycle validator fixtures 25 pass. Strict Blueprint, cargo fmt --check
  and git diff --check pass. All launched validation handles ended.
- Debug host attribution rebuilt after Cargo lanes; release verify-only passes
  with unchanged binary bytes. No new independent reproducibility claim.
- APK verify-only refuses missing provenance. No physical workflow ran, no
  camera/SAS/recreation/Keystore/LAN success inferred from compilation or
  synthetic Python receipt fixtures. Existing denied Gradle/ADB/listener work
  was not repeated. Blueprint remains 36 [ ], no [_]/[x].

Current signed packaging/install/upgrade, connected API31/34/35, physical QR
direct-LAN two-way synchronization/recovery, process death and unrestricted
full host gates remain required. Goal active/incomplete.

```text
PhysicalQrHostTest.kt 12746fda9b0345e6c32595be28060397b8a8e76fb10ee93e3a98fadfb562f088
physical-harness da85a3b32ab7a86835ad8b743edfe5825986533a34ed7622c72d69e090bcffdd
physical-harness-tests f4a5fbd8160ccea1825287c355546fd24db49c1f92cb0090941d53d2c571a99f
main.jar ebc11a7ffe7e70712f34ce0f6585c99950a9c7953de129b44b1c6e57b823ca43
unit.jar 53579dbfbcf54a05b16f56427cf769a0276f01dda11463a5ed8d1aa958b6d946
instrumented.jar 26e6ab378658d4ae9d3a677ffc431b32ea2a6dcb08e264b9651e9d75677d9acb
debug-receipt 9fc429a7993bcacbaae750c54b1a9902caeac594b3fb93495e628d823d5f2796
release-receipt eaea698082564e190c5de4577c0c80a9e52fe9783203462762a7e825ab4131f9
```
