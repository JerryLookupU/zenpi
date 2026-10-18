# Read-Only Android File Browsing

Provisional S2-036/S2-054 progress, not Master acceptance. The immediately
preceding RTK check verified an installed tool but did not change goal
implementation. This continuation revalidated current skill, Blueprint,
specification, dirty worktree, host file listing and Android implementation,
then completed unfinished file policy and UI tests. Existing changes preserved;
no nested agents, reset, push or checkbox promotion.

## Defect and Fix

Host files_list requires Read authority, not prompt control. Android's sole
entry was inside an idle prompt composer. Read-only devices and controlling
devices running a task could not reach the directory metadata capability.

FileReadPolicy is now shared by Compose and ViewModel. Connected current
unique project/session/selected-subtab scope permits browsing independently of
prompt authority. Scanner/reset/loading/pending write/revoke states fence it.
Directory navigation and paging also require successful current metadata.
Failed reads may refresh. Attachment insertion separately requires prompt
control and no accepted/running prompt. Browsing does not send a task, grant
write authority or project host file bytes/absolute roots.

Read-only and running-task clients now have a Browse host files entry. Idle
controlling clients retain Attach host file. Three JVM tests cover read-only
and running-task separation, lifecycle/scope uniqueness, stale metadata,
failed-read refresh and unknown-write fences. The two existing named Compose
tests were extended for entry visibility, read-only directory/page navigation,
disabled attachment, refresh and running-task restrictions. Compose tests were
compiled only, not device-run. This change adds no new cross-language file
listing lane; Rust host and Kotlin policy evidence are distinct.

## Verification

Cwd /Users/mac/Github/zenpi; HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b; verification date 2026-10-04 Asia/Shanghai.

- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-readonly-files-20261004: exit 0; 494 ordinary JVM tests,
  production/unit/instrumented Kotlin compilation, current terminal QR decode
  and all explicit current Rust/Kotlin interop lanes pass.
- ZENPI_HOME=/private/tmp/stage2-readonly-files-home-20261004 rtk cargo test
  --locked --offline --test remote_workspace files_: 2 passed, 70 filtered.
  Tests exercise Read-device selected-root metadata, privacy exclusions,
  revoke, no journal/checkpoint writes, pagination and changed snapshots.
- Python Stage 2 discovery: 135 pass; lifecycle verifier tests: 25 pass.
- cargo fmt --check, git diff --check and strict Blueprint pass. All 36 rows
  remain [ ]; compiled UI tests do not satisfy connected Android gates.
- Debug host rebuilt after source-check Cargo lanes; verify-only passes.
- APK verify-only refuses missing build provenance. No permission bypass,
  stale APK installation or physical QR/direct-LAN acceptance claim.

```text
FileReadPolicy.kt da9e23cec6466ace634776358b55adf0689b072e22016cb36ae21f19167d03fc
PairingViewModel.kt 20bf18f8c9a7986bdfc5aee42d052bd4324fef630d2dcf48660115b2957176f2
MainActivity.kt b30b2a964064bb013f3f504a718a7c84fc9b931edee6c25ed2a8d3662a9745ed
FileReadPolicyTest.kt 99003479a0de59e13b36df87e0e9fcfc6d4c47703705fce49eef9c6265da2c8f
WorkspaceFilesScreenTest.kt 31a4848ae9f1caa4a0a1dcfb9a98e0b613073d8b222498571ae2b90d6fdd64e1
source-log 4ad1d044545bd00797f2dd57477db528dc7fd4be36e625fa7be3a43192bd29e8
main.jar ffa3b7dde0f1065f565f7cf2b0819464d3db86ee82b3bd37b4193f18ba3f7e29
unit.jar c77ff7161e79a7aa5283da6a9c635ebab4cd4aa0f16559b0bfa02bde1a6f78c8
instrumented.jar b26bf6b73ea948a77668310bce33a6f3ab3ced9d6a754c1f75b8e3d0cb730a63
debug-host-receipt c4973fcb85948dd24dd006092e521d5c8f25fba51d218758ad4688ef51f647aa
debug-host-inputs 404f2e9839daf95bc19114225349e4ccfa78c94b539941804f15e682bf8b5a83
debug-host-binary 51cf433e9ebfd59aeebdbf46f844c0603872e798f8e4b6443308aed8627eca67
```

Goal active and incomplete. Current APK, physical QR/direct-LAN bidirectional
sync/recovery, connected API lifecycle, complete desktop parity and signed
reproducible release remain required unaccepted gates.
