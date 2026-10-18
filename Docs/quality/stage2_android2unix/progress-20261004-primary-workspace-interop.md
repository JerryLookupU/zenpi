# Android Primary Workspace and Actual Desktop View

Provisional S2-015/S2-034/S2-052/S2-054 progress. The preceding RTK turn
verified the installed CLI and Rust formatting, but did not advance product
implementation; this turn revalidated current host attribution, missing APK
attribution and the ADB socket refusal before taking the next safe source action.
Execution skill, Blueprint, specification, desktop workspace dispatcher,
Kotlin command/receipt/admission codecs and dirty integration were inspected.
No nested agents, authoritative checkbox promotion, reset, commit or push.

## Change

Added remote_android_workspace_interop and AndroidRustWorkspaceTest, wired as
an explicit current-source checker lane. Kotlin decodes real desktop listing
and selected-session snapshots, generates framed open/select/close commands,
and Rust dispatches them through actual ProjectRuntimeHost and TuiState.
The owner is prepared with isolated explicit OpenAI-compatible configuration;
no provider request or Echo backend is used. Fixture files remain local.

Eight stages open, select both directions, lose a CAS race to a local desktop
selection, close, reopen, select and close again. Assertions cover actual
desktop tab selection/count, independent unsent drafts, durable checkpoint,
closed agent phase, and preserved child files. Read-only grants, ungranted
open destinations, admitted-project busy close, stale revisions and conflicting
request IDs are refused without checkpoint/view changes. Exact retry returns
the original receipt without another mutation. A restarted actual desktop
returns all seven successful receipts without reopening closed tabs; new
request identities using the retired owner are refused.

Kotlin reads every success/refusal with three-byte fragmentation, validates
owner/revision/selection receipts, refuses altered receipt fields, replaces
its project projection and rejects new writes built from the stale projection.
Only an exact pending write remains retry-admissible after a desktop winner.
Foreground, network and camera fences are checked independently.

## Verification

Cwd /Users/mac/Github/zenpi, HEAD
1a8ff1acf72e0816fad503a05df2b1d18e07361b, existing dirty integration preserved.
Final observed UTC 2026-10-03T21:38:49Z (Oct 4 local time).
Blueprint digest d76e70e369f11f0fd3d1905d56d79c2a89401b92215da48ad85314f390d96d97;
spec digest 403e6339b0318a420d93600fc52926acf6a9e46ec899bd9a9edb03c235190a28.

- First complete source run: exit 101 at the new fixture's open assertion.
  Agent::new did not retain configuration needed to prepare another owner.
  Fixed fixture to use Agent::prepare_project_with_options and explicit
  isolated overrides, not relaxed production checks or an Echo replacement.
  A separate ad-hoc rerun first had an incorrectly joined Java classpath;
  corrected runner then passed. Both failures are non-acceptance evidence.
- zsh tools/stage2_android_source_check.sh
  /private/tmp/stage2-primary-workspace-20261004-final: exit 0 (pipefail/rg
  output filter). Current QR fixtures, main/unit/instrumented Kotlin compile,
  474 ordinary JVM tests, wire/rename/P256/worktree/session/ACK/approval/PTY
  and new primary-workspace lanes pass. New lane has 16 Kotlin invocations.
- env ZENPI_HOME=/private/tmp/stage2-primary-regression-20261004 rtk cargo test
  --offline --locked --test remote_workspace --quiet: exit 0, 72 pass.
- python3 -m unittest discover -s tools -p 'test_stage2*.py': 132 pass.
- python3 tools/test_validate_stage2_android_lifecycle.py: 23 pass.
- Rust fmt check, git diff --check, strict Blueprint pass; 36 rows remain [ ].
- Host provenance was correctly invalidated by Cargo test replacing the
  executable; rebuilt after all source lanes before separate verification.
- APK verify-only refuses missing provenance. adb devices -l fails to start
  its smart-socket listener with Operation not permitted. No permission
  bypass, stale APK install, device execution or physical QR/LAN claim.

```text
remote_android_workspace_interop.rs 50768d9b697bb2dd0cf9e023c06c2f3e6ced5ab4a401e0e09d1600b59f1caf7a
AndroidRustWorkspaceTest.kt 626852d7afb69b4dd00068382e74ce4e12a8656469b824f3ca296dab3be89411
stage2_android_source_check.sh e8c1e5c1128a5cffc1934dae0a57d8dbe3946f74a8463d2ae951cdf33bb62fba
workspace-results 81bfbfb9a7443214b14ee64df68ff26c4b3d3453794ee73b8d0fe84750c52600
main.jar d11429817490daef47383be790488fb2dad445c406e07ac91121277872d4f7c9
unit.jar 8f41468142c6dc64e1feb76c9ada37e3302a102d83d913c0a041be511dce97fc
instrumented.jar 8c22b80093a6d240fc71428f32893dddb9b7a3c7fe1e5b4efaff04ca507bba8d
debug-host-receipt 68d5107249b011a6d898488edeec96dbe8c083091a3bc3bceb129a6812343dcc
debug-host-binary 677c02131aa670e18cd3813bb1a8016026b375a3155f9c010b0c62f29d287df4
```

Goal active and incomplete. This validates current cross-language desktop
owner/view behavior, not Android UI, Keystore, TLS/TCP listener or physical
transport. Fresh APK, physical camera QR/direct-LAN bidirectional sync and
recovery, connected API31/34/35 lifecycle, complete parity, migration/reset/
uninstall and signed reproducible release remain unaccepted gates.
